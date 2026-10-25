#!/usr/bin/env python3
"""Build doc.json for five pages of The Twenty-Fourth Clue from the book's own sources.

    python3 samples/the-twenty-fourth-clue/make.py

Reads only the book (through the `book` link): the day-1 manuscript, book2/spec/days.json, the day-14
puzzle spec and drawing, the town map drawing, the stamp and plate pictures and the bundled fonts.
Writes doc.json beside this file and the trimmed drawings under build/art/.

The pages are the book's 2, 6, 10, 13 and 63. Recto and verso follow the physical page in Matenadar,
so empty pages keep each rebuilt page on the same side as in the book (rebuilt 2, 4, 6, 7, 9).
"""
from __future__ import annotations

import json
import math
import re
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
BOOK = HERE / "book"
ART = HERE / "build" / "art"
FONTS = BOOK / "assets" / "fonts"

INK = "#000"
MID = "#8a8a8a"
TITLE = "The Twenty-Fourth Clue"
PX = 0.75                     # one CSS pixel in points: Chrome floors border widths to whole pixels
ORDINALS = [None, "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth",
            "eleventh", "twelfth", "thirteenth", "fourteenth", "fifteenth", "sixteenth", "seventeenth",
            "eighteenth", "nineteenth", "twentieth", "twenty-first", "twenty-second", "twenty-third", "twenty-fourth"]


# ------------------------------------------------------------------ font metrics (for SVG baselines)
def hhea(path: Path) -> tuple[float, float]:
    """Ascender and descender (as fractions of the em) from a font's hhea table."""
    data = path.read_bytes()
    n = struct.unpack(">H", data[4:6])[0]
    tables = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack(">4sIII", data[12 + 16 * i: 28 + 16 * i])
        tables[tag.decode()] = off
    upm = struct.unpack(">H", data[tables["head"] + 18: tables["head"] + 20])[0]
    asc, desc = struct.unpack(">hh", data[tables["hhea"] + 4: tables["hhea"] + 8])
    return asc / upm, -desc / upm


METRICS = {
    "League Gothic": hhea(FONTS / "LeagueGothic-VF.ttf"),
    "Kalam": hhea(FONTS / "Kalam-Regular.ttf"),
    "Literata": hhea(FONTS / "Literata-VF.ttf"),
    "Besley": hhea(FONTS / "Besley-VF.ttf"),
}


def top_for_baseline(font: str, size: float, baseline: float, lh: float | None = None) -> float:
    """The top of a one-line frame whose first baseline lands at `baseline` (CSS half-leading model)."""
    asc, desc = METRICS[font]
    lh = size if lh is None else lh
    return baseline - ((lh - (asc + desc) * size) / 2 + asc * size)


def r2(v):
    return round(v, 2) if isinstance(v, (int, float)) else v


# ------------------------------------------------------------------ small element helpers
def text(x, y, w, content, style=None, **kw):
    el = {"type": "text", "x": r2(x), "y": r2(y), "w": r2(w)}
    if isinstance(content, str):
        el["text"] = content
    elif content and isinstance(content[0], dict) and content[0].get("type"):
        el["blocks"] = content
    else:
        el["content"] = content
    if style:
        el["style"] = style
    el.update(kw)
    return el


def rect(x, y, w, h, fill=None, stroke=None, **kw):
    el = {"type": "rect", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(h)}
    if fill:
        el["fill"] = fill
    if stroke:
        el["stroke"] = stroke
    el.update(kw)
    return el


def line(x1, y1, x2, y2, width, color=INK, **kw):
    s = {"width": width, "color": color}
    s.update(kw)
    return {"type": "line", "x1": r2(x1), "y1": r2(y1), "x2": r2(x2), "y2": r2(y2), "stroke": s}


def stroke(width, color=INK, **kw):
    s = {"width": width, "color": color}
    s.update(kw)
    return s


def svg(x, y, w, h, markup=None, src=None, **kw):
    el = {"type": "svg", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(h), "fit": "contain"}
    if markup:
        el["markup"] = markup
    if src:
        el["src"] = src
    el.update(kw)
    return el


def border_rect(x, y, w, h, width, **kw):
    """A CSS border box: the stroke lies inside the box, as a border does."""
    return rect(x + width / 2, y + width / 2, w - width, h - width, stroke=stroke(width), **kw)


def inline_md(s: str) -> list:
    """*italic* and **bold** from the book's small markdown dialect into runs."""
    out = []
    for part in re.split(r"(\*\*[^*]+\*\*|\*[^*]+\*)", s):
        if not part:
            continue
        if part.startswith("**"):
            out.append({"text": part[2:-2], "strong": True})
        elif part.startswith("*"):
            out.append({"text": part[1:-1], "em": True})
        else:
            out.append(part)
    return out


# ------------------------------------------------------------------ the book's SVG parts (parts.py), as markup
def postmark_svg(date: str, uid: str) -> str:
    """parts.postmark: a round date stamp with five cancellation waves; text on circular paths.
    The day number is left out here: it is set as live type over the drawing (postmark())."""
    month, _, num = date.partition(" ")
    r_out, r_in, r_text, cx, cy = 58, 40, 43.2, 62, 62
    top = f"M {cx - r_text} {cy} A {r_text} {r_text} 0 0 1 {cx + r_text} {cy}"
    rb = r_text + 9.8
    bot = f"M {cx - rb} {cy} A {rb} {rb} 0 0 0 {cx + rb} {cy}"
    waves = ""
    for k in range(5):
        y = cy - 4 * 7.5 + k * 15
        waves += (f'<path d="M 122 {y} q13 -8 26 0 t26 0 t26 0 t26 0 t26 0" fill="none" stroke="{INK}" '
                  f'stroke-width="2.4" stroke-linecap="round"/>')
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 250 126"><g transform="rotate(-6 62 62)">{waves}'
            f'<circle cx="{cx}" cy="{cy}" r="{r_out}" fill="#fff" stroke="{INK}" stroke-width="2.8"/>'
            f'<circle cx="{cx}" cy="{cy}" r="{r_in}" fill="none" stroke="{INK}" stroke-width="1.1"/>'
            f'<path id="{uid}t" d="{top}" fill="none"/><path id="{uid}b" d="{bot}" fill="none"/>'
            f'<text font-family="League Gothic" font-size="12.6" letter-spacing="1.9" fill="{INK}"><textPath href="#{uid}t" '
            f'startOffset="50%" text-anchor="middle">ALDERWICK POST OFFICE</textPath></text>'
            f'<text font-family="League Gothic" font-size="12.6" letter-spacing="2.4" fill="{INK}"><textPath href="#{uid}b" '
            f'startOffset="50%" text-anchor="middle">ROUTE 24</textPath></text>'
            f'<circle cx="{cx - 49}" cy="{cy + 3}" r="1.9" fill="{INK}"/><circle cx="{cx + 49}" cy="{cy + 3}" r="1.9" fill="{INK}"/>'
            f'<text x="{cx}" y="{cy - 16}" text-anchor="middle" font-family="League Gothic" font-size="18" letter-spacing="3.6" '
            f'fill="{INK}">{month}</text></g></svg>')


def postmark(x: float, y: float, w: float, date: str, uid: str, rotate: float = 0) -> dict:
    """The postmark drawing with its day number as live Besley Black: the book's SVG text inherited the body's
    old-style figures, which an SVG renderer does not apply. The number turns with the stamp's -6 degrees."""
    num = date.partition(" ")[2]
    size = 46 if len(num) == 1 else 40
    k = w / 250
    number = text(62 - 40, top_for_baseline("Besley", size, 62 + 25), 80, num, "pm-num", set={"size": size})
    return {"type": "group", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(126 * k), "rotate": rotate, "children": [
        svg(0, 0, w, 126 * k, postmark_svg(date, uid)),
        {"type": "group", "x": 0, "y": 0, "scale": round(k, 5), "children": [
            {"type": "group", "x": 0, "y": 0, "w": 124, "h": 124, "rotate": -6, "children": [number]}]}]}


def scallop_path(w: float, h: float, r: float, pitch: float, ox: float, oy: float) -> str:
    """parts.scallop_path: a w x h rectangle with half-round bites along each edge, starting at (ox, oy)."""
    def edge(length):
        n = max(3, round(length / pitch))
        return n, length / n

    d = [f"M {ox} {oy}"]
    n, step = edge(w)
    gap = (step - 2 * r) / 2
    d += [f"h {gap:.3f} a {r} {r} 0 0 0 {2 * r} 0 h {gap:.3f}"] * n
    n, step = edge(h)
    gap = (step - 2 * r) / 2
    d += [f"v {gap:.3f} a {r} {r} 0 0 0 0 {2 * r} v {gap:.3f}"] * n
    n, step = edge(w)
    gap = (step - 2 * r) / 2
    d += [f"h {-gap:.3f} a {r} {r} 0 0 0 {-2 * r} 0 h {-gap:.3f}"] * n
    n, step = edge(h)
    gap = (step - 2 * r) / 2
    d += [f"v {-gap:.3f} a {r} {r} 0 0 0 0 {-2 * r} v {-gap:.3f}"] * n
    return " ".join(d) + " Z"


WAVE = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 52 10"><path d="M2 5 q6 -5 12 0 t12 0 t12 0 t12 0" fill="none" '
        'stroke="#000" stroke-width="1.6" stroke-linecap="round"/></svg>')
ARROW = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 44 20"><path d="M2 10 H39 M31 3.5 L40.5 10 L31 16.5" fill="none" '
         'stroke="#000" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>')


def parcel_icon(filled: bool) -> str:
    """parts.parcel_icon: one parcel of the difficulty mark."""
    if not filled:
        return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 26"><rect x="2.6" y="5.6" width="24.8" height="17.8" '
                'fill="#fff" stroke="#000" stroke-width="0.9" stroke-dasharray="1.6 2.2" stroke-linecap="round"/></svg>')
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 26"><rect x="2" y="5" width="26" height="19" fill="#000" '
            'stroke="#000" stroke-width="1.6"/><path d="M15 5 V24 M2 14.5 H28" stroke="#fff" stroke-width="1.7"/>'
            '<path d="M15 5 c-5 -6 -9 -1 0 0 c9 -1 5 -6 0 0" fill="none" stroke="#000" stroke-width="1.5"/></svg>')


# ------------------------------------------------------------------ design
def opsz(size_pt: float) -> dict:
    """Chrome's font-optical-sizing: auto sets opsz to the size in CSS px, not points."""
    return {"opsz": round(size_pt * 4 / 3, 2)}


DESIGN = {
    "colors": {"ink": INK, "paper": "#fff", "mid": MID},
    "fonts": {"serif": ["Literata"], "display": ["Besley"], "label": ["League Gothic"], "iona": ["Kalam"]},
    "base": {"font": "serif", "size": 12, "line_height": "17.5pt", "color": "ink", "lang": "en-US",
             "variations": opsz(12)},
    "styles": {
        # the story: ragged right, hyphenated, first-line indents, first paragraph flush
        "story": {"size": 12.4, "line_height": "17.8pt", "indent": "1.35em", "hyphenate": True, "align": "left",
                  "features": ["onum", "pnum"], "variations": opsz(12.4)},
        "kicker": {"font": "label", "size": 13, "tracking": "0.2em", "line_height": 1, "features": []},
        "daytitle": {"font": "display", "weight": 800, "size": 28, "line_height": "29.5pt", "tracking": "-0.012em", "balance": True,
                     "hyphenate": False, "features": ["lnum"]},
        "tab": {"font": "label", "size": 13, "tracking": "0.14em", "line_height": 1, "color": "paper", "features": []},
        "addr": {"font": "iona", "size": 13.5, "line_height": "16.5pt", "features": ["lnum"]},
        "handoff": {"font": "label", "size": 13, "tracking": "0.2em", "line_height": 1, "features": []},
        "run": {"font": "label", "size": 10.5, "tracking": "0.14em", "case": "upper", "line_height": 1,
                "baseline": -0.47, "features": []},
        "folio": {"font": "display", "weight": 700, "size": 10.5, "line_height": 1, "features": ["lnum"]},
        "caption": {"font": "iona", "size": 12.5, "line_height": "16pt", "align": "center", "features": ["lnum"]},
        "stamp-legend": {"font": "label", "size": 11.5, "tracking": "2.4pt", "line_height": 1, "features": []},
        "stamp-value": {"font": "display", "weight": 900, "size": 15, "line_height": 1, "align": "right",
                        "features": ["onum"]},
        "pm-num": {"font": "display", "weight": 900, "size": 46, "line_height": 1, "align": "center", "features": ["onum"]},
        # the parcel label
        "strip": {"font": "label", "size": 17, "tracking": "0.2em", "line_height": 1, "color": "paper", "features": []},
        "form-key": {"font": "label", "size": 12, "tracking": "0.16em", "line_height": 1, "features": []},
        "parcel-hand": {"font": "iona", "size": 14, "line_height": "20pt", "features": ["lnum"]},
        # the sleeve
        "sleeve-tab": {"font": "label", "size": 19, "tracking": "0.12em", "line_height": 1, "color": "paper",
                       "features": []},
        "what": {"italic": True, "size": 10.5, "line_height": "13pt", "features": ["onum", "pnum"], "variations": opsz(10.5)},
        "diff-word": {"font": "label", "size": 9.6, "tracking": "0.14em", "case": "upper", "line_height": 1,
                      "features": []},
        "todo-label": {"font": "label", "size": 12.5, "tracking": "0.16em", "line_height": "17.5pt", "features": []},
        "steps": {"size": 10.6, "line_height": "14.2pt", "space_after": 2.5, "features": ["lnum", "pnum"],
                  "variations": opsz(10.6)},
        "answer": {"font": "iona", "size": 14, "line_height": 1, "features": ["lnum"]},
        "board-title": {"font": "label", "size": 12.5, "tracking": "1.5pt", "line_height": 1, "color": "paper",
                        "features": []},
        "board-right": {"font": "label", "size": 10.25, "tracking": "1.23pt", "line_height": 1, "color": "paper",
                        "align": "right", "features": []},
        "note-hand": {"font": "iona", "size": 13.4, "line_height": "17.4pt", "features": ["lnum"]},
        "map-note": {"font": "iona", "size": 12, "line_height": 1, "features": ["lnum"]},
    },
    "spans": {
        # the lead-in: the first sentence in small caps, a little larger and spaced
        "lead": {"case": "small-caps", "size": "1.04em", "tracking": "0.04em", "variations": opsz(12.4 * 1.04)},
    },
}

# Page frames. Verso: outside 0.7 in left, inside 0.95 in right; recto mirrored. Body 44.64 .. 740.17 pt.
TOP, BODY_BOTTOM = 44.64, 740.17
VERSO_L, RECTO_L, BODY_W = 50.39, 68.39, 493.22
FOOT_Y = 792 - 0.36 * 72 - 0.2 * 72                # the foot box: 0.2 in tall, 0.36 in above the foot of the page
GAP = 0.14 * 72                                      # folio - rule - run gaps


def foot(left: float, verso: bool) -> dict:
    """Folio, a flexible rule and the running text, as a three-column table (auto | 1fr | auto).

    The rule's bottom sits on the folio's baseline and is lifted 3 pt (CSS translateY), 1 px thick.
    """
    folio = {"content": [{"field": "page"}], "style": "folio"}
    lead = 0 if verso else 0.14 * 10.5             # after the recto's run text: its trailing letter-spacing
    rule = {"blocks": [{"type": "canvas", "h": 10.5, "elements": [
        line(lead, 9.58 - 3 - PX / 2, "100%", 9.58 - 3 - PX / 2, PX)]}]}
    run = {"content": [{"field": "section.run"}], "style": "run"}
    cells = [folio, rule, run] if verso else [run, rule, folio]
    # CSS letter-spacing also follows the last letter, so a right-aligned run stops one tracking short of the edge
    trail = 0.14 * 10.5 if verso else 0
    return {"type": "text", "x": r2(left - GAP / 2), "y": r2(FOOT_Y), "w": r2(BODY_W + GAP - trail),
            "when": "verso" if verso else "recto",
            "blocks": [{"type": "table", "columns": ["auto", "1fr", "auto"], "stroke": "none",
                        "inset": {"x": r2(GAP / 2), "y": 0}, "rows": [cells]}]}


MASTERS = {
    "matter": {"width": "8.5in", "height": "11in",
               "margins": {"top": TOP, "bottom": r2(792 - BODY_BOTTOM), "inside": "0.95in", "outside": "0.7in"},
               "fill": "paper"},
    "page": {"inherits": "matter", "foreground": [foot(VERSO_L, True), foot(RECTO_L, False)]},
    # story pages are narrower: 0.8 in outside, 1.2 in inside
    "story": {"inherits": "page", "margins": {"top": TOP, "bottom": r2(792 - BODY_BOTTOM), "inside": "1.2in",
                                              "outside": "0.8in"}, "style": "story"},
}


# ------------------------------------------------------------------ page 2: the parcel label
def parcel_page() -> dict:
    """A 5 in card turned -1.4 deg with an outline-only offset shadow, a black strip, form rows, Iona's hand,
    and the DEC 1 postmark hanging off its corner (turned 9 deg more)."""
    W, H = 360.0, 272.31                          # the card's border box (height from its content, as measured)
    X, Y = 117.0, 256.24                          # centred in the body
    b = PX                                         # 1.4 pt border prints as one CSS pixel
    kids = [
        # box-shadow: 4pt 4pt 0 -0.7pt #fff, 4pt 4pt 0 0.7pt #000  ->  a 1.4 pt ring around the box, offset 4,4
        rect(4, 4, W, H, fill="paper", stroke=stroke(1.4)),
        border_rect(0, 0, W, H, b, fill="paper"),
        rect(b, b, W - 2 * b, 31, fill="ink"),
        text(b + 16, b + 9, 330, "ALDERWICK BLIND ROUTE", "strip"),
    ]
    row_top = b + 31
    inner_l, inner_r = b + 0.3 * 72, W - b - 0.3 * 72
    keyed = lambda label, hidden=False: {"content": [{"text": label, **({"color": "paper"} if hidden else {})}], "style": "form-key"}
    # the rule starts after the label's trailing letter-spacing (CSS tracks the last letter too)
    rule_cell = {"blocks": [{"type": "canvas", "h": 12, "elements": [line(0.16 * 12, 12 - 0.375, "100%", 12 - 0.375, PX)]}]}
    rows = [[keyed("DELIVER TO"), rule_cell], [keyed("DELIVER TO", hidden=True), rule_cell]]
    for k, cells in enumerate(rows):
        kids.append({"type": "text", "x": r2(inner_l - 5), "y": r2(row_top + k * 37.92 + 0.36 * 72), "w": r2(inner_r - inner_l + 10),
                     "blocks": [{"type": "table", "columns": ["auto", "1fr"], "stroke": "none", "inset": {"x": 5, "y": 0},
                                 "rows": [cells]}]})
    kids.append({"type": "text", "x": r2(inner_l - 5), "y": r2(row_top + 2 * 37.92 + 0.36 * 72), "w": r2(inner_r - inner_l + 10),
                 "blocks": [{"type": "table", "columns": ["auto", "1fr", "auto", "1fr"], "stroke": "none",
                             "inset": {"x": 5, "y": 0},
                             "rows": [[keyed("BEGUN ON"), rule_cell, keyed("FINISHED ON"), rule_cell]]}]})
    hand_top = row_top + 3 * 37.92 + 0.3 * 72
    kids.append(text(inner_l, hand_top, W - 2 * b - 0.3 * 72 - 1.75 * 72,
                     "One sleeve a day.\nNo looking at tomorrow.\nCuriosity does not count\nas postage.", "parcel-hand"))
    # the postmark: right -0.5 in, bottom -0.42 in of the card's padding box, 2.5 in wide, rotate(9deg)
    kids.append(postmark(215.25, 207.08, 180, "DEC 1", "pmlabel", rotate=9))
    return {"type": "page", "id": "p2-label", "master": "matter", "page_from": 2, "elements": [
        {"type": "group", "id": "parcel", "x": X, "y": Y, "w": W, "h": H, "rotate": -1.4, "children": kids}]}


# ------------------------------------------------------------------ page 6: the town map
def map_page() -> dict:
    """The map is a code-drawn drawing (2,000 paths): placed as the book's SVG. Its margin note, which the book
    drew inside the SVG, is set here as a Matenadar text frame turned -90 degrees."""
    src = (BOOK / "puzzles/e2/map/town.html").read_text(encoding="utf-8")
    note_re = re.compile(r'<text x="11\.4" y="448"[^>]*>([^<]*)</text>')
    note = note_re.search(src).group(1)
    src = note_re.sub("", src, count=1)
    ART.mkdir(parents=True, exist_ok=True)
    (ART / "town.svg").write_text(src, encoding="utf-8")
    scale = BODY_W / 493.0                         # viewBox 0 0 493 660 drawn at the body width, top-aligned
    # the note's baseline starts at (11.4, 448) in drawing units and reads upward
    bx, by = VERSO_L + 11.4 * scale, TOP + 448 * scale
    W, Hh = 300.0, 12.0
    b = top_for_baseline("Kalam", 12, 0, 12) * -1      # baseline below the frame top
    cx, cy = bx - b + Hh / 2, by - W / 2               # centre of the frame so that rotating -90 puts the start there
    return {"type": "page", "id": "p6-map", "master": "page", "page_from": 6, "layers": True,
            "vars": {"run": TITLE}, "elements": [
                svg(VERSO_L, TOP, BODY_W, 660 * scale, src="build/art/town.svg", id="town-map"),
                text(cx - W / 2, cy - Hh / 2, W, note, "map-note", rotate=-90, id="map-note"),
            ]}


# ------------------------------------------------------------------ page 10: the day-1 opener
def read_day(n: int) -> dict:
    src = (BOOK / f"manuscript/days/day-{n:02d}.md").read_text(encoding="utf-8")
    meta = {}
    if src.startswith("---"):
        head, _, src = src[3:].partition("\n---")
        for ln in head.strip().splitlines():
            k, _, v = ln.partition(":")
            meta[k.strip()] = v.strip()
    opener, _, after = src.partition("<!-- AFTER -->")
    paras = [" ".join(p.split()) for p in re.split(r"\n\s*\n", opener.strip()) if p.strip()]
    return {"meta": meta, "paras": paras}


ABBREVIATIONS = {"Mr.", "Mrs.", "Ms.", "Dr.", "St."}


def story_blocks(paras: list[str], day: int) -> list[dict]:
    """parts.story_html: the first sentence (if six words or fewer, else three words) is the small-caps lead-in."""
    out = []
    for i, ptext in enumerate(paras):
        if i == 0:
            words = ptext.split(" ")
            n = min(3, len(words))
            for k, w in enumerate(words[:6]):
                if w[-1:] in ".?!”" and w not in ABBREVIATIONS:
                    n = k + 1
                    break
            lead = " ".join(words[:n])
            rest = " ".join(words[n:])
            content = [{"text": lead, "span": "lead"}] + ([" "] + inline_md(rest) if rest else [])
        else:
            content = inline_md(ptext)
        out.append({"type": "p", "id": f"d{day}-p{i + 1}", "style": "story", "content": content})
    return out


def stamp_group(x: float, y: float, picture: str, value: str, tilt: float) -> dict:
    """parts.stamp, drawn natively: a perforated stamp (scalloped outline, hard black offset copy), an inner
    frame, the picture fitted into its window, the legend and the value. viewBox -3 -3 131 159 at 1.5 in."""
    w, h = 122.0, 150.0
    s = 108 / 131                                   # the 1.5 in div holds the 131-unit-wide drawing
    o = 3.0                                         # viewBox origin -3: shift every unit by +3
    shape = scallop_path(w, h, 2.6, 8.6, o, o)
    shadow = scallop_path(w, h, 2.6, 8.6, o + 3, o + 3)
    leg_top = top_for_baseline("League Gothic", 11.5, h - 9.5 + o)
    val_top = top_for_baseline("Besley", 15, h - 8.4 + o)
    inner = [
        {"type": "path", "x": 0, "y": 0, "w": 131, "h": 159, "d": shadow, "fill": "ink"},
        {"type": "path", "x": 0, "y": 0, "w": 131, "h": 159, "d": shape, "fill": "paper", "stroke": stroke(0.9)},
        rect(9 + o, 9 + o, 104, 118, stroke=stroke(1.1)),
        {"type": "image", "src": picture, "x": 16 + o, "y": 16 + o, "w": 90, "h": 104, "fit": "contain"},
        text(11 + o, leg_top, 80, "ALDERWICK", "stamp-legend"),
        text(w - 11 + o - 40, val_top, 40, value, "stamp-value"),
    ]
    return {"type": "group", "x": r2(x), "y": r2(y), "w": 108, "h": 132.47, "rotate": tilt,
            "children": [{"type": "group", "x": 0, "y": 0, "scale": round(s, 5), "children": inner}]}


def story_section(days: dict) -> dict:
    d = days[1]
    st = read_day(1)
    W = 468.01                                       # 6.5 in story measure
    # the envelope face: 1.92 in tall; stamp top right, postmark over it, kicker and title bottom left
    head = [
        stamp_group(W - 108, 0, "book/visuals/e2/stamps/day-01.jpg", "1", 1.6),
        postmark(W - 0.52 * 72 - 198, 0.3 * 72, 198, d["date"], "pm1"),
        text(0, 138.24 - 79.99, 234.01, f"THE {ORDINALS[1].upper()} OF DECEMBER", "kicker"),
        text(0, 138.24 - 79.99 + 13 + 8, 234.01, d["title"], "daytitle", id="d1-title"),
    ]
    # the sleeve line: a 0.8 pt (1 px) box; a black tab; the address in Iona's hand
    tab_w = 63.38
    sleeve = [
        border_rect(0, 0, W, 28, PX),
        rect(PX, PX, tab_w, 28 - 2 * PX, fill="ink"),
        text(PX + 9, PX + 5 + (28 - 2 * PX - 8 - 13) / 2, tab_w - 9, "SLEEVE 1", "tab"),
        text(PX + tab_w + 10, PX + 6, W - tab_w - 2 * PX - 20, d["address"], "addr"),
    ]
    # the hand-off: a wave, OPEN SLEEVE 1, an arrow, centred
    hand = [
        svg(148.08, 1.5, 44.64, 10, WAVE),
        text(202.71, 0, 100, "OPEN SLEEVE 1", "handoff"),
        svg(297.92, 1.5, 22, 10, ARROW),
    ]
    flow = [
        {"type": "canvas", "id": "d1-envelope", "h": 138.24, "elements": head},
        {"type": "space", "h": 10},
        {"type": "canvas", "id": "d1-sleeveline", "h": 28, "elements": sleeve},
        {"type": "space", "h": 14},
        *story_blocks(st["paras"], 1),
        {"type": "space", "h": 16},
        {"type": "canvas", "id": "d1-handoff", "h": 13, "elements": hand},
    ]
    return {"type": "section", "id": "day-1", "master": "story", "start": "verso", "page_from": 10,
            "vars": {"run": TITLE}, "flow": flow}


# ------------------------------------------------------------------ page 13: the day-1 plate
def plate_page(days: dict) -> dict:
    d = days[1]
    cap = read_day(1)["meta"]["caption"]
    fx, fy, fw, fh = 71.88, 48.13, 486.23, 663.55      # the frame's border box: 3.5 pt inside the body
    return {"type": "page", "id": "p13-plate", "master": "page", "page_from": 13, "layers": True,
            "vars": {"run": f"{d['date'].title()} · {d['title']}"}, "elements": [
                {"type": "image", "id": "plate-1", "src": "book/visuals/e2/plates/day-01.jpg",
                 "x": fx + PX, "y": fy + PX, "w": r2(fw - 2 * PX), "h": r2(fh - 2 * PX), "fit": "cover"},
                border_rect(fx, fy, fw, fh, PX),
                # outline 0.5 pt (1 px) at a 3 pt offset
                border_rect(fx - 3 - PX, fy - 3 - PX, fw + 6 + 2 * PX, fh + 6 + 2 * PX, PX),
                text(RECTO_L, fy + fh + 3.5 + 9, BODY_W, cap, "caption", id="plate-1-cap"),
            ]}


# ------------------------------------------------------------------ page 63: sleeve 14
def split_puzzle(src: str) -> tuple[str, str, str]:
    """Cut the day-14 drawing into the soap (drawing), the wax-paper note and the board's sheet and strip (set in
    Matenadar), and the rest of the board (drawing)."""
    head = src[:src.index(">", src.index("<svg")) + 1]
    body = src[len(head):src.rindex("</svg>")]
    w0 = body.index('<g transform="rotate(-1.3 363 60.5)">')
    w1 = body.index("</g>", body.index("Six cuts")) + 4
    soap = body[:w0]
    rest = body[w1:]
    # the sheet's shadow, the sheet, the black strip and its two lines of type
    m = re.match(r'(<path d="M 6 509\.2[^>]*/>)(<rect x="3" y="129"[^>]*/>)(<rect x="3" y="129" width="485" height="20"[^>]*/>)'
                 r'(<text[^>]*>[^<]*</text>)(<text[^>]*>[^<]*</text>)', rest)
    assert m, "board sheet not found"
    rest = rest[m.end():]
    open_tag = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 493 510">'
    return open_tag + soap + "</svg>", open_tag + rest + "</svg>", m.group(0)


def sleeve_page(days: dict) -> dict:
    d = days[14]
    pz = d["puzzle"]
    spec = json.loads((BOOK / "puzzles/e2/day-14/spec.json").read_text(encoding="utf-8"))
    p = spec["printed"]
    soap_svg, board_svg, _ = split_puzzle((BOOK / "puzzles/e2/day-14/puzzle.html").read_text(encoding="utf-8"))
    ART.mkdir(parents=True, exist_ok=True)
    (ART / "d14-soap.svg").write_text(soap_svg, encoding="utf-8")
    (ART / "d14-board.svg").write_text(board_svg, encoding="utf-8")
    X, Y = RECTO_L, TOP
    els = []
    # head: black tab, the italic description, the difficulty mark; a 2.2 pt (2 px) rule under all three
    els += [rect(X, Y, 96.22, 28, fill="ink"),
            text(X + 12, Y + 6, 90, f"SLEEVE {d['day']}", "sleeve-tab"),
            text(173.25, Y + 28 - 4 - 13, 256.46, spec["heading"], "what"),
            text(438.35, Y + 28 - 4 - 1.5 - 9.6, 60, ["", "one parcel", "two parcels", "three parcels"][pz["difficulty"]], "diff-word")]
    for i in range(3):
        els.append(svg(495.84 + i * 23.09, Y + 28 - 4 - 17, 19.59, 16.99, parcel_icon(i < pz["difficulty"])))
    els.append(line(X, Y + 28 + 0.75, X + BODY_W, Y + 28 + 0.75, 1.5))
    # the artifact: 1 drawing unit = 1 pt, 0.14 in under the head
    AX, AY = X, Y + 29.5 + 0.14 * 72
    k = BODY_W / 493.0
    U = lambda v: v * k
    els.append(svg(AX, AY, BODY_W, 510 * k, src="build/art/d14-soap.svg", id="d14-soap"))
    # the wax-paper note, opened flat: pinked ends, a hard black copy as its shadow, two old folds, Iona's hand
    x, y, w, h, n = 244.0, 8.0, 238.0, 105.0, 11
    step = h / n
    right = " ".join(f"L {w - (2.6 if j % 2 else 0):.2f} {j * step:.2f}" for j in range(1, n + 1))
    left = " ".join(f"L {(2.6 if j % 2 else 0):.2f} {j * step:.2f}" for j in range(n - 1, -1, -1))
    poly = f"M 0 0 L {w} 0 {right} L 0 {h} {left} Z"
    note = [
        {"type": "path", "x": 2.2, "y": 2.4, "w": w, "h": h, "d": poly, "fill": "ink", "stroke": stroke(0.4, join="miter")},
        {"type": "path", "x": 0, "y": 0, "w": w, "h": h, "d": poly, "fill": "paper", "stroke": stroke(1.2, join="miter")},
    ]
    for fxr in (0.075, 0.80):
        note.append(line(w * fxr, 1.5, w * fxr, h - 1.5, 0.5, MID, dash=[5, 3]))
    note.append(text(30, top_for_baseline("Kalam", 13.4, 23.5, 17.4), 200, "\n".join(p["note"]), "note-hand"))
    els.append({"type": "group", "id": "d14-note", "x": r2(AX + U(x)), "y": r2(AY + U(y)), "w": r2(U(w)), "h": r2(U(h)),
                "rotate": -1.3, "children": [{"type": "group", "x": 0, "y": 0, "scale": round(k, 5), "children": note}]})
    # the locksmith's board: a sheet with a thin hard shadow, a black title strip with reversed caps
    bx, by, bw, bh = 3.0, 129.0, 485.0, 378.0
    board = [
        {"type": "path", "x": 0, "y": 0, "w": 493, "h": 510, "d": f"M {bx + 3} {by + bh + 2.2} H {bx + bw + 2.2} V {by + 3}",
         "stroke": stroke(2.0, join="miter")},
        rect(bx, by, bw, bh, fill="paper", stroke=stroke(1.2)),
        rect(bx, by, bw, 20, fill="ink"),
        text(bx + 9, top_for_baseline("League Gothic", 12.5, by + 20 - 5.6), 300, p["board_title"].upper(), "board-title"),
        text(bx + bw - 9 - 200, top_for_baseline("League Gothic", 10.25, by + 20 - 5.6), 200, p["board_right"].upper(),
             "board-right"),
    ]
    els.append({"type": "group", "id": "d14-board", "x": r2(AX), "y": r2(AY), "scale": round(k, 5), "children": board})
    els.append(svg(AX, AY, BODY_W, 510 * k, src="build/art/d14-board.svg", id="d14-keys"))
    # WHAT TO DO: a 0.8 pt (1 px) rule, the label, three steps
    ty = AY + 524.44 + 0.12 * 72
    els.append(line(X, ty + PX / 2, X + BODY_W, ty + PX / 2, PX))
    els.append(text(X, ty + PX + 6 + 1, 70, "WHAT TO DO", "todo-label"))
    els.append(text(X + 61.13 + 0.16 * 72, ty + PX + 6, 420.57,
                    [{"type": "p", "style": "steps", "content": inline_md(s)} for s in spec["todo"]], id="d14-steps"))
    # the answer: Iona's prompt, one box per letter, the fourth wearing the double postmark ring
    ay = 695.53 + 22.32                               # the row is 0.62 in tall, its items centred
    els.append(text(X, ay - 7, 210, pz["prompt_before"], "answer"))
    bx0, bw_, bh_ = 281.75, 28.8, 31.68
    xs = []
    xcur = bx0
    for i in range(len(pz["answer"])):
        if i + 1 == pz["ring"]:
            xcur += 11.5
        xs.append(xcur)
        xcur += bw_ + (11.5 if i + 1 == pz["ring"] else 0) + 3
    for i, bxp in enumerate(xs):
        els.append(border_rect(bxp, ay - bh_ / 2, bw_, bh_, PX))
        if i + 1 == pz["ring"]:
            cx = bxp + bw_ / 2
            els.append({"type": "ellipse", "x": r2(cx - 0.32 * 72 + 0.75), "y": r2(ay - 0.32 * 72 + 0.75), "w": r2(0.64 * 72 - 1.5),
                        "h": r2(0.64 * 72 - 1.5), "stroke": stroke(1.5)})
            els.append({"type": "ellipse", "x": r2(cx - 0.36 * 72 + 0.375), "y": r2(ay - 0.36 * 72 + 0.375),
                        "w": r2(0.72 * 72 - 0.75), "h": r2(0.72 * 72 - 0.75), "stroke": stroke(PX)})
    run = "STUCK?  NUDGE P. 114  ·  SHOVE P. 116  ·  NEARLY P. 118"
    return {"type": "page", "id": "p63-sleeve", "master": "page", "page_from": 63, "layers": True,
            "vars": {"run": run}, "elements": els}


def blank(n: int) -> dict:
    return {"type": "page", "id": f"blank-{n}", "master": "matter", "elements": []}


def main() -> None:
    days = {d["day"]: d for d in json.loads((BOOK / "book2/spec/days.json").read_text(encoding="utf-8"))["days"]}
    doc = {
        "matenadar": "1",
        "meta": {"title": TITLE, "author": "Levon Arakelyan", "lang": "en-US"},
        "fonts": ["book/assets/fonts"],
        "design": {**DESIGN, "masters": MASTERS},
        "content": [
            blank(1), parcel_page(),
            blank(3), map_page(),
            story_section(days),
            plate_page(days),
            blank(8), sleeve_page(days),
        ],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
