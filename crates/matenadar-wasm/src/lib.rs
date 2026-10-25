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
//! Methods:
//! - `open`: `{"doc": {…}}`. Lays the document out.
//! - `apply`: `{"ops": [op…]}`. Returns the changes, with inverses and changed pages.
//! - `svg`: `{"page": 3, "bleed": false}`
//! - `png`: `{"page": 3, "dpi": 100, "bleed": false}`
//! - `pdf`
//! - `check`: `{"safe": "0.25in", "min_ppi": 300}`
//! - `layout`: `{"pages": [1, 2]}`
//! - `locate`: `{"page", "x", "y"}`
//! - `position`: `{"id"}`
//! - `document`: the current document
//! - `fonts`: the font families given
//! - `close`

use std::alloc::Layout;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use matenadar_model::{Document, Op};
use matenadar_typst::{CheckOptions, Files, Fonts, Paths, Session, check, inspect};
use serde_json::{Value, json};
use typst::foundations::Bytes;

#[derive(Default)]
struct Engine {
    fonts: Vec<Bytes>,
    /// The font set, built when a document opens; a new font drops it.
    built: Option<Arc<Fonts>>,
    files: Files,
    session: Option<Session>,
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
        let bleed = p.get("bleed").and_then(Value::as_bool).unwrap_or(false);
        match method {
            "open" => {
                let doc = p.get("doc").ok_or("open needs doc")?;
                let doc = Document::from_json(&doc.to_string())?;
                let problems = matenadar_model::validate(&doc);
                if !problems.is_empty() {
                    return Err(format!("the document has problems: {}", json!(problems)));
                }
                let fonts = self.built.get_or_insert_with(|| Arc::new(Fonts::from_data(self.fonts.iter().cloned()))).clone();
                let paths = Paths { root: PathBuf::new(), font_dirs: vec![], system_fonts: false };
                let s = Session::with_files(doc, paths, fonts, self.files.clone()).map_err(|e| e.to_string())?;
                let out = json!({"pages": s.page_count(), "problems": s.problems(), "warnings": s.warnings()});
                self.session = Some(s);
                return Ok(Out::Json(out));
            }
            "fonts" => {
                let fonts = self.built.get_or_insert_with(|| Arc::new(Fonts::from_data(self.fonts.iter().cloned())));
                return Ok(Out::Json(json!(fonts.families())));
            }
            "close" => {
                self.session = None;
                return Ok(Out::Json(json!(true)));
            }
            _ => {}
        }
        let s = self.session.as_mut().ok_or("open a document first")?;
        let page = || num_of("page").map(|n| n as usize);
        Ok(match method {
            "apply" => {
                let ops: Vec<Op> = serde_json::from_value(p.get("ops").cloned().unwrap_or(json!([]))).map_err(|e| e.to_string())?;
                let mut changes = Vec::new();
                for op in &ops {
                    changes.push(s.apply(op).map_err(|e| e.to_string())?);
                }
                Out::Json(json!(changes))
            }
            "svg" => Out::Text(s.svg(page()?, bleed).ok_or("no such page")?),
            "png" => {
                let dpi = p.get("dpi").and_then(Value::as_f64).unwrap_or(100.0) as f32;
                Out::Bytes(s.png(page()?, dpi, bleed).ok_or("no such page")?)
            }
            "pdf" => Out::Bytes(s.pdf().map_err(|e| e.to_string())?),
            "check" => {
                let mut o = CheckOptions::default();
                if let Some(safe) = str_of("safe") {
                    o.safe_pt = matenadar_model::Length::parse(safe)?.resolve(11.0, 0.0);
                }
                if let Some(ppi) = p.get("min_ppi").and_then(Value::as_f64) {
                    o.min_ppi = ppi;
                }
                Out::Json(json!(check(s, &o)))
            }
            "layout" => {
                let wanted: Option<Vec<usize>> = p.get("pages").and_then(|v| serde_json::from_value(v.clone()).ok());
                let report: Vec<_> = inspect::report(s).into_iter().filter(|r| wanted.as_ref().is_none_or(|w| w.contains(&r.page))).collect();
                Out::Json(json!(report))
            }
            "locate" => Out::Json(match s.locate(page()?, num_of("x")?, num_of("y")?) {
                Some((node, offset)) => json!({"node": node, "offset": offset}),
                None => Value::Null,
            }),
            "position" => Out::Json(match s.position(str_of("id").ok_or("position needs id")?) {
                Some((page, x, y)) => json!({"page": page, "x": x, "y": y}),
                None => Value::Null,
            }),
            "document" => Out::Json(json!(s.document())),
            other => return Err(format!("unknown method \"{other}\"")),
        })
    }
}
