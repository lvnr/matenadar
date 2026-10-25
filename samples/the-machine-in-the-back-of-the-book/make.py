#!/usr/bin/env python3
"""Build doc.json: five pages of The Machine in the Back of the Book, rebuilt in Matenadar.

Reads only the book (through the `book` symlink): manuscript Markdown, the cost-review and
cartridge-note documents, the assembled cartridge bytes, the prepared art PNGs and the fonts.

    python3 make.py && python3 ../../scripts/review.py .

Pages (original → rebuilt):
     9 → 1  chapter 1 opener, section on master "chapter"
    14 → 2  chapter 2, third page (verso), section on master "flow", tight pour (t2)
    27 → 3  Castellan cost review, a hand-laid page
    29 → 5  chapter 7, second page (recto), section on master "flow"
    48 → 6  cartridge 1 label page, hand-laid (verso)
(rebuilt page 4 is the blank left so that p. 29 lands on a recto)
"""
from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
BOOK = HERE / "book"
MS = BOOK / "manuscript"
ART = "book/build/art"

# The engine's `list` and `code` blocks fail to compile in the current build (see GAPS.md):
# lists are set as two-column tables and code as boxed paragraphs until they work.
USE_LIST_BLOCK = False

# ---------------------------------------------------------------- geometry (pt; trim coordinates)
IN = 72.0
TRIM_W, TRIM_H = 7 * IN, 10 * IN
COL_W, NOTE_W, NOTE_GAP = 4.1 * IN, 1.45 * IN, 0.2 * IN
INSIDE, TOP, COL_H = 0.75 * IN, 0.6 * IN, 8.75 * IN
OUTSIDE = TRIM_W - INSIDE - COL_W            # the notes live in here
BOTTOM = TRIM_H - TOP - COL_H

C = {
    "ink": "#1b1b1a", "teal": "#1d5f73", "red": "#c23b2e", "gray": "#5d6670", "ball": "#24399c",
    "rule": "#cfcabb", "tint": "#f1f5f6", "kicker": "#bcd6dd", "strike-ink": "#3a3a3a",
    "hair": "#d7dfe2", "sheet": "#fbfaf6", "sheet-edge": "#d8d4c8", "type": "#232323",
    "cart": "#222324", "rib": "#3a3b3d", "cart-foot": "#1a1a1b", "label": "#f4f0e6",
    "life": "#f3dcd5", "brass-ink": "#5a4210",
}

SERIF, SANS, MONO = "IBM Plex Serif", "IBM Plex Sans", "IBM Plex Mono"
COND = {"font": SANS, "stretch": 75}          # IBM Plex Sans Condensed is the 75 % width of the family
CAVEAT = {"font": "Caveat", "weight": 500}   # variable font, wght axis driven by `weight`
NANUM = {"font": "Nanum Pen"}


def bw(pt: float) -> float:
    """A CSS border width as Chrome lays it out and prints it: snapped to whole CSS pixels (0.75 pt),
    never under one. 1.1 pt and 0.45 pt rules both print 0.75 pt; 2 pt prints 1.5 pt."""
    import math
    px = pt / 0.75
    return (1 if px < 1 else math.floor(px)) * 0.75


# ---------------------------------------------------------------- manuscript → runs

def curl(text: str, prev: str = "") -> str:
    """The book's own quote curling (scripts/blocks.py curl)."""
    out = []
    for ch in text:
        before = out[-1] if out else prev
        opens = before == "" or before.isspace() or before in "([{—–-“‘/"
        if ch == "'":
            ch = "‘" if opens else "’"
        elif ch == '"':
            ch = "“" if opens else "”"
        out.append(ch)
    return "".join(out)


def runs(s: str, curly: bool = True) -> list:
    """Markdown inline (`code`, **bold**, *italic*) to Matenadar runs."""
    out: list = []
    prev = ""
    for i, part in enumerate(re.split(r"`([^`]+)`", s)):
        if i % 2:
            out.append({"text": part, "span": "code"})
            prev = "x"
            continue
        t = curl(part, prev) if curly else part
        if part:
            prev = part[-1]
        for j, bit in enumerate(re.split(r"\*\*(.+?)\*\*", t)):
            if not bit:
                continue
            if j % 2:
                out.append({"text": bit, "strong": True})
            else:
                for k, b2 in enumerate(re.split(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", bit)):
                    if b2:
                        out.append({"text": b2, "em": True} if k % 2 else b2)
    return out


def lines(path: str, a: int, b: int) -> list[str]:
    """Manuscript lines a..b (1-based, inclusive)."""
    return (MS / path).read_text(encoding="utf-8").split("\n")[a - 1:b]


def line(path: str, n: int) -> str:
    return lines(path, n, n)[0]


def item_text(s: str) -> str:
    return re.sub(r"^(\d+\.|-)\s+", "", s)


def dash_runs(text: str) -> list:
    """Garrow's pen has no em dash: the book borrows Caveat's (span .dash)."""
    out = []
    for i, part in enumerate(text.split("—")):
        if i:
            out.append({"text": "—", "span": "dash"})
        if part:
            out.append(part)
    return out


# ---------------------------------------------------------------- blocks

def p(text_or_runs, style: str | None = None, **kw) -> dict:
    b = {"type": "p"}
    if style:
        b["style"] = style
    b["content"] = runs(text_or_runs) if isinstance(text_or_runs, str) else text_or_runs
    b.update(kw)
    return b


def h2(text: str, style: str = "h2") -> dict:
    return {"type": "h", "level": 2, "style": style, "content": runs(text)}


def margin(*items: tuple[str, str], dy: float = -2.2) -> dict:
    """Margin items beside the block before them: Ani's ballpoint ("note") or Garrow's red pen ("red").
    Items that follow one block are stacked 7 pt apart, as the book's paginator does; the engine
    anchors a second margin block to the next block instead, so they share one margin block here."""
    blocks = []
    for i, (kind, text) in enumerate(items):
        content = dash_runs(curl(text)) if kind == "red" else runs(text)
        b = {"type": "p", "style": kind, "content": content}
        if i:
            b["set"] = {"space_before": 7}
        blocks.append(b)
    return {"type": "margin", "dy": dy, "blocks": blocks}


def listing(items: list[str], ordered: bool, start: int = 1, item_style: str = "li",
            indent: float = 0.21 * IN, gap: float = 3.5, after: float = 8, lh: str = "14pt",
            item_size: float = 9.6) -> dict:
    """A list. Markers hang in the indent, right-aligned, as CSS outside markers do."""
    if USE_LIST_BLOCK:
        return {"type": "list", "items": [[p(t, item_style)] for t in items], "ordered": ordered,
                "start": start, "marker": "1." if ordered else "•", "indent": indent, "style": item_style,
                "spacing": gap, "space_after": after}
    rows = []
    for i, t in enumerate(items):
        # Chrome draws a disc marker as a circle about a third of an em across, 9.4 pt clear of the text
        mark = [f"{start + i}.\u2002"] if ordered else [{"text": "•\u2002", "baseline": "0.05em"}]
        mset = {"line_height": lh} if ordered else {"line_height": lh, "size": round(1.33 * item_size, 2)}
        rows.append([{"blocks": [p(mark, "ol-mark" if ordered else "ul-mark", set=mset)]},
                     {"blocks": [p(t, item_style)]}])
    return {"type": "table", "columns": [indent, "1fr"], "rows": rows, "stroke": "none", "inset": 0,
            "row_gap": gap, "style": item_style, "space_before": 0, "space_after": after}


def code_block(text_lines: list[str], style: str = "code") -> dict:
    """Ani's notation: mono lines, spaces kept, a 1.6 pt teal rule on the left."""
    paras = [p([ln.replace(" ", "\u00a0") or "\u00a0"], style) for ln in text_lines]
    return {"type": "box", "stroke": {"left": {"width": bw(1.6), "color": "teal"}},
            "inset": {"left": bw(1.6) + 0.13 * IN, "top": 1, "bottom": 1, "right": 0},
            "blocks": paras, "space_before": 2, "space_after": 10}


def hrule(w, stroke: dict, h: float) -> dict:
    """A horizontal rule as a block of its own height."""
    return {"type": "canvas", "w": w, "h": h, "space_before": 0, "space_after": 0, "elements": [
        {"type": "line", "x1": 0, "y1": h / 2, "x2": w if not isinstance(w, str) else "100%", "y2": h / 2,
         "stroke": stroke}]}


def rules_under(stroke) -> dict:
    """Table cell borders on the bottom side only."""
    return {"bottom": stroke, "top": "none", "left": "none", "right": "none"}


def tbl(header: list[str], rows: list[list[str]], widths: list, style: str = "td") -> dict:
    """.tbl: sans 8.4/11, header 6.9/9 caps over a 1.1 pt ink rule, 0.45 pt rules under rows.
    Header and body cells have different padding and rules, and a table has one inset and one stroke,
    so the header is a table of its own over the body's. A CSS border adds its width to the row; a cell
    stroke sits centred on the grid line, so half of each rule's width is added to the insets."""
    head = {"type": "table", "columns": widths, "rows": [[{"blocks": [p(runs(h), "th")]} for h in header]],
            "stroke": rules_under({"width": bw(1.1), "color": "ink"}), "inset": {"top": 0, "right": 6, "bottom": 2.5 + bw(1.1) / 2, "left": 0},
            "valign": "bottom", "style": "th", "space_before": 0, "space_after": 0}
    body_rows = []
    for r in rows:
        cells = [[c.replace(" ", "\u00a0")] if j == 0 else runs(c) for j, c in enumerate(r)]  # nowrap
        body_rows.append([{"blocks": [p(c, style)]} for c in cells])
    body = {"type": "table", "columns": widths, "rows": body_rows, "stroke": rules_under({"width": bw(0.45), "color": "rule"}),
            "inset": {"top": 2.6 + bw(0.45) / 2, "right": 6, "bottom": 2.6 + bw(0.45) / 2, "left": 0}, "style": style, "space_before": 0,
            "space_after": 0}
    return {"type": "box", "blocks": [head, body], "space_before": 2, "space_after": 10}


# ---------------------------------------------------------------- the design

def styles() -> dict:
    body = {"space_after": 7}
    s = {
        "body": body,
        "body-t2": {"line_height": "13.6pt", "space_after": 4.5},
        "h2": {**COND, "weight": 600, "size": 11.6, "line_height": "13.5pt", "tracking": "0.005em",
               "space_before": 15, "space_after": 6, "keep_with_next": True, "hyphenate": False,
               "rule_above": {"stroke": {"width": 2.2, "color": "teal"}, "width": 0.22 * IN, "gap": 5 + 2.2}},
        "li": {"space_after": 0},
        "li-t2": {"line_height": "13.6pt", "space_after": 0},
        "ol-mark": {**COND, "weight": 600, "size": 8.6, "color": "teal", "align": "right", "hyphenate": False,
                    "space_after": 0},
        "ul-mark": {"color": "teal", "align": "right", "hyphenate": False, "space_after": 0},
        "note": {**CAVEAT, "size": 12.6, "line_height": "13.2pt", "color": "ball", "hyphenate": False},
        "red": {**NANUM, "size": 14.2, "line_height": "13pt", "color": "red", "hyphenate": False},
        "th": {**COND, "weight": 600, "size": 6.9, "line_height": "9pt", "tracking": "0.08em", "case": "upper",
               "space_before": 0, "space_after": 0, "hyphenate": False},
        "td": {**COND, "size": 8.4, "line_height": "11pt", "space_before": 0, "space_after": 0, "hyphenate": False},
        "code": {"font": MONO, "size": 8.4, "line_height": "12.2pt", "features": ["zero"], "hyphenate": False,
                 "space_before": 0, "space_after": 0},
        "listing": {"font": MONO, "size": 8.4, "line_height": "12pt", "features": ["zero"], "hyphenate": False,
                    "space_before": 0, "space_after": 0},
        "lhead": {**COND, "weight": 600, "size": 6.9, "line_height": "9pt", "tracking": "0.12em", "case": "upper",
                  "color": "gray", "space_before": 6, "space_after": 4, "keep_with_next": True},
        "bt": {**COND, "weight": 600, "size": 7, "line_height": "7pt", "tracking": "0.15em", "case": "upper",
               "color": "teal", "space_after": 6},
        "bli": {"size": 9.1, "line_height": "12.8pt", "space_after": 0},
        "cap": {**COND, "size": 7.6, "line_height": "10pt", "color": "gray", "space_before": 5, "space_after": 6},
        "ql": {"font": MONO, "weight": 500, "size": 6.4, "line_height": "8pt", "tracking": "0.02em",
               "color": "gray", "space_before": 2},
        "runhead": {**COND, "weight": 600, "size": 6.6, "line_height": 1, "tracking": "0.13em", "case": "upper",
                    "color": "gray", "hyphenate": False},
        "folio": {"font": MONO, "weight": 600, "size": 7.6, "line_height": 1, "tracking": "0.02em"},
        # the opener band
        "kicker": {**COND, "weight": 600, "size": 7, "line_height": 1, "tracking": "0.16em", "case": "upper",
                   "color": "kicker"},
        "opener-num": {**COND, "weight": 700, "size": 66, "line_height": 0.8, "tracking": "-0.01em",
                       "color": "#ffffff"},
        "opener-title": {**COND, "weight": 600, "size": 21, "line_height": "23pt", "color": "#ffffff",
                         "hyphenate": False},
        # Castellan documents (typewriter)
        "lh-name": {"font": SERIF, "weight": 600, "size": 13.5, "line_height": 1, "tracking": "0.16em",
                    "color": "#4a5059"},
        "lh-div": {**COND, "size": 7.4, "line_height": 1.3, "tracking": "0.1em", "case": "upper", "color": "gray"},
        "memo-t": {"font": "Courier Prime", "weight": 700, "size": 9.5, "line_height": 1, "tracking": "0.3em",
                   "case": "upper", "color": "#2a2a2a", "space_before": 14, "space_after": 10},
        "ty": {"font": "Courier Prime", "size": 9, "line_height": "12.6pt", "color": "type", "hyphenate": False,
               "space_after": 10},
        "ty-meta": {"inherits": "ty", "space_after": 0},
        "ty-meta-l": {"inherits": "ty-meta", "weight": 700},
        "ty-items": {"inherits": "ty", "size": 8.2, "line_height": "11pt", "space_after": 0},
        "ty-items-h": {"inherits": "ty-items", "weight": 700},
        "lrg": {**NANUM, "size": 13.6, "line_height": "12.6pt", "color": "red", "space_after": 7,
                "hyphenate": False},
        "ani-review": {**CAVEAT, "size": 15.5, "line_height": "17pt", "color": "ball", "hyphenate": False},
        # cartridge label
        "lb-mark": {"font": "Archivo", "weight": 900, "stretch": 125, "size": 13, "line_height": 1,
                    "tracking": "0.01em", "color": "#ffffff"},
        "lb-no": {"font": MONO, "weight": 600, "size": 7.4, "line_height": 1, "tracking": "0.14em",
                  "color": "#cfe3e8", "align": "right", "margin_right": 0.14 * 7.4},
        "lb-title": {"font": "Archivo", "weight": 800, "stretch": 68, "size": 34, "line_height": 0.95,
                     "case": "upper", "color": "#ffffff", "hyphenate": False},
        "lb-by": {**COND, "size": 7.6, "line_height": 1, "tracking": "0.04em", "color": "#d7e7eb"},
    }
    return s


def runhead(when: str) -> dict:
    """Recto: chapter number in teal, a 0.16 in gap, the chapter title; verso: the manual's title."""
    gap = {"text": "\u2003\u2002\u2009"}                 # the 0.16 in separator, tracked like the head
    if when == "recto":
        content = [{"field": "section.num", "color": "teal"}, gap, {"field": "section.title"}]
        return {"type": "text", "x": 0, "y": 0.34 * IN, "w": 6.5 * IN, "style": "runhead", "content": content,
                # Chrome keeps the tracking after the last letter inside a right-aligned line
                "set": {"align": "right", "margin_right": 0.13 * 6.6}, "when": "recto"}
    content = ["Magpie Service Manual", gap, "Draft 7"]
    return {"type": "text", "x": 0.5 * IN, "y": 0.34 * IN, "w": 4 * IN, "style": "runhead", "content": content,
            "when": "verso"}


def folio(when: str) -> dict:
    y = TRIM_H - 0.33 * IN - 7.6
    if when == "recto":
        return {"type": "text", "x": 0, "y": y, "w": 6.5 * IN, "style": "folio", "content": [{"field": "page"}],
                "set": {"align": "right", "margin_right": 0.02 * 7.6}, "when": "recto"}
    return {"type": "text", "x": 0.5 * IN, "y": y, "w": 2 * IN, "style": "folio", "content": [{"field": "page"}],
            "when": "verso"}


def masters() -> dict:
    page = {"width": "7in", "height": "10in", "bleed": "0.125in"}
    flow = {
        **page,
        "margins": {"top": TOP, "bottom": BOTTOM, "inside": INSIDE, "outside": OUTSIDE},
        "style": "body",
        "notes": {"width": NOTE_W, "gap": NOTE_GAP, "side": "outside", "style": "note", "spacing": 7},
        "foreground": [runhead("recto"), runhead("verso"), folio("recto"), folio("verso")],
    }
    # A chapter's first page: the teal band from the head with kicker, number and title; no running head.
    band_h = 4.1 * IN
    chapter = {
        "inherits": "flow",
        "background": [
            {"type": "rect", "x": -9, "y": -9, "w": TRIM_W + 18, "h": band_h, "fill": "teal", "when": "first",
             "decorative": True},
            {"type": "text", "x": INSIDE, "y": 0.6 * IN - 9, "w": 3 * IN, "style": "kicker", "text": "Chapter",
             "when": "first"},
            {"type": "text", "x": INSIDE, "y": 0.86 * IN - 9, "w": 3 * IN, "style": "opener-num",
             "content": [{"field": "section.num"}], "when": "first"},
            {"type": "text", "x": INSIDE, "y": 1.95 * IN - 9, "w": 2.05 * IN, "style": "opener-title",
             "content": [{"field": "section.title"}], "when": "first"},
        ],
        "foreground": [
            {**runhead("recto"), "when": "recto not-first"},
            {**runhead("verso"), "when": "verso not-first"},
            folio("recto"), folio("verso"),
        ],
    }
    fixed = {**page, "margins": {"top": TOP, "bottom": BOTTOM, "inside": INSIDE, "outside": OUTSIDE}}
    return {"flow": flow, "chapter": chapter, "fixed": fixed}


# ---------------------------------------------------------------- p. 9: chapter 1 opener

def opener_art(n: int) -> dict:
    """The band's art stands on the band's foot, 0.5 in from the outside edge (contain, bottom-centred).
    Drawn from the flow: a canvas as tall as the band leaves the text column starting at trim 4.3 in."""
    band_bottom = 4.1 * IN - 9                 # trim y of the band's foot
    art_w, art_h = 3.6 * IN, 2.4 * IN
    iw, ih = 1186, 790                          # build/art/chNN.png
    h = art_w * ih / iw                         # contained: full width, bottom-aligned
    x = TRIM_W + 9 - 0.5 * IN - art_w           # recto: right edge 0.5 in inside the bleed edge
    y = band_bottom - 0.22 * IN - h
    top_text = 4.3 * IN
    return {"type": "canvas", "h": top_text - TOP, "space_before": 0, "space_after": 0, "elements": [
        {"type": "image", "src": f"{ART}/ch{n:02d}.png", "x": x - INSIDE, "y": y - TOP, "w": art_w, "h": h,
         "fit": "contain", "decorative": True}]}


def page9() -> dict:
    ch = "part1/01-before-you-begin.md"
    num, title = [x.strip() for x in line(ch, 1)[2:].split("|")]
    flow = [
        opener_art(int(num)),
        p(line(ch, 3)),
        h2(line(ch, 5)[3:]),
        p(line(ch, 7)),
        p(line(ch, 9)),
        margin(("red", line(ch, 12)), ("note", line(ch, 16))),
        h2(line(ch, 19)[3:]),
        listing([item_text(line(ch, 21))], ordered=False),
    ]
    return {"type": "section", "id": "ch01", "master": "chapter", "start": "recto", "numbering": "1",
            "page_from": 9, "title": title, "vars": {"num": num}, "flow": flow}


# ---------------------------------------------------------------- p. 14: chapter 2, third page

def underside_figure() -> dict:
    """The underside PNG with Talin's plate engraved on it: two lines of Plex Mono set flat in a
    250 x 60 box and mapped onto the plate's parallelogram (book: scripts/pages.py underside())."""
    iw, ih = 1246, 831
    w = COL_W
    s = w / iw                                   # pt per image pixel
    h = ih * s
    tl, tr, bl = (488, 285), (731, 345), (460, 338)
    BW, BH = 250.0, 60.0
    a, b = (tr[0] - tl[0]) / BW, (tr[1] - tl[1]) / BW      # image of the box's x axis
    c, d = (bl[0] - tl[0]) / BH, (bl[1] - tl[1]) / BH      # image of its y axis
    import math
    theta = math.degrees(math.atan2(b, a))
    sx = math.hypot(a, b)
    ct, st = math.cos(math.radians(theta)), math.sin(math.radians(theta))
    k_x, sy = ct * c + st * d, -st * c + ct * d             # R(-theta) applied to the y axis
    skew = math.degrees(math.atan2(k_x, sy))
    # the box centre, in pt
    cx = (tl[0] + a * BW / 2 + c * BH / 2) * s
    cy = (tl[1] + b * BW / 2 + d * BH / 2) * s
    bw, bh = BW * s * sx, BH * s * sy
    size = 17.5 * s
    eng = {"font": MONO, "weight": 600, "size": size, "color": "brass-ink", "align": "center", "line_height": 1}
    # baselines at 25 and 47 box units; x/y of a text frame is the top of its first line
    asc = 1.025 * size                          # Plex Mono hhea ascender; line height 1 → no half-leading shift
    half = (size - 1.3 * size) / 2
    y1 = 25 * s * sy - asc - half
    y2 = 47 * s * sy - asc - half
    group = {"type": "group", "x": cx - bw / 2, "y": cy - bh / 2, "w": bw, "h": bh, "rotate": round(theta, 3),
             "skew": round(-skew, 3), "children": [
                 {"type": "text", "x": 0, "y": y1, "w": bw, "text": "MAGPIE No. 1",
                  "set": {**eng, "tracking": f"{2.5 / 17.5:.4f}em"}},
                 {"type": "text", "x": 0, "y": y2, "w": bw, "text": "TALIN KAZARIAN",
                  "set": {**eng, "tracking": f"{0.6 / 17.5:.4f}em"}},
             ]}
    return {"type": "canvas", "h": h, "space_before": 0, "space_after": 0, "elements": [
        {"type": "image", "src": f"{ART}/underside.png", "x": 0, "y": 0, "w": w, "h": h, "fit": "stretch"},
        group]}


def page14() -> dict:
    ch = "part1/02-at-a-glance.md"
    num, title = [x.strip() for x in line(ch, 1)[2:].split("|")]
    strike = " ".join(lines(ch, 37, 37))
    fig_id, cap = [x.strip() for x in line(ch, 40)[len(":::figure"):].split("|")]
    flow = [
        listing([item_text(t) for t in lines(ch, 22, 24)], ordered=True, start=8, item_style="li-t2", lh="13.6pt"),
        # the list starts at the column top, and a note is never set above the column
        margin(("red", line(ch, 27)), ("note", line(ch, 31)), dy=0),
        h2(line(ch, 34)[3:], "h2-t2"),
        p([{"text": curl(strike), "mark": {"kind": "strike", "stroke": {"width": 0.94, "color": "red", "cap": "round"},
                                            "extent": 1.5, "offset": -2.6}}],
          "body-t2", set={"color": "strike-ink"}),
        {"type": "box", "space_before": 4, "space_after": 6,
         "blocks": [underside_figure(), p(cap, "cap", set={"space_after": 0})]},
        margin(("red", line(ch, 43)), ("note", line(ch, 47))),
        h2(line(ch, 50)[3:], "h2-t2"),
        p(line(ch, 52), "body-t2", set={"keep_with_next": True}),
        listing([item_text(t) for t in lines(ch, 54, 57)], ordered=True, start=1, item_style="li-t2", lh="13.6pt"),
        p(line(ch, 59), "body-t2"),
    ]
    return {"type": "section", "id": "ch02", "master": "flow", "start": "next", "numbering": "1",
            "page_from": 14, "title": title, "vars": {"num": num}, "style": "body-t2", "flow": flow}


# ---------------------------------------------------------------- p. 27: the cost review

def doc_fields(text: str) -> tuple[dict, list[str]]:
    fields, body = {}, []
    for ln in text.split("\n"):
        m = re.match(r"^(letterhead|date|to|from|copy|subject|stamp|redpen \d+|bluepen): (.*)$", ln)
        if m:
            fields.setdefault(m.group(1), []).append(m.group(2))
        elif not ln.startswith("# "):
            body.append(ln)
    paras = [x.strip() for x in "\n".join(body).split("\n\n") if x.strip()]
    return fields, paras


def page27() -> dict:
    f, paras = doc_fields((MS / "documents/cost-review.md").read_text(encoding="utf-8"))
    SW, SH = 5.75 * IN, 8.55 * IN
    PAD_T, PAD_X, PAD_B = 0.55 * IN, 0.6 * IN, 0.4 * IN
    B = bw(0.5)                                   # the sheet's border; content sits inside it
    CW = SW - 2 * B - 2 * PAD_X                   # 327.6 pt

    lh_h = 33.12 + 8 + bw(1.2)
    letterhead = {"type": "canvas", "h": lh_h, "elements": [
        {"type": "path", "x": 0, "y": 0, "w": 0.42 * IN, "h": 0.46 * IN, "viewbox": [0, 0, 42, 46], "fill": "gray",
         "d": "M3 44 V12 H9 V6 H15 V12 H18 V6 H24 V12 H27 V6 H33 V12 H39 V44 Z M17 44 V33 C17 29 25 29 25 33 V44 Z"},
        {"type": "text", "x": 0.42 * IN + 10, "y": 3.5, "w": CW - 0.42 * IN - 10, "style": "lh-name",
         "text": "CASTELLAN CORPORATION"},
        {"type": "text", "x": 0.42 * IN + 10, "y": 3.5 + 13.5 + 3, "w": CW - 0.42 * IN - 10, "style": "lh-div",
         "text": "Consumer Products Engineering"},
        {"type": "line", "x1": 0, "y1": lh_h - bw(1.2) / 2, "x2": CW, "y2": lh_h - bw(1.2) / 2,
         "stroke": {"width": bw(1.2), "color": "gray"}},
    ]}
    meta = {"type": "table", "columns": [51.2, "1fr"], "stroke": "none", "style": "ty-meta",
            "inset": {"top": 0, "right": 8, "bottom": 1, "left": 0}, "space_before": 0, "space_after": 6,
            "rows": [[{"blocks": [p([f"{k}:"], "ty-meta-l")]}, {"blocks": [p([f[k.lower()][0]], "ty-meta")]}]
                     for k in ("TO", "FROM", "COPY", "DATE", "SUBJECT")]}
    blocks = [{**letterhead, "space_before": 0, "space_after": 0}, p(["Engineering Cost Review"], "memo-t"),
              meta,
              {**hrule(CW, {"width": bw(1), "color": "#9a9a95"}, bw(1)), "space_before": 4, "space_after": 12}]
    widths4 = [0.27 * IN + 5, 1.32 * IN + 5, 1.12 * IN + 5, 0.58 * IN + 5]
    for para in paras:
        if para.startswith("|"):
            rows = [[c.strip() for c in r.strip().strip("|").split("|")] for r in para.split("\n")]
            head, body = rows[0], rows[2:]
            # Header cells are bottom-aligned ("Unit saving" wraps in its 0.58 in column) over a 1 pt rule;
            # rows have a dotted rule under them. Only the four printed columns are ruled, not the pen column.
            head_t = {"type": "table", "columns": widths4, "rows": [[{"blocks": [p([t], "ty-items-h")]} for t in head]],
                      "stroke": rules_under({"width": bw(1), "color": "#555555"}), "valign": "bottom",
                      "inset": {"top": 0, "right": 5, "bottom": 2 + bw(1) / 2, "left": 0}, "style": "ty-items-h",
                      "space_before": 0, "space_after": 0}
            body_t = {"type": "table", "columns": widths4, "rows": [[{"blocks": [p([c], "ty-items")]} for c in r] for r in body],
                      "stroke": rules_under({"width": bw(0.4), "color": "#aaaaaa", "dash": "dotted"}),
                      "inset": {"top": 2.2 + bw(0.4) / 2, "right": 5, "bottom": 2.2 + bw(0.4) / 2, "left": 0},
                      "style": "ty-items",
                      "space_before": 0, "space_after": 0}
            blocks.append({"type": "box", "blocks": [head_t, body_t], "space_before": 4, "space_after": 10})
        else:
            blocks.append(p([para], "ty"))

    # Garrow's red pen, beside rows 7 to 10 (in the pen column, 0.14 in in)
    red = {int(k.split()[1]): v[0] for k, v in f.items() if k.startswith("redpen")}
    pen = [p([{"text": f"{n}.\u00a0"}] + dash_runs(t), "lrg") for n, t in sorted(red.items())]
    pen_x = B + PAD_X + sum([0.27 * IN + 5, 1.32 * IN + 5, 1.12 * IN + 5, 0.58 * IN + 5]) + 0.14 * IN
    pen_y = 405.3                                  # top of row 7's pen cell, less 1 pt (measured layout)
    blue = f["bluepen"][0]
    blue_h = 2 * 17
    children = [
        {"type": "text", "x": B + PAD_X, "y": B + PAD_T, "w": CW, "blocks": blocks, "style": "ty"},
        {"type": "text", "x": pen_x, "y": pen_y, "w": 1.12 * IN, "blocks": pen},
        {"type": "text", "x": B + PAD_X, "y": SH - B - 0.5 * IN - blue_h, "w": 4.3 * IN, "h": blue_h,
         "style": "ani-review", "text": blue, "rotate": -1.5},
    ]
    sheet = {"type": "group", "x": 0.62 * IN, "y": 0.75 * IN, "w": SW, "h": SH, "rotate": -0.5,
             "fill": "sheet", "children": [
                 # a CSS border lies inside the box
                 {"type": "rect", "x": B / 2, "y": B / 2, "w": SW - B, "h": SH - B,
                  "stroke": {"width": B, "color": "sheet-edge"}}] + children}
    return {"type": "page", "id": "cost-review", "master": "fixed", "elements": [sheet]}


# ---------------------------------------------------------------- p. 29: chapter 7, second page

def records(image: bytes, width: int = 16, start: int = 0):
    """machine/listing.py records(): address, bytes, Fletcher-style check (machine/sim.py check_of)."""
    def check_of(nums):
        total = check = 0
        for n in nums:
            total = (total + n) % 255
            check = (check + total) % 255
        return check
    out = []
    for off in range(0, len(image), width):
        chunk = image[off:off + width]
        addr = (start + off) & 0xFFFF
        out.append((addr, chunk, check_of([addr >> 8, addr & 255, *chunk])))
    return out


def listing_group(image: bytes, group: int, per: int = 8) -> list:
    recs = records(image, 16)
    grp = recs[group * per:(group + 1) * per]
    paras = []
    for i, (a, c, k) in enumerate(grp):
        h1 = " ".join(f"{b:02X}" for b in c[:8])
        h2_ = " ".join(f"{b:02X}" for b in c[8:])
        body = (h1 + ("  " + h2_ if h2_ else "")).ljust(48)
        ln = p([{"text": f"{a:04X}:", "color": "teal", "weight": 500},
                (" " + body + " ").replace(" ", "\u00a0"),
                {"text": f"{k:02X}", "color": "gray"}], "listing")
        if (i + 1) % 4 == 0 and i + 1 < len(grp):
            # a 0.4 pt hairline under every fourth record; the border adds its width to the line
            ln["set"] = {"rule_below": {"stroke": {"width": bw(0.4), "color": "hair"}, "gap": 0}, "space_after": bw(0.4)}
        paras.append(ln)
    start = grp[0][0]
    end = grp[-1][0] + len(grp[-1][1]) - 1
    seg = image[start:end + 1]
    payload = " ".join(f"{a:04X}: {c.hex().upper()} {k:02X}" for a, c, k in records(seg, 64, start))
    n = 57                                         # version 10 at ECC M, alphanumeric (the book's segno)
    size = round((n + 8) * 0.0185, 3) * IN
    box = {"type": "box", "blocks": paras, "space_before": 0, "space_after": 8}
    qr = {"type": "margin", "dy": 0, "blocks": [
        {"type": "canvas", "w": size, "h": size, "space_before": 0, "space_after": 0, "elements": [
            {"type": "qr", "x": 0, "y": 0, "w": size, "data": payload, "ecc": "M", "quiet": 4, "color": "ink",
             "background": "#ffffff"}]},
        p([f"{start:04X}–{end:04X}"], "ql")]}
    return [box, qr]


def page29() -> dict:
    ch = "part1/07-arithmetic.md"
    num, title = [x.strip() for x in line(ch, 1)[2:].split("|")]
    t1 = [[c.strip() for c in r.strip().strip("|").split("|")] for r in lines(ch, 30, 36)]
    t2 = [[c.strip() for c in r.strip().strip("|").split("|")] for r in lines(ch, 44, 50)]
    build_title = line(ch, 54)[len(":::build"):].strip()
    build_items = [item_text(t) for t in lines(ch, 55, 57)]
    test = line(ch, 62)[len(":::test"):].strip()
    image = (BOOK / "machine/build" / f"{test}.bin").read_bytes()
    nrec = len(records(image, 16))
    # 5 pt padding below, plus the last item's 3 pt margin, which the padding keeps inside the box
    build = {"type": "box", "fill": "tint", "stroke": {"top": {"width": bw(2), "color": "teal"}},
             "inset": {"top": bw(2) + 8, "left": 10, "right": 10, "bottom": 5 + 3}, "space_before": 4, "space_after": 12,
             "blocks": [
                 p([f"Build it\u00a0", {"text": f"· {build_title}", "color": "ink"}], "bt"),
                 listing(build_items, ordered=False, item_style="bli", indent=0.17 * IN + 0.02 * IN, gap=3, after=0,
                         lh="12.8pt", item_size=9.1)]}
    lhead = p([{"text": "Listing", "color": "teal"},
               f"\u00a0\u00a0{len(image):,} bytes in {nrec} records\u00a0\u00a0·\u00a0\u00a0a QR code beside every eight"],
              "lhead")
    flow = [
        {**code_block(lines(ch, 22, 25)), "space_before": 0},
        h2(line(ch, 28)[3:]),
        tbl(t1[0], t1[2:], [50.3, 25.7, "1fr"]),
        p(line(ch, 38)),
        margin(("note", line(ch, 41))),
        tbl(t2[0], t2[2:], [68.7, 99.0, 68.7, "1fr"]),
        p(line(ch, 52)),
        build,
        h2(line(ch, 60)[3:]),
        lhead,
        *listing_group(image, 0),
    ]
    return {"type": "section", "id": "ch07", "master": "flow", "start": "recto", "numbering": "1",
            "page_from": 29, "title": title, "vars": {"num": num}, "flow": flow}


# ---------------------------------------------------------------- p. 48: cartridge 1 label

def page48() -> dict:
    head = line("part2/notes.md", 3)[3:]
    key, n, title, by = [x.strip() for x in head.split("|")]
    n = int(n)
    CW_, CH_ = 4.9 * IN, 7.1 * IN
    LW = 4.1 * IN
    band_y = 0.78 * IN + LW
    band_h = 1.48 * IN
    pad_t, pad_x = 0.12 * IN, 0.16 * IN
    ribs = [{"type": "rect", "x": 0.55 * IN + i * (CW_ - 1.1 * IN - 0.05 * IN) / 8, "y": 0.18 * IN,
             "w": 0.05 * IN, "h": 0.42 * IN, "fill": "rib", "radius": 0.02 * IN} for i in range(9)]
    label = {"type": "group", "x": 0.4 * IN, "y": 0.78 * IN, "w": LW, "h": LW + band_h, "children": [
        {"type": "rect", "x": 0, "y": 0, "w": LW, "h": LW + band_h, "fill": "label"},
        {"type": "image", "src": f"{ART}/label-{key}.png", "x": 0, "y": 0, "w": LW, "h": LW},
        {"type": "rect", "x": 0, "y": LW, "w": LW, "h": band_h, "fill": "teal"},
        {"type": "text", "x": pad_x, "y": LW + pad_t, "w": 2 * IN, "style": "lb-mark", "text": "MAGPIE"},
        # baseline-aligned with MAGPIE: same 13 pt line box, mono 7.4 pt set on its baseline
        {"type": "text", "x": LW - pad_x - 2 * IN, "y": LW + pad_t + 3.85, "w": 2 * IN, "style": "lb-no",
         "text": f"CARTRIDGE {n:02d}"},
        {"type": "text", "x": pad_x, "y": LW + pad_t + 13 + pad_t, "w": LW - 2 * pad_x, "style": "lb-title",
         "set": {"size": 30} if len(title) > 9 else {}, "text": title},
        {"type": "text", "x": pad_x, "y": LW + band_h - 0.13 * IN - 7.6, "w": LW - 2 * pad_x, "style": "lb-by",
         "content": runs(by)},
    ]}
    cart = {"type": "group", "x": 1.05 * IN, "y": 1.45 * IN, "w": CW_, "h": CH_, "children": [
        {"type": "rect", "x": 0, "y": 0, "w": CW_, "h": CH_, "fill": "cart",
         "radius": {"top_left": 0.16 * IN, "top_right": 0.16 * IN, "bottom_left": 0.1 * IN, "bottom_right": 0.1 * IN}},
        *ribs,
        label,
        {"type": "rect", "x": 0, "y": CH_ - 0.36 * IN, "w": CW_, "h": 0.36 * IN, "fill": "cart-foot",
         "radius": {"top_left": 0, "top_right": 0, "bottom_left": 0.1 * IN, "bottom_right": 0.1 * IN}},
    ]}
    return {"type": "page", "id": f"label-{key}", "master": "fixed", "fill": key, "elements": [cart]}


# ---------------------------------------------------------------- the document

def main() -> None:
    doc = {
        "matenadar": "1",
        "meta": {"title": "The Machine in the Back of the Book (sample pages)", "author": "Ani Kazarian",
                 "lang": "en-US"},
        "fonts": ["book/assets/fonts"],
        "design": {
            "colors": C,
            "fonts": {},
            "base": {"font": SERIF, "size": 9.6, "line_height": "14pt", "color": "ink", "lang": "en-US",
                     "hyphenate": True},
            "styles": styles(),
            "spans": {
                "code": {"font": MONO, "size": 8.5, "features": ["zero"]},
                "dash": {**CAVEAT},
            },
            "masters": masters(),
        },
        "content": [
            page9(),
            page14(),
            page27(),
            page29(),
            page48(),
        ],
    }
    doc["design"]["styles"]["h2-t2"] = {"inherits": "h2", "space_before": 9, "space_after": 4}
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
