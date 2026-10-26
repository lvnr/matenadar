// The page editor demo: the engine runs in a worker (dist/client.js), the
// page draws its pages as SVG, and every change goes through the editing
// API. Serve the repository's root (scripts/demo.py says how).

import { MatenadarClient } from "../../dist/client.js";

const $ = (id) => document.getElementById(id);
const encoder = new TextEncoder();
const ROOT = new URL("../../", import.meta.url);

const state = {
  samples: [],
  sample: null,
  pages: [],
  spreads: [],
  zoom: 1,
  guides: true,
  bleed: true,
  sel: null, // {hit, id, node, rects}
  hidden: {},
  fonts: new Set(),
  busy: false,
};

const mt = await MatenadarClient.start({ base: new URL("dist/", ROOT) });
window.mt = mt; // for trying the API from the console

// ---- Status ----

function status(text, error = false) {
  const el = $("status");
  el.textContent = text;
  el.classList.toggle("error-text", error);
}

async function run(label, fn) {
  if (state.busy) return;
  state.busy = true;
  try {
    return await fn();
  } catch (error) {
    status(`${label}: ${error.message}`, true);
  } finally {
    state.busy = false;
  }
}

// ---- Opening a book ----

/**
 * Gives every block and element of the content an id, as Aldus's composed
 * documents have (the samples were written by hand and leave many out).
 */
function withIds(doc) {
  const used = new Set();
  const seen = (v) => {
    if (Array.isArray(v)) v.forEach(seen);
    else if (v && typeof v === "object") {
      if (typeof v.id === "string") used.add(v.id);
      Object.values(v).forEach(seen);
    }
  };
  seen(doc);
  let n = 0;
  const give = (v) => {
    if (Array.isArray(v)) v.forEach(give);
    else if (v && typeof v === "object") {
      if (typeof v.type === "string" && v.id === undefined && !["page-break", "space", "column-break"].includes(v.type)) {
        let id;
        do id = `${v.type}-${++n}`;
        while (used.has(id));
        used.add(id);
        v.id = id;
      }
      Object.values(v).forEach(give);
    }
  };
  give(doc.content);
  return doc;
}

const url = (path) => new URL(path.split("/").map(encodeURIComponent).join("/"), ROOT);

async function open(sample) {
  state.sample = sample;
  state.sel = null;
  state.hidden = {};
  status(`Opening ${sample.title}…`);
  const doc = withIds(await (await fetch(url(sample.doc))).json());
  for (const font of sample.fonts) {
    if (state.fonts.has(font)) continue;
    state.fonts.add(font);
    const response = await fetch(url(font));
    if (response.ok) await mt.addFont(await response.arrayBuffer()).catch(() => 0);
  }
  const { files } = await mt.assets(doc);
  await Promise.all(
    files.map(async (path) => {
      const response = await fetch(url(sample.root + path));
      if (response.ok) await mt.addFile(path, await response.arrayBuffer());
    }),
  );
  const t = performance.now();
  const opened = await mt.open(doc);
  const ms = performance.now() - t;
  await rebuild();
  showIssues(opened.diagnostics);
  await showPatch();
  inspect();
  setUndo(false, false);
  status(`${opened.pages} pages · opened in ${Math.round(ms)} ms`);
}

// ---- Pages ----

function shown(info) {
  const b = state.bleed ? info.bleed : { top: 0, right: 0, bottom: 0, left: 0 };
  return { x: -b.left, y: -b.top, w: info.trim.w + b.left + b.right, h: info.trim.h + b.top + b.bottom };
}

async function rebuild() {
  state.pages = await mt.pages();
  state.spreads = await mt.spreads();
  const host = $("pages");
  host.replaceChildren();
  for (const spread of state.spreads) {
    const wrap = document.createElement("div");
    const row = document.createElement("div");
    row.className = "spread";
    for (const n of spread) row.append(pageElement(state.pages[n - 1]));
    const label = document.createElement("div");
    label.className = "label";
    label.textContent = spread.map((n) => state.pages[n - 1].label || n).join(" · ");
    wrap.append(row, label);
    host.append(wrap);
  }
  await Promise.all(state.pages.map((p) => draw(p.page)));
}

function pageElement(info) {
  const box = shown(info);
  const el = document.createElement("div");
  el.className = "page";
  el.dataset.page = info.page;
  el.style.width = `${box.w * state.zoom}px`;
  el.style.height = `${box.h * state.zoom}px`;
  const art = document.createElement("div");
  art.className = "art";
  const overlay = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  overlay.classList.add("overlay");
  overlay.setAttribute("viewBox", `${box.x} ${box.y} ${box.w} ${box.h}`);
  el.append(art, overlay);
  el.addEventListener("pointerdown", (event) => pointerDown(event, info.page, el));
  return el;
}

const pageEl = (n) => document.querySelector(`.page[data-page="${n}"]`);

async function draw(n) {
  const el = pageEl(n);
  if (!el) return;
  el.querySelector(".art").innerHTML = await mt.svg(n, { bleed: state.bleed });
  overlay(n);
}

function overlay(n, ghost) {
  const el = pageEl(n);
  const info = state.pages[n - 1];
  if (!el || !info) return;
  const box = shown(info);
  const { w, h } = info.trim;
  const parts = [];
  if (state.guides) {
    parts.push(`<path class="bleed" fill-rule="evenodd" d="M${box.x} ${box.y}h${box.w}v${box.h}h${-box.w}z M0 0v${h}h${w}v${-h}z"/>`);
    parts.push(`<rect class="trim" x="0" y="0" width="${w}" height="${h}"/>`);
    const m = info.margins;
    if (m) {
      const left = info.side === "recto" ? m.inside : m.outside;
      const right = info.side === "recto" ? m.outside : m.inside;
      parts.push(`<rect class="margins" x="${left}" y="${m.top}" width="${w - left - right}" height="${h - m.top - m.bottom}"/>`);
    }
    parts.push(`<rect class="safe" x="18" y="18" width="${w - 36}" height="${h - 36}"/>`);
  }
  for (const r of state.sel?.rects ?? []) {
    if (r.page !== n) continue;
    const { x0, y0, x1, y1 } = r.rect;
    parts.push(`<rect class="selection" x="${x0 - 1.5}" y="${y0 - 1.5}" width="${x1 - x0 + 3}" height="${y1 - y0 + 3}"/>`);
  }
  if (ghost) parts.push(`<rect class="selection" x="${ghost.x0}" y="${ghost.y0}" width="${ghost.x1 - ghost.x0}" height="${ghost.y1 - ghost.y0}" stroke-dasharray="4 3"/>`);
  el.querySelector(".overlay").innerHTML = parts.join("");
}

function points(event, n, el) {
  const info = state.pages[n - 1];
  const box = shown(info);
  const rect = el.getBoundingClientRect();
  const scale = rect.width / box.w;
  return { x: (event.clientX - rect.left) / scale + box.x, y: (event.clientY - rect.top) / scale + box.y, scale };
}

// ---- Selecting and dragging ----

async function pointerDown(event, n, el) {
  if (state.busy) return;
  const start = points(event, n, el);
  const hit = await mt.hit(n, start.x, start.y);
  // Pressing on the selected frame (or on something inside it) moves it.
  const sel = state.sel;
  const movable = sel && sel.node && typeof sel.node.x === "number" && typeof sel.node.y === "number";
  const onIt = hit && sel && (hit.id === sel.id || hit.ancestors.some((a) => a.id === sel.id));
  if (movable && onIt) {
    drag(event, n, el, start);
    return;
  }
  await select(hit, hit?.id);
}

function drag(event, n, el, start) {
  const sel = state.sel;
  const rect = sel.rects.find((r) => r.page === n).rect;
  el.classList.add("dragging");
  el.setPointerCapture(event.pointerId);
  let delta = { x: 0, y: 0 };
  const move = (e) => {
    const p = points(e, n, el);
    delta = { x: p.x - start.x, y: p.y - start.y };
    overlay(n, { x0: rect.x0 + delta.x, y0: rect.y0 + delta.y, x1: rect.x1 + delta.x, y1: rect.y1 + delta.y });
  };
  const up = async () => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", up);
    el.classList.remove("dragging");
    if (Math.hypot(delta.x, delta.y) < 1) return overlay(n);
    const round = (v) => Math.round(v * 10) / 10;
    const frame = {};
    if (Math.abs(delta.x) >= 0.1) frame.x = round(sel.node.x + delta.x);
    if (Math.abs(delta.y) >= 0.1) frame.y = round(sel.node.y + delta.y);
    await edit(`Moved ${sel.id}`, () => mt.moveFrame(sel.id, frame));
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", up);
}

async function select(hit, id) {
  if (!hit || !id) {
    state.sel = null;
  } else {
    const [node, rects, mark] = await Promise.all([mt.node(id), mt.rects(id), mt.node(`${id}--break`)]);
    state.sel = { hit, id, node, rects, breakTo: mark ? (mark.to ?? "page") : "" };
  }
  for (const p of state.pages) overlay(p.page);
  inspect();
}

// ---- Editing ----

async function edit(label, fn) {
  await run(label, async () => {
    const t = performance.now();
    const result = await fn();
    const ms = performance.now() - t;
    const before = state.pages.length;
    if (result.page_count !== before) {
      await rebuild();
    } else {
      state.pages = await mt.pages();
      await Promise.all(result.changed_pages.map(draw));
    }
    if (state.sel) await select(state.sel.hit, state.sel.id);
    showIssues(result.diagnostics);
    await showPatch();
    setUndo(result.can_undo, result.can_redo);
    const pages = result.changed_pages.length ? `pages ${result.changed_pages.join(", ")}` : "no page changed";
    const extra = result.lines_after !== undefined ? ` · ${result.lines_before} → ${result.lines_after} lines at ${result.tracking.toFixed(4)} em` : "";
    status(`${label} · ${Math.round(ms)} ms in the worker · ${pages}${extra}`);
  });
}

function setUndo(canUndo, canRedo) {
  $("undo").disabled = !canUndo;
  $("redo").disabled = !canRedo;
}

/** The words of a paragraph or heading as plain text, as the engine counts them. */
function plain(node) {
  if (typeof node.text === "string") return node.text;
  return (node.content ?? []).map((r) => (typeof r === "string" ? r : typeof r.text === "string" ? r.text : "")).join("");
}

/** One change of plain text as delete-text and insert-text (byte offsets), so marks stay. */
function textOps(id, before, after) {
  let p = 0;
  while (p < before.length && p < after.length && before[p] === after[p]) p++;
  let s = 0;
  while (s < before.length - p && s < after.length - p && before[before.length - 1 - s] === after[after.length - 1 - s]) s++;
  const start = encoder.encode(before.slice(0, p)).length;
  const end = encoder.encode(before.slice(0, before.length - s)).length;
  const text = after.slice(p, after.length - s);
  const ops = [];
  if (end > start) ops.push({ op: "delete-text", id, start, end });
  if (text) ops.push({ op: "insert-text", id, offset: start, text });
  return ops;
}

const pt = (v) => {
  const n = length(v);
  return n === "" || n === undefined ? "" : Math.round(n * 100) / 100;
};

const length = (v) => {
  if (typeof v === "number") return v;
  const m = /^(-?[\d.]+)(pt|in|mm|cm|px)?$/.exec(String(v ?? "").trim());
  if (!m) return "";
  const n = Number(m[1]);
  return { pt: n, undefined: n, in: n * 72, mm: (n * 72) / 25.4, cm: (n * 72) / 2.54, px: n * 0.75 }[m[2]];
};

// ---- The inspector ----

function inspect() {
  const panel = $("inspector");
  const sel = state.sel;
  if (!sel) {
    panel.innerHTML = `<h2>Selection</h2><p class="hint">Click anything on a page. Drag a selected frame on a designed page to move it.</p>`;
    return;
  }
  const { hit, id } = sel;
  const node = sel.node ?? {};
  const type = node.type ?? hit.kind;
  const chain = [hit.id, ...hit.ancestors.map((a) => a.id)].filter((v, i, all) => all.indexOf(v) === i);
  const html = [`<h2>Selection</h2><div class="crumbs">${chain.map((c) => `<button data-crumb="${c}" class="${c === id ? "on" : ""}">${c}</button>`).join("")}</div>`];
  html.push(`<p class="note">${type} on page ${hit.page}${hit.offset !== undefined ? ` · at byte ${hit.offset}` : ""}</p>`);

  if (type === "p" || type === "h") {
    const s = node.set ?? {};
    html.push(`<div class="sub">Words</div><textarea id="words">${escape(plain(node))}</textarea>
      <div class="row"><button id="apply-words">Apply words</button></div>
      <div class="sub">Block</div>
      <div class="fields">
        <label>Space before</label><input type="number" id="space_before" step="1" value="${pt(s.space_before)}" placeholder="style">
        <label>Space after</label><input type="number" id="space_after" step="1" value="${pt(s.space_after)}" placeholder="style">
        <label>Align</label><select id="align">${options(["", "left", "center", "right", "justify"], s.align ?? "", "style")}</select>
        <label>Keep with next</label><select id="keep_with_next">${options(["", "true", "false"], s.keep_with_next === undefined ? "" : String(s.keep_with_next), "style")}</select>
        <label>Break before</label><select id="break">${options(["", "page", "recto", "verso"], sel.breakTo ?? "", "none")}</select>
        <label>On its page, hide</label><select id="hide">${options(["", "all", "folio", "running"], (state.hidden[id] ?? [])[0] ?? "", "nothing")}</select>
      </div>`);
    if (type === "p") html.push(`<div class="row"><button id="nudge-less">One line shorter</button><button id="nudge-more">One line longer</button></div>`);
  }
  if (type === "image" || type === "figure") {
    const c = node.crop ?? { x: 0, y: 0, w: 1, h: 1 };
    const f = node.focus ?? [0.5, 0.5];
    html.push(`<div class="sub">Picture</div><div class="fields">
      ${["x", "y", "w", "h"].map((k) => `<label>Crop ${k}</label><input type="range" min="0" max="1" step="0.01" data-crop="${k}" value="${c[k] ?? (k === "w" || k === "h" ? 1 : 0)}">`).join("")}
      <label>Fit</label><select id="fit">${options(["cover", "contain", "stretch"], node.fit ?? (type === "figure" ? "contain" : "cover"))}</select>
      <label>Focus x</label><input type="range" min="0" max="1" step="0.01" data-focus="0" value="${f[0]}">
      <label>Focus y</label><input type="range" min="0" max="1" step="0.01" data-focus="1" value="${f[1]}">
      <label>Replace</label><input type="file" id="replace" accept="image/*">
    </div><div class="row"><button id="uncrop">Whole picture</button></div>`);
  }
  if (typeof node.x === "number" || typeof node.y === "number") {
    html.push(`<div class="sub">Frame</div><div class="fields">
      ${["x", "y", "w", "h"].map((k) => `<label>${k}</label><input type="number" step="0.5" data-frame="${k}" value="${pt(node[k])}">`).join("")}
    </div>`);
  }
  html.push(`<details><summary>Node</summary><pre>${escape(JSON.stringify(node, null, 1))}</pre></details>`);
  panel.innerHTML = html.join("");
  wire(panel, sel);
}

function options(values, current, empty = "") {
  return values.map((v) => `<option value="${v}" ${v === current ? "selected" : ""}>${v || empty}</option>`).join("");
}

function escape(text) {
  return String(text).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}

function wire(panel, sel) {
  const { id, node } = sel;
  panel.querySelectorAll("[data-crumb]").forEach((b) =>
    b.addEventListener("click", () => select(sel.hit, b.dataset.crumb)),
  );
  const on = (selector, event, fn) => panel.querySelector(selector)?.addEventListener(event, fn);
  on("#apply-words", "click", () => {
    const ops = textOps(id, plain(node), panel.querySelector("#words").value);
    if (ops.length) edit(`Words of ${id}`, () => mt.apply(ops));
  });
  for (const key of ["space_before", "space_after"]) {
    on(`#${key}`, "change", (e) => edit(`${key} of ${id}`, () => mt.setBlock(id, { [key]: e.target.value === "" ? null : `${e.target.value}pt` })));
  }
  on("#align", "change", (e) => edit(`Align ${id}`, () => mt.setBlock(id, { align: e.target.value || null })));
  on("#keep_with_next", "change", (e) => edit(`Keep ${id}`, () => mt.setBlock(id, { keep_with_next: e.target.value === "" ? null : e.target.value === "true" })));
  on("#break", "change", (e) => {
    edit(`Break before ${id}`, () => mt.breakBefore(id, e.target.value || null));
  });
  on("#hide", "change", (e) => {
    state.hidden[id] = e.target.value ? [e.target.value] : [];
    edit(`Page of ${id}`, () => mt.hideOnPage(id, state.hidden[id]));
  });
  on("#nudge-less", "click", () => edit(`Nudged ${id}`, () => mt.nudge(id, -1)));
  on("#nudge-more", "click", () => edit(`Nudged ${id}`, () => mt.nudge(id, 1)));
  panel.querySelectorAll("[data-crop]").forEach((input) =>
    input.addEventListener("change", () => {
      const crop = {};
      panel.querySelectorAll("[data-crop]").forEach((i) => (crop[i.dataset.crop] = Number(i.value)));
      crop.w = Math.min(crop.w, 1 - crop.x) || 0.01;
      crop.h = Math.min(crop.h, 1 - crop.y) || 0.01;
      edit(`Cropped ${id}`, () => mt.setPicture(id, { crop }));
    }),
  );
  on("#uncrop", "click", () => edit(`Uncropped ${id}`, () => mt.setPicture(id, { crop: null, focus: null })));
  on("#fit", "change", (e) => edit(`Fit of ${id}`, () => mt.setPicture(id, { fit: e.target.value })));
  panel.querySelectorAll("[data-focus]").forEach((input) =>
    input.addEventListener("change", () => {
      const focus = [...panel.querySelectorAll("[data-focus]")].map((i) => Number(i.value));
      edit(`Focus of ${id}`, () => mt.setPicture(id, { focus }));
    }),
  );
  on("#replace", "change", async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const path = `uploads/${file.name}`;
    await mt.addFile(path, await file.arrayBuffer());
    edit(`Replaced ${id}`, () => mt.setPicture(id, { src: path }));
  });
  panel.querySelectorAll("[data-frame]").forEach((input) =>
    input.addEventListener("change", () => {
      if (input.value === "") return;
      edit(`Frame of ${id}`, () => mt.moveFrame(id, { [input.dataset.frame]: Number(input.value) }));
    }),
  );
}

// ---- Diagnostics and the patch ----

function showIssues(issues) {
  $("count").textContent = issues.length ? `(${issues.length})` : "";
  const list = $("issues");
  list.replaceChildren(
    ...issues.slice(0, 200).map((issue) => {
      const li = document.createElement("li");
      li.className = issue.severity;
      li.innerHTML = `<span class="code">${issue.code}</span>${issue.page ? ` · p. ${issue.page}` : ""} — ${escape(issue.message)}`;
      li.addEventListener("click", async () => {
        if (issue.page) pageEl(issue.page)?.scrollIntoView({ behavior: "smooth", block: "center" });
        if (issue.node) {
          const rects = await mt.rects(issue.node);
          const r = rects.find((x) => x.page === issue.page) ?? rects[0];
          if (r) await select({ id: issue.node, kind: "", page: r.page, rect: r.rect, ancestors: [] }, issue.node);
        }
      });
      return li;
    }),
  );
  if (!issues.length) list.innerHTML = `<li class="hint">Nothing to fix.</li>`;
}

async function showPatch() {
  $("patch").textContent = JSON.stringify(await mt.patch(), null, 1);
}

// ---- The bar ----

$("undo").addEventListener("click", () => edit("Undone", () => mt.undo()));
$("redo").addEventListener("click", () => edit("Redone", () => mt.redo()));
document.addEventListener("keydown", (e) => {
  if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "z" || e.target.closest("textarea, input")) return;
  e.preventDefault();
  if (e.shiftKey ? !$("redo").disabled : !$("undo").disabled) e.shiftKey ? $("redo").click() : $("undo").click();
});
$("zoom").addEventListener("change", (e) => {
  state.zoom = Number(e.target.value);
  run("Zoom", rebuild);
});
$("guides").addEventListener("change", (e) => {
  state.guides = e.target.checked;
  state.pages.forEach((p) => overlay(p.page));
});
$("bleed").addEventListener("change", (e) => {
  state.bleed = e.target.checked;
  run("Bleed", rebuild);
});
$("pdf").addEventListener("click", () =>
  run("PDF", async () => {
    const pdf = await mt.pdf();
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([pdf], { type: "application/pdf" }));
    a.download = `${state.sample.slug}.pdf`;
    a.click();
  }),
);
$("sample").addEventListener("change", (e) => run("Open", () => open(state.samples.find((s) => s.slug === e.target.value))));

state.samples = await (await fetch(new URL("samples.json", import.meta.url))).json();
const first = new URLSearchParams(location.search).get("book") ?? "the-machine-in-the-back-of-the-book";
$("sample").innerHTML = state.samples.map((s) => `<option value="${s.slug}" ${s.slug === first ? "selected" : ""}>${escape(s.title)}</option>`).join("");
await run("Open", () => open(state.samples.find((s) => s.slug === first) ?? state.samples[0]));
