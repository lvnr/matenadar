//! Matenadar in WebAssembly: one document laid out in memory, edited with
//! operations and rendered to SVG, PNG or PDF, in a browser or any other
//! WebAssembly host. There is no file system, so fonts and the document's
//! pictures are handed over as bytes. `js/matenadar.js` wraps it.
//!
//! The protocol, for hosts without the wrapper: get a buffer from
//! `mt_alloc`, copy bytes into it and pass it to a call, which takes it
//! over. Every call returns 0 on success or 1 on failure and leaves its
//! result (or the error) at `mt_result_ptr` / `mt_result_len` until the
//! next call.
//!
//! - `mt_font(font)`: adds a font file (OTF, TTF or a collection).
//! - `mt_file(path, data)`: adds or replaces a picture or drawing under the
//!   document's asset root (`images/plate.jpg`).
//! - `mt_call(request)`: `{"method": …, "params": {…}}`, much as in
//!   `matenadar serve`. Results are JSON, except `svg` (text) and `png`
//!   and `pdf` (bytes).
//!
//! Methods (`js/matenadar.js` names them in camelCase):
//!
//! - Loading: `assets` `{doc}` (the files to fetch), `open` `{doc, check}`,
//!   `close`, `fonts`.
//! - Pages: `pages`, `spreads`, `svg` `{page, bleed}`, `png` `{page, scale
//!   | dpi, bleed}`, `rgba` `{page, scale, bleed}` (8 bytes of width and
//!   height, little-endian, then the pixels), `pdf`, `fingerprint`,
//!   `fonts_used`.
//! - Finding: `hit` `{page, x, y}`, `rects` `{id}`, `node` `{id}`, `lines`
//!   `{id}`, `locate`, `position` `{id}`, `layout` `{pages}`, `document`.
//! - Editing, each one step to undo: `apply` `{ops}`, `set_words` `{id,
//!   content}`, `set_fields` `{id, fields, style}`, `break_before` `{id,
//!   to}`, `hide_on_page` `{id, hide}`, `nudge` `{id, lines}`; then `undo`,
//!   `redo`.
//! - Results: `diagnostics`, `check` `{safe, min_ppi}`, `patch`, `rebase`.

use std::alloc::Layout;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use matenadar_model::{Document, Op};
use matenadar_typst::{CheckOptions, Editor, Files, Fonts, Paths, Session, check, inspect};
use serde_json::{Value, json};
use typst::foundations::Bytes;

#[derive(Default)]
struct Engine {
    fonts: Vec<Bytes>,
    /// The font set, built when a document opens; a new font drops it.
    built: Option<Arc<Fonts>>,
    files: Files,
    editor: Option<Editor>,
    result: Vec<u8>,
}

thread_local! {
    static ENGINE: RefCell<Engine> = RefCell::new(Engine::default());
}

enum Out {
    Json(Value),
    Text(String),
    Bytes(Vec<u8>),
}

type Result<T> = std::result::Result<T, String>;

/// A buffer of `len` bytes for the host to fill and pass to a call.
#[unsafe(no_mangle)]
pub extern "C" fn mt_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::NonNull::dangling().as_ptr();
    }
    // SAFETY: the layout has a non-zero size.
    unsafe { std::alloc::alloc(Layout::array::<u8>(len).expect("buffer size")) }
}

#[unsafe(no_mangle)]
pub extern "C" fn mt_result_ptr() -> *const u8 {
    ENGINE.with_borrow(|e| e.result.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn mt_result_len() -> usize {
    ENGINE.with_borrow(|e| e.result.len())
}

/// Takes back a buffer from `mt_alloc`.
///
/// # Safety
/// `ptr` must come from `mt_alloc(len)` and not have been taken before.
unsafe fn take(ptr: *mut u8, len: usize) -> Vec<u8> {
    if len == 0 {
        return Vec::new();
    }
    // SAFETY: allocated by `mt_alloc` with exactly this size and alignment.
    unsafe { Vec::from_raw_parts(ptr, len, len) }
}

/// Runs `f` on the engine and keeps what it gives as the result.
fn run(f: impl FnOnce(&mut Engine) -> Result<Out>) -> u32 {
    ENGINE.with_borrow_mut(|e| {
        let (status, result) = match f(e) {
            Ok(Out::Json(v)) => (0, v.to_string().into_bytes()),
            Ok(Out::Text(t)) => (0, t.into_bytes()),
            Ok(Out::Bytes(b)) => (0, b),
            Err(msg) => (1, msg.into_bytes()),
        };
        e.result = result;
        status
    })
}

/// Adds a font file.
///
/// # Safety
/// The buffer must come from `mt_alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mt_font(ptr: *mut u8, len: usize) -> u32 {
    let data = unsafe { take(ptr, len) };
    run(|e| {
        let faces = typst::text::Font::iter(Bytes::new(data.clone())).count();
        if faces == 0 {
            return Err("not a font file".into());
        }
        e.fonts.push(Bytes::new(data));
        e.built = None;
        Ok(Out::Json(json!({"faces": faces})))
    })
}

/// Adds or replaces a file under the document's asset root. Open documents
/// see it at their next operation.
///
/// # Safety
/// Both buffers must come from `mt_alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mt_file(path: *mut u8, path_len: usize, data: *mut u8, data_len: usize) -> u32 {
    let (path, data) = unsafe { (take(path, path_len), take(data, data_len)) };
    run(|e| {
        let path = String::from_utf8(path).map_err(|_| "the path is not UTF-8")?;
        if path.split('/').any(|c| c == "..") {
            return Err(format!("file paths may not leave the asset root: {path}"));
        }
        e.files.give(&path, data);
        Ok(Out::Json(json!(true)))
    })
}

/// Calls a method: `{"method": …, "params": {…}}`.
///
/// # Safety
/// The buffer must come from `mt_alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mt_call(ptr: *mut u8, len: usize) -> u32 {
    let request = unsafe { take(ptr, len) };
    run(|e| {
        let req: Value = serde_json::from_slice(&request).map_err(|err| format!("the request is not JSON: {err}"))?;
        let method = req.get("method").and_then(Value::as_str).ok_or("the request has no method")?;
        e.call(method, req.get("params").unwrap_or(&Value::Null))
    })
}

impl Engine {
    fn call(&mut self, method: &str, p: &Value) -> Result<Out> {
        let str_of = |k: &str| p.get(k).and_then(Value::as_str);
        let num_of = |k: &str| p.get(k).and_then(Value::as_f64).ok_or_else(|| format!("{method} needs {k}"));
        let id_of = || str_of("id").ok_or_else(|| format!("{method} needs id"));
        let bleed = p.get("bleed").and_then(Value::as_bool).unwrap_or(false);
        match method {
            "assets" => {
                let doc = Document::from_json(&p.get("doc").ok_or("assets needs doc")?.to_string())?;
                return Ok(Out::Json(json!({"fonts": doc.fonts, "files": doc.assets()})));
            }
            "open" => {
                let doc = Document::from_json(&p.get("doc").ok_or("open needs doc")?.to_string())?;
                let problems = matenadar_model::validate(&doc);
                if !problems.is_empty() {
                    return Err(format!("the document has problems: {}", json!(problems)));
                }
                let opts = check_options(p.get("check").unwrap_or(&Value::Null))?;
                let fonts = self.built.get_or_insert_with(|| Arc::new(Fonts::from_data(self.fonts.iter().cloned()))).clone();
                let paths = Paths { root: PathBuf::new(), font_dirs: vec![], system_fonts: false };
                let s = Session::with_files(doc, paths, fonts, self.files.clone()).map_err(|e| e.to_string())?;
                let editor = Editor::new(s, opts);
                let out = json!({"pages": editor.session().page_count(), "problems": editor.session().problems(), "warnings": editor.session().warnings(), "diagnostics": editor.diagnostics()});
                self.editor = Some(editor);
                return Ok(Out::Json(out));
            }
            "fonts" => {
                let fonts = self.built.get_or_insert_with(|| Arc::new(Fonts::from_data(self.fonts.iter().cloned())));
                return Ok(Out::Json(json!(fonts.families())));
            }
            "close" => {
                self.editor = None;
                return Ok(Out::Json(json!(true)));
            }
            _ => {}
        }
        let e = self.editor.as_mut().ok_or("open a document first")?;
        let page = || num_of("page").map(|n| n as usize);
        let scale = || -> Result<f32> {
            match (p.get("scale").and_then(Value::as_f64), p.get("dpi").and_then(Value::as_f64)) {
                (Some(s), _) => Ok(s as f32),
                (None, Some(d)) => Ok((d / 72.0) as f32),
                _ => Ok(100.0 / 72.0),
            }
        };
        let edited = |r: std::result::Result<matenadar_typst::editor::Edited, matenadar_typst::EngineError>| r.map(|x| Out::Json(json!(x))).map_err(|e| e.to_string());
        Ok(match method {
            "pages" => Out::Json(json!(e.session().pages())),
            "spreads" => {
                let n = e.session().page_count();
                let mut spreads = vec![json!([1])];
                let mut p = 2;
                while p <= n {
                    spreads.push(if p < n { json!([p, p + 1]) } else { json!([p]) });
                    p += 2;
                }
                Out::Json(json!(spreads))
            }
            "svg" => Out::Text(e.session().svg(page()?, bleed).ok_or("no such page")?),
            "png" => {
                let dpi = scale()? * 72.0;
                Out::Bytes(e.session().png(page()?, dpi, bleed).ok_or("no such page")?)
            }
            "rgba" => {
                let (w, h, data) = e.session().rgba(page()?, scale()?, bleed).ok_or("no such page")?;
                let mut out = Vec::with_capacity(8 + data.len());
                out.extend_from_slice(&w.to_le_bytes());
                out.extend_from_slice(&h.to_le_bytes());
                out.extend_from_slice(&data);
                Out::Bytes(out)
            }
            "pdf" => Out::Bytes(e.session().pdf().map_err(|e| e.to_string())?),
            "fingerprint" => Out::Json(json!(e.session().fingerprints())),
            "fonts_used" => Out::Json(json!(e.session().fonts_used())),
            "hit" => Out::Json(json!(e.hit(page()?, num_of("x")?, num_of("y")?))),
            "rects" => Out::Json(json!(e.rects(id_of()?))),
            "node" => Out::Json(e.node(id_of()?).unwrap_or(Value::Null)),
            "lines" => Out::Json(json!(e.lines(id_of()?))),
            "locate" => Out::Json(match e.session().locate(page()?, num_of("x")?, num_of("y")?) {
                Some((node, offset)) => json!({"node": node, "offset": offset}),
                None => Value::Null,
            }),
            "position" => Out::Json(match e.session().position(id_of()?) {
                Some((page, x, y)) => json!({"page": page, "x": x, "y": y}),
                None => Value::Null,
            }),
            "layout" => {
                let wanted: Option<Vec<usize>> = p.get("pages").and_then(|v| serde_json::from_value(v.clone()).ok());
                let report: Vec<_> = inspect::report(e.session()).into_iter().filter(|r| wanted.as_ref().is_none_or(|w| w.contains(&r.page))).collect();
                Out::Json(json!(report))
            }
            "document" => Out::Json(json!(e.document())),
            "apply" => {
                let ops: Vec<Op> = serde_json::from_value(p.get("ops").cloned().unwrap_or(json!([]))).map_err(|e| e.to_string())?;
                edited(e.edit(ops))?
            }
            "set_words" => edited(e.set_words(id_of()?, p.get("content").cloned().ok_or("set_words needs content")?))?,
            "set_fields" => {
                let fields = p.get("fields").and_then(Value::as_object).ok_or("set_fields needs fields")?;
                edited(e.set_fields(id_of()?, fields, p.get("style").and_then(Value::as_bool).unwrap_or(false)))?
            }
            "break_before" => edited(e.break_before(id_of()?, str_of("to")))?,
            "hide_on_page" => {
                let hide: Vec<String> = serde_json::from_value(p.get("hide").cloned().unwrap_or(json!([]))).map_err(|e| e.to_string())?;
                edited(e.hide_on_page(id_of()?, &hide))?
            }
            "nudge" => {
                let lines = p.get("lines").and_then(Value::as_i64).ok_or("nudge needs lines")?;
                Out::Json(json!(e.nudge(id_of()?, lines as i32).map_err(|e| e.to_string())?))
            }
            "caret" => Out::Json(json!(e.caret(id_of()?, num_of("offset")? as usize, p.get("upstream").and_then(Value::as_bool).unwrap_or(false)))),
            "offset_at" => Out::Json(json!(e.offset_at(id_of()?, page()?, num_of("x")?, num_of("y")?))),
            "selection_rects" => Out::Json(json!(e.selection_rects(id_of()?, num_of("start")? as usize, num_of("end")? as usize))),
            "text_lines" => Out::Json(json!(e.text_lines(id_of()?))),
            "split_block" => edited(e.split_block(id_of()?, num_of("offset")? as usize, str_of("new_id").ok_or("split_block needs new_id")?))?,
            "join_block" => Out::Json(json!(e.join_block(id_of()?).map_err(|e| e.to_string())?)),
            "mark_text" => edited(e.mark_text(
                id_of()?,
                num_of("start")? as usize,
                num_of("end")? as usize,
                str_of("field").ok_or("mark_text needs field")?,
                p.get("value").cloned().unwrap_or(Value::Null),
            ))?,
            "source" => Out::Json(json!(e.session().source_export())),
            "undo" => edited(e.undo())?,
            "redo" => edited(e.redo())?,
            "diagnostics" => Out::Json(json!(e.diagnostics())),
            "check" => Out::Json(json!(check(e.session(), &check_options(p)?))),
            "patch" => Out::Json(json!(e.patch())),
            "rebase" => {
                e.rebase();
                Out::Json(json!(true))
            }
            other => return Err(format!("unknown method \"{other}\"")),
        })
    }
}

/// Check settings: `{"safe": "0.25in", "gutter": "0.5in", "min_ppi": 300}`.
fn check_options(p: &Value) -> Result<CheckOptions> {
    let mut o = CheckOptions::default();
    let len = |k: &str| -> Result<Option<f64>> {
        match p.get(k) {
            Some(Value::Number(n)) => Ok(n.as_f64()),
            Some(Value::String(s)) => Ok(Some(matenadar_model::Length::parse(s)?.resolve(11.0, 0.0))),
            _ => Ok(None),
        }
    };
    if let Some(v) = len("safe")? {
        o.safe_pt = v;
    }
    o.gutter_pt = len("gutter")?;
    if let Some(ppi) = p.get("min_ppi").and_then(Value::as_f64) {
        o.min_ppi = ppi;
    }
    Ok(o)
}
