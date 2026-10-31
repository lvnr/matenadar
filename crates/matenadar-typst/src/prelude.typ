// Matenadar's layout library. The compiler includes it ahead of every
// generated document; nothing outside the engine sees it.

// Sections and pages leave a <mt-sec> marker on their first page and an
// <mt-end> marker where they end.
#let mt-sec(info) = [#metadata(info) <mt-sec>]
#let mt-end(id) = [#metadata(id) <mt-end>]

#let mt-pages-of(label) = query(label).map(m => m.location().page())

// The section or page the current page belongs to.
#let mt-cur() = {
  let p = here().page()
  query(<mt-sec>).filter(m => m.location().page() <= p).at(-1, default: none)
}

#let mt-is-first() = mt-pages-of(<mt-sec>).contains(here().page())

// A page left empty so the next section opens on the right side.
#let mt-is-blank() = {
  let p = here().page()
  let starts = mt-pages-of(<mt-sec>)
  if starts.contains(p) or not starts.contains(p + 1) { return false }
  let ends = mt-pages-of(<mt-end>)
  not ends.contains(p)
}

#let mt-is-recto() = calc.odd(here().page())

// Whether the page leaves out a master element known by `names` (its id,
// its style): a node on the page asked (`on_page.hide`).
#let mt-hidden(names) = {
  let p = here().page()
  query(<mt-hide>).any(m => m.location().page() == p and m.value.any(h => h == "all" or h in names))
}

// A length given relative to a container, made absolute.
#let mt-abs(v, full) = if type(v) == ratio { v * full } else if type(v) == relative { v.ratio * full + v.length.to-absolute() } else { v.to-absolute() }

// A picture cut to `crop` (x, y, w, h as fractions of the picture), fitted
// into its box as if that part were the whole: `cover` fills the box (the
// window on the part chosen by `focus`), `contain` shows all of it,
// `stretch` distorts it to the box. Without a width or height, the box
// takes the part's proportions.
// `draw(w, h)`, written where the picture is placed, draws it at a size, so
// that the picture on the page is traced back to its element.
#let mt-crop(src, crop, width: auto, height: auto, fit: "cover", focus: (0.5, 0.5), draw: none) = layout(size => {
  let nat = measure(image(src))
  let (cx, cy, cw, ch) = crop
  let (pw, ph) = (nat.width * cw, nat.height * ch)
  let w = if width == auto { none } else { mt-abs(width, size.width) }
  let h = if height == auto { none } else { mt-abs(height, size.height) }
  if w == none and h == none { w = size.width }
  if w == none { w = h * (pw / ph) }
  if h == none { h = w * (ph / pw) }
  let (sx, sy) = if fit == "stretch" { (w / pw, h / ph) } else {
    let s = if fit == "contain" { calc.min(w / pw, h / ph) } else { calc.max(w / pw, h / ph) }
    (s, s)
  }
  let (vw, vh) = (pw * sx, ph * sy)
  let (ww, wh) = (calc.min(vw, w), calc.min(vh, h))
  box(width: w, height: h, place(top + left, dx: (w - ww) / 2, dy: (h - wh) / 2,
    box(width: ww, height: wh, clip: true, place(top + left,
      dx: -nat.width * cx * sx - (vw - ww) * focus.at(0),
      dy: -nat.height * cy * sy - (vh - wh) * focus.at(1),
      if draw == none { image(src, width: nat.width * sx, height: nat.height * sy, fit: "stretch") } else { draw(nat.width * sx, nat.height * sy) }))))
})

// A paragraph at the narrowest width that keeps its number of lines.
#let mt-balanced(al, body) = layout(size => {
  let w = size.width
  let full = measure(block(width: w, body)).height
  let lo = w * 0.35
  let hi = w
  for _ in range(12) {
    let mid = (lo + hi) / 2
    if measure(block(width: mid, body)).height > full + 0.01pt { lo = mid } else { hi = mid }
  }
  align(al, block(width: hi, body))
})

// Blocks shared out over `n` columns that end level. Blocks stay whole,
// and keep the spacing between them.
#let mt-balance(n, gutter, rule, items) = layout(size => {
  let colw = (size.width - gutter * (n - 1)) / n
  let hs = items.map(it => measure(block(width: colw, it)).height)
  // The space between each block and the next, as the flow sets it.
  let gaps = range(calc.max(items.len() - 1, 0)).map(i =>
    calc.max(0pt, measure(block(width: colw, items.at(i) + items.at(i + 1))).height - hs.at(i) - hs.at(i + 1)))
  let total = hs.sum(default: 0pt) + gaps.sum(default: 0pt)
  // The smallest column height that fits everything in n columns, in order.
  let fits(cap) = {
    let cols = 1
    let used = 0pt
    for (i, h) in hs.enumerate() {
      let add = if used > 0pt { gaps.at(i - 1) + h } else { h }
      if used + add > cap and used > 0pt { cols += 1; used = h } else { used += add }
    }
    cols <= n
  }
  let lo = calc.max(..hs, total / n)
  let hi = total
  for _ in range(20) {
    let mid = (lo + hi) / 2
    if fits(mid) { hi = mid } else { lo = mid }
  }
  let cols = ((),)
  let used = 0pt
  for (i, h) in hs.enumerate() {
    let add = if used > 0pt { gaps.at(i - 1) + h } else { h }
    if used + add > hi + 0.01pt and used > 0pt { cols.push(()); used = h } else { used += add }
    cols.last().push(items.at(i))
  }
  while cols.len() < n { cols.push(()) }
  grid(
    columns: (colw,) * n,
    column-gutter: gutter,
    stroke: (x, y) => if x > 0 and rule != none { (left: rule) } else { none },
    inset: (x, y) => if x > 0 and rule != none { (left: gutter / 2) } else { 0pt },
    ..cols.map(c => c.join()),
  )
})

// A list marker drawn on a shape, centred on the marker's capitals.
#let mt-marker(shape, body) = box({
  set text(top-edge: "cap-height", bottom-edge: "baseline")
  place(center + horizon, shape)
  body
})

#let mt-wrap(st, body) = {
  let w = st.at("wrap", default: none)
  if w == none { body } else { w(body) }
}

// Text over a halo: the same text stroked in the halo's colour, behind it.
#let mt-halo(s, body) = box({
  place(top + left, text(stroke: (paint: s.paint, thickness: s.thickness * 2, join: "round"), fill: s.paint, body))
  body
})

// Letter spacing after a run's last letter, which Typst leaves out at run
// boundaries. The word joiner after it keeps the line from breaking there.
#let mt-tr = context { let t = text.tracking; if t != 0pt { h(t) + "\u{2060}" } }

// A character style or inline overrides.
#let mt-span(st, body) = mt-wrap(st, text(..st.text, body))

// A paragraph in a style (see the compiler's style dictionaries). `k`
// scales the type, for frames that shrink to fit.
#let mt-par(st, k: 1, first: false, w: 100%, body) = {
  let tx = st.at("text", default: (:))
  if k != 1 {
    if "size" in tx { tx.insert("size", tx.size * k) }
    if "tracking" in tx { tx.insert("tracking", tx.tracking * k) }
  }
  if "costs" in st { tx.insert("costs", st.costs) }
  let indent = if first and not st.at("indent-first", default: false) { 0pt } else { st.at("indent", default: 0pt) }
  let al = st.at("align", default: "left")
  // An explicit paragraph: Typst only indents and hangs real paragraphs.
  let body = par(mt-wrap(st, body))
  let a = if al == "right" { right } else if al == "center" { center } else { left }
  let shaped = if al == "outside" or al == "inside" {
    context {
      let r = (al == "outside") == calc.odd(here().page())
      align(if r { right } else { left }, body)
    }
  } else if st.at("balance", default: false) {
    mt-balanced(a, body)
  } else if al == "left" or al == "justify" {
    body
  } else {
    align(a, body)
  }
  let inner = {
    set text(..tx)
    set par(justify: al == "justify", leading: 0pt, spacing: 0pt, first-line-indent: (amount: indent, all: true), hanging-indent: st.at("hanging", default: 0pt))
    if "linebreaks" in st { set par(linebreaks: st.linebreaks) }
    if "limits" in st { set par(justification-limits: st.limits) }
    if "rule-above" in st {
      let r = st.at("rule-above")
      block(above: 0pt, below: r.gap, line(length: r.width, stroke: r.stroke))
    }
    shaped
    if "rule-below" in st {
      let r = st.at("rule-below")
      block(above: r.gap, below: 0pt, line(length: r.width, stroke: r.stroke))
    }
  }
  block(
    above: st.at("above", default: 0pt) * k,
    below: st.at("below", default: 0pt) * k,
    sticky: st.at("sticky", default: false),
    breakable: st.at("breakable", default: true),
    fill: st.at("fill", default: none),
    inset: st.at("inset", default: 0pt),
    width: w,
    if "left" in st or "right" in st { pad(left: st.at("left", default: 0pt), right: st.at("right", default: 0pt), inner) } else { inner },
  )
}

// A paragraph opening with a drop cap `lines` lines deep. The cap's top
// lines up with the first line's capitals and its foot sits on the
// baseline of line `lines`; the words beside it are found by measuring.
#let mt-drop(st, k: 1, first: false, cap, rest) = layout(size => {
  let d = st.drop
  let tx = st.at("text", default: (:))
  let n = d.lines
  let base = tx.at("size", default: 11pt) * k
  let lh = d.lh * base
  let probe(f) = text(..tx, ..f, top-edge: "cap-height", bottom-edge: "baseline", "H")
  let body-cap = measure(probe((size: base))).height
  let cap-tx = (size: 100pt)
  if "font" in d { cap-tx.insert("font", d.font) }
  if "weight" in d { cap-tx.insert("weight", d.weight) }
  let ratio = measure(probe(cap-tx)).height / 100pt
  cap-tx.insert("size", ((n - 1) * lh + body-cap) / ratio)
  if "fill" in d { cap-tx.insert("fill", d.fill) }
  let cap-box = text(..tx, ..cap-tx, top-edge: "cap-height", bottom-edge: "baseline", cap)
  let capw = measure(cap-box).width + d.gap
  let top = tx.at("top-edge", default: 0.8em)
  let lead = top.em * base + top.abs - body-cap
  let narrow = size.width - capw
  // The words: one string, or an array of words, each with the space after
  // it (content that keeps each letter's place in the paragraph's source).
  let (words, sep) = if type(rest) == str { (rest.split(" "), " ") } else { (rest, none) }
  let head = st + (below: 0pt)
  let tail = st + (above: 0pt, indent-first: false)
  let part(ws) = mt-par(head, k: k, first: true, w: narrow, ws.join(sep))
  let lo = 0
  let hi = words.len()
  while lo < hi {
    let mid = calc.ceil((lo + hi) / 2)
    if measure(part(words.slice(0, mid))).height <= n * lh + 0.5pt { lo = mid } else { hi = mid - 1 }
  }
  block(above: st.at("above", default: 0pt) * k, below: 0pt, grid(
    columns: (capw, narrow),
    move(dy: lead, cap-box),
    mt-par(head + (above: 0pt), k: k, first: true, w: narrow, [#words.slice(0, lo).join(sep)#if lo < words.len() { linebreak(justify: true) }]),
  ))
  if lo < words.len() { mt-par(tail, k: k, first: true, words.slice(lo).join(sep)) }
})

// A heading: a paragraph in its style that is also a Typst heading (for the
// PDF's bookmarks) and leaves a <mt-h> marker for contents and running heads.
#let mt-head(level, outlined, title, st, k: 1, body) = {
  show heading: it => mt-par(st, k: k, first: true, [#metadata((level: level, title: title, outlined: outlined)) <mt-h>#it.body])
  heading(level: level, outlined: outlined, bookmarked: outlined, body)
}

// A date written `YYYY-MM-DD`, shown as `format` (Typst's date format).
#let mt-date(s, format) = {
  let p = s.split("-")
  if p.len() != 3 { return s }
  datetime(year: int(p.at(0)), month: int(p.at(1)), day: int(p.at(2))).display(format)
}

#let mt-field-of(name) = {
  let cur = mt-cur()
  let info = if cur == none { (:) } else { cur.value }
  if name == "page" {
    let n = info.at("numbering", default: "1")
    if n == none { n = "1" }
    counter(page).display(n)
  } else if name == "pages" {
    str(counter(page).final().first())
  } else if name == "heading" {
    let p = here().page()
    let hs = query(<mt-h>).filter(h => h.value.level == 1 and h.location().page() <= p)
    if hs.len() > 0 { hs.last().value.title } else { [] }
  } else if name == "doc.title" {
    let t = document.title
    if t == none { "" } else { t }
  } else if name == "doc.author" {
    document.author.join(", ")
  } else if name == "section.title" {
    info.at("title", default: "")
  } else if name.starts-with("section.") {
    info.at("vars", default: (:)).at(name.slice(8), default: "")
  } else { "" }
}

#let mt-field(name, format: none) = {
  let v = mt-field-of(name)
  if format != none and type(v) == str { mt-date(v, format) } else { v }
}

// Places `body` with the anchor point (fractions of its size) at (x, y).
#let mt-at(x, y, fx, fy, body) = {
  if fx == 0 and fy == 0 {
    place(top + left, dx: x, dy: y, body)
  } else {
    place(top + left, dx: x, dy: y, context {
      let s = measure(body)
      move(dx: -s.width * fx, dy: -s.height * fy, body)
    })
  }
}

// A QR code: `runs` of dark modules (x, y, length) on an n × n grid.
#let mt-qr(w, n, runs, dark, light) = box(width: w, height: w, {
  let m = w / n
  if light != none { place(rect(width: w, height: w, fill: light, stroke: none)) }
  place(curve(fill: dark, stroke: none, ..runs.map(r => (
    curve.move((r.at(0) * m, r.at(1) * m)),
    curve.line((r.at(2) * m, 0pt), relative: true),
    curve.line((0pt, m), relative: true),
    curve.line((-r.at(2) * m, 0pt), relative: true),
    curve.close(),
  )).flatten()))
})

#let mt-rot(angle, body) = if angle == 0deg { body } else { rotate(angle, origin: center + horizon, reflow: false, body) }

// Content set at the scale that makes its longest line fill `w`.
#let mt-fitw(w, kmin, kmax, make) = context {
  let nat = measure(make(1)).width
  let k = if nat > 0pt and type(w) == length { calc.clamp(w / nat, kmin, kmax) } else { 1 }
  make(k)
}

// Content set at scale k until it fits `h`, down to `kmin`.
#let mt-shrink(w, h, kmin, make) = context {
  let k = 1.0
  while k > kmin and measure(block(width: w, make(k))).height > h { k = k - 0.02 }
  make(k)
}

// Overflow of a frame of fixed height, recorded for checks.
#let mt-overflow(id, w, h, body) = context {
  let m = measure(block(width: w, body))
  if m.height > h + 0.5pt { [#metadata((id: id, over: (m.height - h).pt())) <mt-overflow>] }
}

// A margin note in the outer (or given) column, level with `anchor` when
// that marker exists on this page, else where the note stands.
#let mt-note-bottom = state("mt-note-bottom", (0, 0pt))

#let mt-note(side, w, gap, spacing, anchor, dy, body) = context {
  let recto = calc.odd(here().page())
  let right = side == "right" or (side == "outside" and recto) or (side == "inside" and not recto)
  let x = if right { 100% + gap } else { -gap - w }
  let here-pos = here().position()
  let y = here-pos.y
  if anchor != none {
    let marks = query(anchor)
    if marks.len() > 0 {
      let pos = marks.first().location().position()
      if pos.page == here-pos.page { y = pos.y }
    }
  }
  y = y + dy
  let last = mt-note-bottom.get()
  if last.at(0) == here-pos.page and y < last.at(1) + spacing { y = last.at(1) + spacing }
  let h = measure(block(width: w, body)).height
  mt-note-bottom.update((here-pos.page, y + h))
  place(left, dx: x, dy: y - here-pos.y, block(width: w, body))
}

// The contents, from the headings' markers.
#let mt-toc(depth, styles, leader) = context {
  for m in query(<mt-h>).filter(m => m.value.outlined and m.value.level <= depth) {
    let st = styles.at(m.value.level - 1, default: styles.last())
    let loc = m.location()
    let num = counter(page).at(loc).first()
    let cur = query(<mt-sec>).filter(s => s.location().page() <= loc.page()).at(-1, default: none)
    let fmt = if cur == none { "1" } else { cur.value.at("numbering", default: "1") }
    if fmt == none { fmt = "1" }
    mt-par(st, first: true, link(loc, {
      m.value.title
      if leader == none { h(1fr) } else { box(width: 1fr, repeat(gap: 0.3em, leader)) }
      numbering(fmt, num)
    }))
  }
}

// A page number of a labelled node.
#let mt-page-of(lab) = context {
  let found = query(lab)
  if found.len() == 0 { "??" } else {
    let loc = found.first().location()
    let cur = query(<mt-sec>).filter(m => m.location().page() <= loc.page()).at(-1, default: none)
    let fmt = if cur == none { "1" } else { cur.value.at("numbering", default: "1") }
    if fmt == none { fmt = "1" }
    numbering(fmt, counter(page).at(loc).first())
  }
}

// One page's part of a picture laid across two facing pages. The picture,
// cut to `crop`, covers `total` (both pages' width with their outer bleeds,
// and the height with both bleeds), leaving `focus` in view; this page shows
// the window `size` wide that starts `from` along it.
// `draw(w, h)`, written where the spread is, draws the picture at a size.
#let mt-spread-part(src, crop, focus, total, from, size, draw) = context {
  let nat = measure(image(src))
  let (cx, cy, cw, ch) = crop
  let (pw, ph) = (nat.width * cw, nat.height * ch)
  let s = calc.max(total.at(0) / pw, total.at(1) / ph)
  let ox = (pw * s - total.at(0)) * focus.at(0)
  let oy = (ph * s - total.at(1)) * focus.at(1)
  box(width: size.at(0), height: size.at(1), clip: true, place(top + left,
    dx: -nat.width * cx * s - ox - from,
    dy: -nat.height * cy * s - oy,
    draw(nat.width * s, nat.height * s)))
}

// A plate's picture, whole, as large as the space allows, with its
// caption under it; the two are centred as one on the space.
// `draw(w, h)` draws the picture (cut to `crop`) at a size.
#let mt-plate(src, crop, draw, caption, gap) = layout(size => {
  let gap = gap.to-absolute()
  let nat = measure(image(src))
  let aspect = (nat.width * crop.at(2)) / (nat.height * crop.at(3))
  let cap = if caption == none { none } else { block(width: size.width, caption) }
  let ch = if cap == none { 0pt } else { measure(cap).height + gap }
  let w = calc.min(size.width, (size.height - ch) * aspect)
  let h = w / aspect
  box(width: size.width, height: size.height, place(center + horizon,
    stack(spacing: gap, box(width: w, height: h, draw(w, h)), ..if cap != none { (cap,) })))
})

// A numbered figure's number or label (`3.2`, `Figure 3.2`), found by id.
#let mt-fig-ref(id, part) = context {
  let found = query(<mt-fig>).filter(m => m.value.id == id)
  if found.len() == 0 { "??" } else { found.first().value.at(part) }
}

// The list of figures, from the figures' markers.
#let mt-lof(st, leader) = context {
  for m in query(<mt-fig>) {
    let loc = m.location()
    let num = counter(page).at(loc).first()
    let cur = query(<mt-sec>).filter(s => s.location().page() <= loc.page()).at(-1, default: none)
    let fmt = if cur == none { "1" } else { cur.value.at("numbering", default: "1") }
    if fmt == none { fmt = "1" }
    mt-par(st, first: true, link(loc, {
      m.value.number
      h(0.6em)
      m.value.caption
      if leader == none { h(1fr) } else { box(width: 1fr, repeat(gap: 0.3em, leader)) }
      numbering(fmt, num)
    }))
  }
}

// A soft panel for words over a picture: `opacity` at its middle, fading
// out over `feather` at its edge (layers whose opacities add up to it).
#let mt-scrim(w, h, paint, opacity, feather, radius, shape) = {
  let n = if feather > 0pt { 8 } else { 1 }
  let a = 1 - calc.pow(1 - opacity, 1 / n)
  let fill = paint.transparentize(100% - a * 100%)
  box(width: w, height: h, for i in range(n) {
    let d = feather * i / n
    let (sw, sh) = (calc.max(w - 2 * d, 0pt), calc.max(h - 2 * d, 0pt))
    place(top + left, dx: d, dy: d, if shape == "ellipse" {
      ellipse(width: sw, height: sh, fill: fill, stroke: none)
    } else {
      rect(width: sw, height: sh, fill: fill, stroke: none, radius: calc.max(radius - d, 0pt))
    })
  })
}

// `body` placed as `mt-at` places it, then moved (never resized) so it
// stays inside the safe zone of a `size` page: `zone` is (top, bottom,
// inside, outside), mirrored on versos, or (top, bottom, left, right).
#let mt-safe-at(x, y, fx, fy, zone, mirrored, size, body) = context {
  let s = measure(body)
  let (pw, ph) = size
  let (zt, zb, za, zc) = zone
  let (l, r) = if mirrored and not calc.odd(here().page()) { (zc, za) } else { (za, zc) }
  let x0 = mt-abs(x, pw) - s.width * fx
  let y0 = mt-abs(y, ph) - s.height * fy
  x0 = calc.max(l, calc.min(x0, pw - r - s.width))
  y0 = calc.max(zt, calc.min(y0, ph - zb - s.height))
  place(top + left, dx: x0, dy: y0, body)
}

// A logic-grid puzzle: `cats` is ((name, (item, …)), …), every category the
// same length. The first category runs down the side with the others across
// the top; the last ones go down the side under it, in the usual staircase.
// `marks` are (category, item, category, item, mark): "o" a dot, "x" a
// cross, anything else written in the cell.
#let mt-logic(cats, cell, marks, st, lines, frame) = context {
  let k = cats.len()
  let n = cats.at(0).at(1).len()
  let tx = st.at("text", default: (:))
  let ink = tx.at("fill", default: black)
  let lab(s) = mt-span(st, text(top-edge: "cap-height", bottom-edge: "baseline", s))
  let pad = 4pt
  let across = range(1, k)
  let down = (0,) + range(k - 1, 1, step: -1)
  let up(s) = rotate(-90deg, reflow: true, lab(s))
  let item-w(c) = calc.max(0pt, ..cats.at(c).at(1).map(i => measure(lab(i)).width))
  let name-h = measure(lab("Hg")).height
  let head = calc.max(0pt, ..across.map(item-w)) + pad * 2
  let sidew = calc.max(0pt, ..down.map(item-w)) + pad * 2
  let x0 = name-h + pad * 2 + sidew
  let y0 = name-h + pad * 2 + head
  let bw = cell * n
  let cols(r) = if r == 0 { across.len() } else { across.len() - r }
  box(width: x0 + bw * across.len(), height: y0 + bw * down.len(), {
    for (j, c) in across.enumerate() {
      let x = x0 + bw * j
      place(top + left, dx: x, dy: 0pt, box(width: bw, height: name-h + pad, align(center + top, lab(cats.at(c).at(0)))))
      for (i, it) in cats.at(c).at(1).enumerate() {
        let s = measure(up(it))
        place(top + left, dx: x + cell * i + (cell - s.width) / 2, dy: y0 - pad - s.height, up(it))
      }
    }
    for (r, c) in down.enumerate() {
      let y = y0 + bw * r
      place(top + left, dx: 0pt, dy: y, box(width: name-h + pad, height: bw, align(left + horizon, up(cats.at(c).at(0)))))
      for (i, it) in cats.at(c).at(1).enumerate() {
        place(top + left, dx: name-h + pad * 2, dy: y + cell * i, box(width: sidew - pad, height: cell, align(right + horizon, lab(it))))
      }
      for j in range(cols(r)) {
        let (bx, by) = (x0 + bw * j, y)
        for a in range(1, n) {
          place(top + left, line(start: (bx + cell * a, by), end: (bx + cell * a, by + bw), stroke: lines))
          place(top + left, line(start: (bx, by + cell * a), end: (bx + bw, by + cell * a), stroke: lines))
        }
        place(top + left, dx: bx, dy: by, rect(width: bw, height: bw, fill: none, stroke: frame))
      }
    }
    for (ac, ai, bc, bi, m) in marks {
      let at = none
      for (r, sc) in down.enumerate() {
        for j in range(cols(r)) {
          let tc = across.at(j)
          if sc == ac and tc == bc { at = (r, j, ai, bi) }
          if sc == bc and tc == ac { at = (r, j, bi, ai) }
        }
      }
      if at != none {
        let (r, j, row, col) = at
        let cx = x0 + bw * j + cell * col
        let cy = y0 + bw * r + cell * row
        place(top + left, dx: cx, dy: cy, box(width: cell, height: cell, align(center + horizon, if m == "o" {
          circle(radius: cell * 0.2, fill: ink, stroke: none)
        } else if m == "x" {
          let d = cell * 0.22
          box(width: 2 * d, height: 2 * d, {
            place(line(start: (0pt, 0pt), end: (2 * d, 2 * d), stroke: 0.8pt + ink))
            place(line(start: (0pt, 2 * d), end: (2 * d, 0pt), stroke: 0.8pt + ink))
          })
        } else { lab(m) })))
      }
    }
  })
}

// A comic balloon round `body`, the words, set at most `maxw` wide in lines
// of even length: `speech` and `whisper` (dashed) ellipses, a `thought`
// cloud, a `shout` burst or a `caption` box. The tail runs from inside the
// balloon to `tail`, a point given from the balloon's anchor (`fx`, `fy` of
// its size); balloon and tail are one outline.
#let mt-balloon(shape, body, longest, st, maxw, inset, tail, fx, fy, fill, pen) = context {
  let words = { set align(center); body }
  let maxw = if maxw == none { 11em.to-absolute() } else { maxw.to-absolute() }
  let inset = inset.to-absolute()
  // The narrowest width that keeps as many lines, with no word sticking out.
  let floor = calc.max(0pt, ..longest.map(w => measure(mt-span(st, w)).width))
  let one = measure(words).width
  let maxw = calc.max(maxw, floor)
  let full = measure(block(width: maxw, words)).height
  let (lo, hi) = (floor, calc.min(one, maxw))
  for _ in range(12) {
    let mid = (lo + hi) / 2
    if measure(block(width: mid, words)).height > full + 0.01pt { lo = mid } else { hi = mid }
  }
  let (tw, th) = (hi, full)
  let (w, h) = if shape == "caption" { (tw + 2 * inset, th + 2 * inset) } else { ((tw + 2 * inset) * calc.sqrt(2), (th + 2 * inset) * calc.sqrt(2)) }
  let s = stroke(pen)
  let ink = if s.paint == auto { black } else { s.paint }
  let thick = if s.thickness == auto { 1pt } else { s.thickness }
  let dash = if shape == "whisper" { "dashed" } else if s.dash == auto { none } else { s.dash }
  let (cx, cy) = (w / 2, h / 2)
  let (a, b) = (w / 2, h / 2)
  // Shapes as functions of their fill and stroke, drawn twice: a thick
  // stroke under every shape, then the fills over it, so only the outer
  // edge of their union shows.
  let shapes = ()
  if shape == "caption" {
    shapes.push((f, st) => place(top + left, rect(width: w, height: h, fill: f, stroke: st)))
  } else if shape == "shout" {
    let n = 14
    let pts = range(2 * n).map(i => {
      let t = 180deg * i / n
      let r = if calc.even(i) { 1.3 } else { 1.02 }
      (cx + a * r * calc.cos(t), cy + b * r * calc.sin(t))
    })
    shapes.push((f, st) => place(top + left, polygon(fill: f, stroke: st, ..pts)))
  } else {
    shapes.push((f, st) => place(top + left, ellipse(width: w, height: h, fill: f, stroke: st)))
    if shape == "thought" {
      let r = calc.min(w, h) * 0.17 + 2pt
      let (af, bf) = (a / 1pt, b / 1pt)
      let per = calc.pi * (3 * (af + bf) - calc.sqrt((3 * af + bf) * (af + 3 * bf))) * 1pt
      let n = calc.max(8, int(calc.round(per / (r * 1.45))))
      for i in range(n) {
        let t = 360deg * i / n
        let (px, py) = (cx + a * calc.cos(t), cy + b * calc.sin(t))
        shapes.push((f, st) => place(top + left, dx: px - r, dy: py - r, circle(radius: r, fill: f, stroke: st)))
      }
    }
  }
  let bubbles = ()
  if tail != none and shape != "caption" {
    let (tx, ty) = (tail.at(0) + w * fx, tail.at(1) + h * fy)
    let (dx, dy) = (tx - cx, ty - cy)
    let len = calc.sqrt(calc.pow(dx / 1pt, 2) + calc.pow(dy / 1pt, 2)) * 1pt
    if len > 1pt {
      let (ux, uy) = (dx / len, dy / len)
      // Where the line to the tip leaves the ellipse.
      let edge = 1 / calc.sqrt(calc.pow(ux * 1pt / a, 2) + calc.pow(uy * 1pt / b, 2))
      if shape == "thought" {
        let m = calc.min(w, h)
        for (t, r) in ((0.3, 0.1), (0.62, 0.065), (0.9, 0.04)) {
          let d = edge * 1pt + (len - edge * 1pt) * t
          bubbles.push((cx + ux * d - m * r, cy + uy * d - m * r, m * r))
        }
      } else {
        let base = calc.min(w, h) * 0.3
        let from = edge * 1pt * 0.55
        let (bx, by) = (cx + ux * from, cy + uy * from)
        let (nx, ny) = (-uy * base / 2, ux * base / 2)
        // A slight bend, the same way on both sides.
        let bend = (len - from) * 0.12
        let mid(px, py) = ((px + tx) / 2 + uy * bend, (py + ty) / 2 - ux * bend)
        let p1 = (bx + nx, by + ny)
        let p2 = (bx - nx, by - ny)
        shapes.push((f, st) => place(top + left, curve(fill: f, stroke: st,
          curve.move(p1),
          curve.quad(mid(..p1), (tx, ty)),
          curve.quad(mid(..p2), p2),
          curve.close(),
        )))
      }
    }
  }
  let under = (paint: ink, thickness: 2 * thick, dash: dash, join: "round")
  let over = (paint: ink, thickness: thick, dash: dash)
  box(width: w, height: h, {
    for sh in shapes { sh(fill, under) }
    for sh in shapes { sh(fill, none) }
    for (x, y, r) in bubbles { place(top + left, dx: x, dy: y, circle(radius: r, fill: fill, stroke: over)) }
    place(center + horizon, block(width: tw, words))
  })
}
