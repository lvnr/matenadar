//! Editing laid-out pages the way a person does in a page view: find what
//! is under the pointer, change it, undo and redo, see what the change
//! broke, and keep the changes as operations that a later build replays
//! exactly.
//!
//! Every change is a list of operations applied together and laid out
//! once: one step to undo. The patch is the steps' operations, addressed by
//! node id, split into the words (which belong to the manuscript) and the
//! layout (which belongs to the design), with repeated settings of one
//! field folded into the last.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use matenadar_model::runs;
use matenadar_model::walk::{Node, walk};
use matenadar_model::{Block, Document, Element, Item, Length, Op};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::caret::{Caret, TextLine};
use crate::check::{CheckOptions, Issue, check_pages, document_issues};
use crate::inspect::{Kind, Rect, items_on};
use crate::session::{EngineError, Session};

/// Blocks whose words a manuscript holds, and their fields that are words.
const WORD_BLOCKS: [(&str, &[&str]); 6] = [
    ("p", &["/text", "/content"]),
    ("h", &["/text", "/content"]),
    ("break", &["/text"]),
    ("figure", &["/src", "/caption", "/caption_content"]),
    ("list", &["/items"]),
    ("code", &["/text"]),
];

/// Paragraph settings that belong to where a paragraph starts, which the
/// second half of a split one does not take.
const STARTS: [&str; 5] = ["space_before", "drop_cap", "rule_above", "page_break_before", "indent_first"];

/// The tracking steps `nudge` tries, in ems, and how far it goes.
const NUDGE_STEP: f64 = 0.0025;
const NUDGE_STEPS: usize = 8;

/// Which part of the book an operation changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Part {
    /// The words: the manuscript's paragraphs, headings, figures and lists.
    Words,
    /// Everything else: styles, placement, breaks, overrides.
    Layout,
}

#[derive(Clone, Debug)]
struct Step {
    ops: Vec<Op>,
    inverses: Vec<Op>,
    parts: Vec<Part>,
}

/// The result of an edit, an undo or a redo.
#[derive(Debug, Serialize)]
pub struct Edited {
    pub changed_pages: Vec<usize>,
    pub page_count: usize,
    pub layout_ms: f64,
    pub diagnostics: Vec<Issue>,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct NodeRef {
    pub id: String,
    pub kind: &'static str,
}

/// What is under a point.
#[derive(Debug, Serialize)]
pub struct Hit {
    /// The innermost node: a paragraph, a figure, an element.
    pub id: String,
    pub kind: &'static str,
    pub page: usize,
    /// The node's extent on this page.
    pub rect: Rect,
    /// Where in the node's plain text, in bytes, when the point is on words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// The innermost placed element (a text frame, an image) around it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<NodeRef>,
    /// Every node it is in, innermost first.
    pub ancestors: Vec<NodeRef>,
}

/// A join: the block that took the words, where they start in it, and
/// the edit.
#[derive(Debug, Serialize)]
pub struct Joined {
    pub into: String,
    pub offset: usize,
    #[serde(flatten)]
    pub edited: Edited,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct PageRect {
    pub page: usize,
    pub rect: Rect,
}

/// The changes since the document was opened (or rebased).
#[derive(Debug, Default, Serialize)]
pub struct Patch {
    /// Changes to the words, in order. The nodes they touched are listed in
    /// `touched`; their words now are what `node` gives.
    pub words: Vec<Op>,
    pub touched: Vec<String>,
    /// Changes to the layout, in order, each field's settings folded into
    /// the last.
    pub layout: Vec<Op>,
}

/// What `nudge` did.
#[derive(Debug, Serialize)]
pub struct Nudged {
    pub lines_before: usize,
    pub lines_after: usize,
    /// The paragraph's tracking now, in ems.
    pub tracking: f64,
    #[serde(flatten)]
    pub edited: Edited,
}

pub struct Editor {
    session: Session,
    base: Value,
    undo: Vec<Step>,
    redo: Vec<Step>,
    opts: CheckOptions,
    issues: BTreeMap<usize, Vec<Issue>>,
    doc_issues: Vec<Issue>,
}

fn engine(msg: impl Into<String>) -> EngineError {
    EngineError(msg.into())
}

impl Editor {
    pub fn new(session: Session, opts: CheckOptions) -> Self {
        let base = serde_json::to_value(session.document()).unwrap_or_default();
        let mut e = Self { session, base, undo: Vec::new(), redo: Vec::new(), opts, issues: BTreeMap::new(), doc_issues: Vec::new() };
        let all: BTreeSet<usize> = (1..=e.session.page_count()).collect();
        e.recheck(&all);
        e
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn document(&self) -> &Document {
        self.session.document()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Every issue, the document's first, then page by page.
    pub fn diagnostics(&self) -> Vec<Issue> {
        let mut out = self.doc_issues.clone();
        out.extend(self.issues.values().flatten().cloned());
        out
    }

    /// Applies operations as one step.
    pub fn edit(&mut self, ops: Vec<Op>) -> Result<Edited, EngineError> {
        if ops.is_empty() {
            return Err(engine("no operations"));
        }
        let parts = ops.iter().map(|op| self.part_of(op)).collect();
        let changes = self.session.apply_all(&ops)?;
        self.undo.push(Step { ops, inverses: changes.inverses, parts });
        self.redo.clear();
        Ok(self.after(changes.changed_pages, changes.compile))
    }

    pub fn undo(&mut self) -> Result<Edited, EngineError> {
        let step = self.undo.pop().ok_or_else(|| engine("nothing to undo"))?;
        match self.session.apply_all(&step.inverses) {
            Ok(changes) => {
                self.redo.push(step);
                Ok(self.after(changes.changed_pages, changes.compile))
            }
            Err(e) => {
                self.undo.push(step);
                Err(e)
            }
        }
    }

    pub fn redo(&mut self) -> Result<Edited, EngineError> {
        let mut step = self.redo.pop().ok_or_else(|| engine("nothing to redo"))?;
        match self.session.apply_all(&step.ops) {
            Ok(changes) => {
                step.inverses = changes.inverses;
                self.undo.push(step);
                Ok(self.after(changes.changed_pages, changes.compile))
            }
            Err(e) => {
                self.redo.push(step);
                Err(e)
            }
        }
    }

    /// Makes the document as it stands the base of the patch, and forgets
    /// the steps (after the patch has been saved).
    pub fn rebase(&mut self) {
        self.base = serde_json::to_value(self.session.document()).unwrap_or_default();
        self.undo.clear();
        self.redo.clear();
    }

    fn after(&mut self, changed: Vec<usize>, compile: std::time::Duration) -> Edited {
        let count = self.session.page_count();
        let around: BTreeSet<usize> = changed.iter().flat_map(|&p| [p.saturating_sub(1), p, p + 1]).filter(|&p| p >= 1 && p <= count).collect();
        self.issues.retain(|&p, _| p <= count);
        self.recheck(&around);
        Edited {
            changed_pages: changed,
            page_count: count,
            layout_ms: compile.as_secs_f64() * 1000.0,
            diagnostics: self.diagnostics(),
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
        }
    }

    fn recheck(&mut self, pages: &BTreeSet<usize>) {
        for p in pages {
            self.issues.remove(p);
        }
        for issue in check_pages(&self.session, &self.opts, pages) {
            self.issues.entry(issue.page.unwrap_or(0)).or_default().push(issue);
        }
        self.doc_issues = document_issues(&self.session);
    }

    // ---- Finding things ----

    /// The innermost node under a point (points from the trim's top-left).
    /// Only nodes the document has count, not those the engine makes (a
    /// text frame's paragraphs).
    pub fn hit(&self, page: usize, x: f64, y: f64) -> Option<Hit> {
        let ids = self.ids();
        let items = items_on(&self.session, [page]);
        let real = |i: &crate::inspect::Item| -> Vec<NodeRef> {
            i.ancestors.iter().filter(|a| ids.contains(&a.0)).map(|(id, kind)| NodeRef { id: id.clone(), kind }).collect()
        };
        let (best, ancestors) = items
            .iter()
            .filter(|i| i.rect.contains(x, y, 1.0))
            .map(|i| (i, real(i)))
            .filter(|(_, a)| !a.is_empty())
            .min_by(|a, b| a.0.rect.area().total_cmp(&b.0.rect.area()))?;
        let NodeRef { id, kind } = ancestors[0].clone();
        let rect = items.iter().filter(|i| i.ancestors.iter().any(|a| a.0 == id)).map(|i| i.rect).reduce(Rect::join).unwrap_or(best.rect);
        let offset = match best.kind {
            Kind::Text { .. } => self.session.locate(page, x, y).filter(|(n, _)| *n == id).map(|(_, o)| o),
            _ => None,
        };
        Some(Hit {
            id,
            kind,
            page,
            rect,
            offset,
            element: best.element.clone().filter(|e| ids.contains(&e.0)).map(|(id, kind)| NodeRef { id, kind }),
            ancestors,
        })
    }

    /// Every id the document gives a node.
    fn ids(&self) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        walk(self.session.document(), &mut |_, n| {
            let id = match n {
                Node::Block(b) => b.id(),
                Node::Element(e) => e.id(),
                Node::Item(i) => Some(i.id()),
                Node::Inline(_) => None,
            };
            if let Some(id) = id {
                out.insert(id.to_string());
            }
        });
        out
    }

    /// Where a node is: its extent on each page it reaches.
    pub fn rects(&self, id: &str) -> Vec<PageRect> {
        let count = self.session.page_count();
        let start = self.session.position(id).map(|p| p.0).unwrap_or(1);
        let mut out = Vec::new();
        for page in start..=count {
            let rect = items_on(&self.session, [page]).iter().filter(|i| i.ancestors.iter().any(|a| a.0 == id)).map(|i| i.rect).reduce(Rect::join);
            match rect {
                Some(rect) => out.push(PageRect { page, rect }),
                None if !out.is_empty() => break,
                None => {}
            }
        }
        out
    }

    /// A node as JSON.
    pub fn node(&self, id: &str) -> Option<Value> {
        let root = serde_json::to_value(self.session.document()).ok()?;
        let at = matenadar_model::ops::find_id(&root, id)?;
        root.pointer(&at).cloned()
    }

    /// How many lines a paragraph sets in.
    pub fn lines(&self, id: &str) -> usize {
        crate::caret::text_lines(&self.session, id).len()
    }

    /// The caret before the byte `offset` of a node's words; `upstream`
    /// keeps it at the end of a line that breaks there.
    pub fn caret(&self, id: &str, offset: usize, upstream: bool) -> Option<Caret> {
        crate::caret::caret(&self.session, id, offset, upstream)
    }

    /// The place in a node's words nearest a point on a page.
    pub fn offset_at(&self, id: &str, page: usize, x: f64, y: f64) -> Option<usize> {
        crate::caret::offset_at(&self.session, id, page, x, y)
    }

    /// A rectangle per line over the bytes `start..end` of a node's words.
    pub fn selection_rects(&self, id: &str, start: usize, end: usize) -> Vec<PageRect> {
        crate::caret::selection_rects(&self.session, id, start, end)
    }

    /// The lines a node's words set in, with the bytes each sets.
    pub fn text_lines(&self, id: &str) -> Vec<TextLine> {
        crate::caret::text_lines(&self.session, id)
    }

    // ---- Words ----

    /// Splits a paragraph or heading at a byte offset of its words, taking
    /// out the words `start..end` first (Enter over a selection; equal for a
    /// caret): it keeps the words before, and a new block after it, `new_id`,
    /// takes the rest, with the same type, style and overrides (but not those
    /// of a paragraph's start, such as space before or a drop cap). A heading
    /// split at its end is followed by a paragraph. One step.
    pub fn split_block(&mut self, id: &str, start: usize, end: usize, new_id: &str) -> Result<Edited, EngineError> {
        let node = self.words_block(id)?;
        let (start, end) = (start.min(end), start.max(end));
        let (before, rest) = runs::split_at(&runs::runs_of(&node), start).map_err(engine)?;
        let (_, after) = runs::split_at(&rest, end - start).map_err(engine)?;
        let mut block = if node["type"] == "h" && after.is_empty() {
            json!({"type": "p"})
        } else {
            let mut b = node.clone();
            if let Some(m) = b.as_object_mut() {
                for key in ["id", "text", "content", "short"] {
                    m.shift_remove(key);
                }
                if let Some(Value::Object(set)) = m.get_mut("set") {
                    for key in STARTS {
                        set.shift_remove(key);
                    }
                    if set.is_empty() {
                        m.shift_remove("set");
                    }
                }
            }
            b
        };
        block["id"] = json!(new_id);
        match runs::as_text(&after) {
            Some(t) if t.is_empty() => {}
            Some(t) => block["text"] = json!(t),
            None => block["content"] = json!(after),
        }
        let mut ops = words_ops(id, &node, before)?;
        ops.push(op(json!({"op": "insert", "after": id, "value": block}))?);
        self.edit(ops)
    }

    /// Joins a paragraph or heading to the paragraph or heading before it in
    /// the same list, which takes its words; it goes. Says which block took
    /// them, and where they start in it.
    pub fn join_block(&mut self, id: &str) -> Result<Joined, EngineError> {
        let node = self.words_block(id)?;
        let root = serde_json::to_value(self.session.document()).map_err(|e| engine(e.to_string()))?;
        let at = matenadar_model::ops::find_id(&root, id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        let (list, index) = at.rsplit_once('/').unwrap_or_default();
        let prev = index.parse::<usize>().ok().and_then(|i| i.checked_sub(1)).and_then(|i| root.pointer(&format!("{list}/{i}")));
        let Some(prev) = prev.filter(|p| matches!(p.get("type").and_then(Value::as_str), Some("p" | "h"))) else {
            return Err(engine(format!("there is no paragraph or heading before {id} to join it to")));
        };
        let into = prev.get("id").and_then(Value::as_str).ok_or_else(|| engine("the block before has no id"))?.to_string();
        let words = runs::runs_of(prev);
        let offset = runs::plain(&words).len();
        let mut ops = words_ops(&into, prev, runs::join(words, runs::runs_of(&node)))?;
        ops.push(op(json!({"op": "remove", "id": id}))?);
        let edited = self.edit(ops)?;
        Ok(Joined { into, offset, edited })
    }

    /// Marks the words `start..end` of a paragraph, heading or text frame:
    /// `em`, `strong`, `span` (a character style), `color`, `size`,
    /// `tracking`, `case`… A null value takes the mark off.
    pub fn mark_text(&mut self, id: &str, start: usize, end: usize, field: &str, value: Value) -> Result<Edited, EngineError> {
        let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        if !matches!(node.get("type").and_then(Value::as_str), Some("p" | "h" | "text")) {
            return Err(engine(format!("{id} has no words to mark")));
        }
        let marked = runs::set_mark(&runs::runs_of(&node), start, end, field, &value).map_err(engine)?;
        let ops = words_ops(id, &node, marked)?;
        if ops.is_empty() {
            return Err(engine("nothing to change"));
        }
        self.edit(ops)
    }

    /// A paragraph or heading, as JSON.
    fn words_block(&self, id: &str) -> Result<Value, EngineError> {
        let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        match node.get("type").and_then(Value::as_str) {
            Some("p" | "h") => Ok(node),
            kind => Err(engine(format!("{id} is a {}, not a paragraph or heading", kind.unwrap_or("node")))),
        }
    }

    // ---- Gestures ----

    /// Sets fields of a node: a picture's `src`, `crop`, `fit`, `focus`, an
    /// element's `x`, `y`, `w`, `h`… A null value removes the field.
    /// Paragraphs and headings take paragraph settings (`space_before`,
    /// `align`, `keep_with_next`…) as overrides of their style.
    pub fn set_fields(&mut self, id: &str, fields: &Map<String, Value>, as_style: bool) -> Result<Edited, EngineError> {
        let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        let styled = as_style && matches!(node.get("type").and_then(Value::as_str), Some("p" | "h"));
        let mut ops = Vec::new();
        for (key, value) in fields {
            let path = if styled { format!("/set/{key}") } else { format!("/{key}") };
            let present = node.pointer(&path).is_some();
            ops.push(match value {
                Value::Null if !present => continue,
                Value::Null => op(json!({"op": "unset", "id": id, "path": path}))?,
                v => op(json!({"op": "set", "id": id, "path": path, "value": v}))?,
            });
        }
        if ops.is_empty() {
            return Err(engine("nothing to change"));
        }
        self.edit(ops)
    }

    /// Replaces a paragraph's or heading's words with runs.
    pub fn set_words(&mut self, id: &str, content: Value) -> Result<Edited, EngineError> {
        let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        let mut ops = Vec::new();
        if node.get("text").is_some() {
            ops.push(op(json!({"op": "unset", "id": id, "path": "/text"}))?);
        }
        ops.push(op(json!({"op": "set", "id": id, "path": "/content", "value": content}))?);
        self.edit(ops)
    }

    /// Starts the node on a new page (`page`, or the next `recto` or
    /// `verso`), or with `None` takes back such a break.
    pub fn break_before(&mut self, id: &str, to: Option<&str>) -> Result<Edited, EngineError> {
        let mark = format!("{id}--break");
        let has_mark = self.node(&mark).is_some();
        let ops = match (to, has_mark) {
            (Some(to), true) => match to {
                "page" => vec![op(json!({"op": "unset", "id": mark, "path": "/to"}))?],
                side => vec![op(json!({"op": "set", "id": mark, "path": "/to", "value": side}))?],
            },
            (Some(to), false) => {
                let mut block = json!({"type": "page-break", "id": mark, "weak": true});
                if to != "page" {
                    block["to"] = json!(to);
                }
                vec![op(json!({"op": "insert", "before": id, "value": block}))?]
            }
            (None, true) => vec![op(json!({"op": "remove", "id": mark}))?],
            (None, false) => {
                // A break the style asks for: override it on this paragraph.
                let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
                if !matches!(node.get("type").and_then(Value::as_str), Some("p" | "h")) {
                    return Err(engine(format!("no break before {id} to take back")));
                }
                vec![op(json!({"op": "set", "id": id, "path": "/set/page_break_before", "value": false}))?]
            }
        };
        self.edit(ops)
    }

    /// Leaves master elements off the page where the node lands: by id, by
    /// a text element's style (`folio`, `running`), or `all`. An empty list
    /// puts them back.
    pub fn hide_on_page(&mut self, id: &str, hide: &[String]) -> Result<Edited, EngineError> {
        let present = self.session.document().on_page.contains_key(id);
        let path = format!("/on_page/{}", id.replace('~', "~0").replace('/', "~1"));
        let ops = if hide.is_empty() {
            if !present {
                return Err(engine(format!("nothing is hidden on {id}'s page")));
            }
            vec![op(json!({"op": "unset", "at": "", "path": path}))?]
        } else {
            vec![op(json!({"op": "set", "at": "", "path": path, "value": {"hide": hide}}))?]
        };
        self.edit(ops)
    }

    /// Sets a paragraph one line shorter (`lines` -1) or longer (+1) by the
    /// smallest change of its tracking that does it (at most 0.02 em).
    pub fn nudge(&mut self, id: &str, lines: i32) -> Result<Nudged, EngineError> {
        if lines == 0 {
            return Err(engine("nudge by -1 or +1 lines"));
        }
        let node = self.node(id).ok_or_else(|| engine(format!("no node with id \"{id}\"")))?;
        if node.get("type").and_then(Value::as_str) != Some("p") {
            return Err(engine("only a paragraph can be nudged"));
        }
        let before = self.lines(id);
        let target = before as i64 + lines as i64;
        if before == 0 || target < 1 {
            return Err(engine(format!("{id} sets in {before} lines")));
        }
        let start = self.tracking_of(id);
        let dir = lines.signum() as f64;
        let mut first_inverse: Option<Vec<Op>> = None;
        let mut changed: BTreeSet<usize> = BTreeSet::new();
        let mut found: Option<(f64, Op, std::time::Duration)> = None;
        for k in 1..=NUDGE_STEPS {
            let t = start + dir * NUDGE_STEP * k as f64;
            let value = format!("{}em", matenadar_model::units::num((t * 10000.0).round() / 10000.0));
            let set = op(json!({"op": "set", "id": id, "path": "/set/tracking", "value": value}))?;
            let changes = self.session.apply_all(std::slice::from_ref(&set))?;
            first_inverse.get_or_insert(changes.inverses);
            changed.extend(changes.changed_pages);
            let now = self.lines(id) as i64;
            if now == target {
                found = Some((t, set, changes.compile));
                break;
            }
            if (now - target).signum() == lines.signum() as i64 {
                break;
            }
        }
        let inverses = first_inverse.unwrap_or_default();
        match found {
            Some((t, set, compile)) => {
                let parts = vec![Part::Layout];
                self.undo.push(Step { ops: vec![set], inverses, parts });
                self.redo.clear();
                let edited = self.after(changed.into_iter().collect(), compile);
                Ok(Nudged { lines_before: before, lines_after: target as usize, tracking: t, edited })
            }
            None => {
                if !inverses.is_empty() {
                    self.session.apply_all(&inverses)?;
                }
                Err(engine(format!(
                    "{id} can't be set in {target} lines by tracking within {} em",
                    NUDGE_STEP * NUDGE_STEPS as f64
                )))
            }
        }
    }

    /// A paragraph's tracking in ems, as its style and overrides set it.
    fn tracking_of(&self, id: &str) -> f64 {
        let doc = self.session.document();
        let mut found: Option<(Option<String>, Option<matenadar_model::Style>, Option<String>)> = None;
        for item in &doc.content {
            if let Item::Section(s) = item {
                let flow_style = s
                    .style
                    .clone()
                    .or_else(|| doc.design.master(&s.master).and_then(|m| m.style))
                    .or_else(|| doc.design.styles.contains_key("body").then(|| "body".to_string()));
                matenadar_model::walk::blocks("", &s.flow, &mut |_, n| {
                    if let Node::Block(Block::P(p)) = n
                        && p.id.as_deref() == Some(id)
                    {
                        found = Some((p.style.clone(), p.set.clone(), flow_style.clone()));
                    }
                });
            }
        }
        let Some((style, set, flow)) = found else { return 0.0 };
        let name = style.or(flow).unwrap_or_default();
        let mut st = doc.design.style(&name);
        if let Some(over) = &set {
            st = st.with(over);
        }
        match st.text.tracking {
            Some(Length::Em(v)) => v,
            Some(Length::Pt(v)) => v / st.text.size.and_then(|s| s.pt()).unwrap_or(11.0),
            _ => 0.0,
        }
    }

    // ---- The patch ----

    fn part_of(&self, op: &Op) -> Part {
        let kind_of = |id: &str| self.kind_of(id);
        part_of(op, &kind_of)
    }

    fn kind_of(&self, id: &str) -> Option<&'static str> {
        let mut out = None;
        walk(self.session.document(), &mut |_, n| {
            if out.is_some() {
                return;
            }
            match n {
                Node::Block(b) if b.id() == Some(id) => out = Some(b.kind()),
                Node::Element(e) if e.id() == Some(id) => out = Some(element_kind(e)),
                Node::Item(i) if i.id() == id => out = Some(i.kind()),
                _ => {}
            }
        });
        out
    }

    /// The steps' operations since the base, words and layout apart.
    pub fn patch(&self) -> Patch {
        let all: Vec<(&Op, Part)> = self.undo.iter().flat_map(|s| s.ops.iter().zip(s.parts.iter().copied())).collect();
        let mut out = Patch::default();
        let mut touched = BTreeSet::new();
        // The last setting of each field.
        let mut last: HashMap<(String, String), usize> = HashMap::new();
        for (i, (op, part)) in all.iter().enumerate() {
            if *part == Part::Layout
                && let Some(key) = field_key(op)
            {
                last.insert(key, i);
            }
        }
        for (i, (op, part)) in all.iter().enumerate() {
            match part {
                Part::Words => {
                    if let Some(id) = touched_id(op) {
                        touched.insert(id);
                    }
                    out.words.push((*op).clone());
                }
                Part::Layout => {
                    if let Some(key) = field_key(op)
                        && (last.get(&key) != Some(&i) || self.as_base(op))
                    {
                        continue;
                    }
                    out.layout.push((*op).clone());
                }
            }
        }
        out.touched = touched.into_iter().collect();
        out
    }

    /// Whether a setting leaves its field as the base had it.
    fn as_base(&self, op: &Op) -> bool {
        let (target, path, value) = match op {
            Op::Set { node, path, value } => (node, path, Some(value)),
            Op::Unset { node, path } => (node, path, None),
            _ => return false,
        };
        let base_ptr = match (&target.id, &target.at) {
            (Some(id), _) => match matenadar_model::ops::find_id(&self.base, id) {
                Some(p) => p,
                None => return false,
            },
            (None, Some(at)) => at.clone(),
            _ => return false,
        };
        match (self.base.pointer(&format!("{base_ptr}{path}")), value) {
            (None, None) => true,
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        }
    }
}

/// Equal JSON, with 59 and 59.0 the same number.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        _ => a == b,
    }
}

/// The operations that give a node these words: plain `text` when they
/// have no marks, else `content`.
fn words_ops(id: &str, node: &Value, words: Vec<Value>) -> Result<Vec<Op>, EngineError> {
    let mut ops = Vec::new();
    match runs::as_text(&words) {
        Some(text) => {
            if node.get("content").is_some() {
                ops.push(op(json!({"op": "unset", "id": id, "path": "/content"}))?);
            }
            match node.get("text") {
                Some(_) if text.is_empty() => ops.push(op(json!({"op": "unset", "id": id, "path": "/text"}))?),
                Some(t) if *t == json!(text) => {}
                _ if text.is_empty() => {}
                _ => ops.push(op(json!({"op": "set", "id": id, "path": "/text", "value": text}))?),
            }
        }
        None => {
            if node.get("text").is_some() {
                ops.push(op(json!({"op": "unset", "id": id, "path": "/text"}))?);
            }
            ops.push(op(json!({"op": "set", "id": id, "path": "/content", "value": words}))?);
        }
    }
    Ok(ops)
}

fn op(v: Value) -> Result<Op, EngineError> {
    serde_json::from_value(v).map_err(|e| engine(e.to_string()))
}

fn element_kind(e: &Element) -> &'static str {
    match e {
        Element::Image(_) => "image",
        Element::Text(_) => "text",
        _ => "element",
    }
}

/// Which part of the book an operation changes, given the kinds of nodes.
pub fn part_of(op: &Op, kind_of: &dyn Fn(&str) -> Option<&'static str>) -> Part {
    let words_of = |kind: Option<&str>| kind.and_then(|k| WORD_BLOCKS.iter().find(|(b, _)| *b == k)).map(|(_, f)| *f);
    match op {
        Op::InsertText { .. } | Op::DeleteText { .. } => Part::Words,
        Op::Set { node, path, .. } | Op::Unset { node, path } => match &node.id {
            Some(id) => match words_of(kind_of(id)) {
                Some(fields) if fields.iter().any(|f| path == f || path.starts_with(&format!("{f}/"))) => Part::Words,
                _ => Part::Layout,
            },
            None => Part::Layout,
        },
        Op::Insert { value, .. } => {
            if words_of(value.get("type").and_then(Value::as_str)).is_some() {
                Part::Words
            } else {
                Part::Layout
            }
        }
        Op::Remove { node } | Op::Move { node, .. } => match node.id.as_deref().and_then(kind_of) {
            k if words_of(k).is_some() => Part::Words,
            _ => Part::Layout,
        },
    }
}

/// The field a setting changes: (node, path).
fn field_key(op: &Op) -> Option<(String, String)> {
    let (node, path) = match op {
        Op::Set { node, path, .. } | Op::Unset { node, path } => (node, path),
        _ => return None,
    };
    let target = node.id.clone().or_else(|| node.at.clone().map(|a| format!("@{a}")))?;
    Some((target, path.clone()))
}

fn touched_id(op: &Op) -> Option<String> {
    match op {
        Op::Set { node, .. } | Op::Unset { node, .. } | Op::Remove { node } | Op::Move { node, .. } | Op::InsertText { node, .. } | Op::DeleteText { node, .. } => {
            node.id.clone()
        }
        Op::Insert { value, .. } => value.get("id").and_then(Value::as_str).map(str::to_string),
    }
}
