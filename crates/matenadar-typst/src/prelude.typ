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

#let mt-wrap(st, body) = {
  let w = st.at("wrap", default: none)
  if w == none { body } else { w(body) }
}

// Text over a halo: the same text stroked in the halo's colour, behind it.
#let mt-halo(s, body) = box({
  place(top + left, text(stroke: (paint: s.paint, thickness: s.thickness * 2, join: "round"), fill: s.paint, body))
  body
})

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

// A heading: a paragraph in its style that is also a Typst heading (for the
// PDF's bookmarks) and leaves a <mt-h> marker for contents and running heads.
#let mt-head(level, outlined, title, st, k: 1, body) = {
  show heading: it => mt-par(st, k: k, first: true, [#metadata((level: level, title: title, outlined: outlined)) <mt-h>#it.body])
  heading(level: level, outlined: outlined, bookmarked: outlined, body)
}

#let mt-field(name) = {
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
  } else if name == "section.title" {
    info.at("title", default: "")
  } else if name.starts-with("section.") {
    info.at("vars", default: (:)).at(name.slice(8), default: "")
  } else { "" }
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

#let mt-rot(angle, body) = if angle == 0deg { body } else { rotate(angle, origin: center + horizon, reflow: false, body) }

// Content set at scale k until it fits `h`, down to `kmin`.
#let mt-shrink(w, h, kmin, make) = context {
  let k = 1.0
  while k > kmin and measure(block(width: w, make(k))).height > h { k = k - 0.02 }
  make(k)
}

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
