#!/usr/bin/env python3
"""Build doc.json for five pages of the Third Shift Handbook from the book's own sources.

    uv run --with fonttools python3 samples/third-shift-handbook/make.py

Reads only the book (through the `book` link): the page-structured manuscript (parsed with the book's own
scripts/hb/engine.py, imported without writing bytecode), the hands table and its seeded jitter, the machine
drawing (scripts/hb/diagram.py), the mascot picture and the bundled fonts. Writes doc.json beside this file.

The pages are the book's 1, 7, 25, 30 and 75. Recto and verso follow the physical page in Matenadar, so two
empty pages keep each rebuilt page on its side (rebuilt 1, 3, 5, 6, 7).
"""
from __future__ import annotations

import json
import math
import random
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True                    # importing the book's scripts must not write into the book
HERE = Path(__file__).resolve().parent
BOOK = HERE / "book"
sys.path.insert(0, str(BOOK / "scripts" / "hb"))
import engine                                     # noqa: E402  (the book's parser, hands and seeds)
import diagram                                    # noqa: E402  (the book's machine drawing)

from fontTools.ttLib import TTFont                # noqa: E402

FONTS = BOOK / "assets" / "fonts"
PX = 0.75                                         # one CSS pixel; Chrome floors border widths to whole pixels

C = {"ink": "#141414", "yellow": "#f4b400", "pen-blue": "#233f94", "pen-black": "#1b1b1f", "pencil": "#5d5a56",
     "pencil-dk": "#3b3937", "ink-black": "#1c2433", "pink": "#f7d9d6", "rule-blue": "#9db0c4", "tape": "#efe7c8",
     "tape-edge": "#d9cfa6", "slip": "#fdfcf6", "slip-edge": "#8d8d8d", "grey": "#555555", "paper": "#ffffff",
     "form-edge": "#b08a88", "area-edge": "#7a4a48", "serial": "#b8322c"}
PEN = {"var(--pen-blue)": C["pen-blue"], "var(--pencil-dk)": C["pencil-dk"], "var(--ink-black)": C["ink-black"],
       "var(--pen-black)": C["pen-black"], "var(--pencil)": C["pencil"]}

# ------------------------------------------------------------------ fonts: families, metrics, advances
FILES = {
    "TeX Gyre Schola": "texgyreschola-regular.otf", "TeX Gyre Heros": "texgyreheros-regular.otf",
    "Jost": "Jost-Medium.ttf", "Jost Bold": "Jost-Bold.ttf", "League Gothic": "LeagueGothic-Regular.ttf",
    "Special Elite": "SpecialElite-Regular.ttf", "Cutive Mono": "CutiveMono-Regular.ttf",
}
_fonts: dict[str, TTFont] = {}


def font(file: str) -> TTFont:
    if file not in _fonts:
        path = FONTS / file if (FONTS / file).exists() else FONTS / "hands" / file
        _fonts[file] = TTFont(path)
    return _fonts[file]


def family_of(file: str) -> str:
    n = font(file)["name"]
    return n.getDebugName(16) or n.getDebugName(1)


def metrics(file: str) -> tuple[float, float]:
    t = font(file)
    upm = t["head"].unitsPerEm
    os2, hh = t["OS/2"], t["hhea"]
    if os2.fsSelection & 128:
        return os2.sTypoAscender / upm, -os2.sTypoDescender / upm
    return hh.ascent / upm, -hh.descent / upm


def advance(file: str, text: str, size: float, tracking: float = 0) -> float:
    t = font(file)
    cmap, hmtx, upm = t.getBestCmap(), t["hmtx"], t["head"].unitsPerEm
    return sum(hmtx[cmap[ord(ch)]][0] / upm * size + tracking for ch in text)


def top_for_baseline(file: str, size: float, baseline: float, lh: float) -> float:
    """The top of a frame whose first baseline lands at `baseline` (CSS half-leading model)."""
    asc, desc = metrics(file)
    return baseline - ((lh - (asc + desc) * size) / 2 + asc * size)


def r2(v):
    return round(v, 2) if isinstance(v, (int, float)) else v


def stroke(width, color="ink", **kw):
    s = {"width": r2(width), "color": color}
    s.update(kw)
    return s


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


def rect(x, y, w, h, fill=None, stroke_=None, **kw):
    el = {"type": "rect", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(h)}
    if fill:
        el["fill"] = fill
    if stroke_:
        el["stroke"] = stroke_
    el.update(kw)
    return el


def line(x1, y1, x2, y2, width, color="ink"):
    return {"type": "line", "x1": r2(x1), "y1": r2(y1), "x2": r2(x2), "y2": r2(y2), "stroke": stroke(width, color)}


def tl_rotation(x, y, w, h, deg):
    """CSS turns notes about their top-left corner (transform-origin 0 0); Matenadar turns elements about their
    centre. Return the top-left that makes a centre rotation land where the corner rotation did."""
    a = math.radians(deg)
    cx, cy = w / 2, h / 2
    rx, ry = cx * math.cos(a) - cy * math.sin(a), cx * math.sin(a) + cy * math.cos(a)
    return x + rx - cx, y + ry - cy


# ------------------------------------------------------------------ the hands: engine.HANDS, seeded like engine.render_hand
QUOTES = str.maketrans({})


def curl(s: str) -> str:
    """build.curl_quotes: an opening quote after a space or a bracket, a closing one (or an apostrophe) otherwise."""
    out = []
    for ch in s:
        before = out[-1] if out else ""
        opens = before == "" or before.isspace() or before in "([{\u2014\u2013-\u201c\u2018"
        if ch == "'":
            out.append("\u2018" if opens else "\u2019")
        elif ch == '"':
            out.append("\u201c" if opens else "\u201d")
        else:
            out.append(ch)
    return "".join(out)


STROKE = {"ruth": 1.05, "walt": 2.1, "coyle": 0.95, "clerk": 1.0}         # layout.js pen widths, CSS px


def hand(textsrc: str, who: str, key: str, *, size: float | None = None, amp: float | None = None,
         lh: float | None = None, marks: bool = True, struck: bool = False, font_file: str | None = None) -> dict:
    """Handwriting as Matenadar runs, with the book's own seeded jitter (the same draws, in the same order, as
    engine.render_hand): every letter gets its vertical nudge and size wobble, every word its drifting baseline.
    The ±2.4 deg turn of each letter and ±1.5 deg of each word are drawn but have no run equivalent.
    Substituted letters (Ruth's k and full stop, Coyle's figures) come from the sister font at their scale.
    _x_ is underlined and ~x~ struck through with a pen mark in the hand's own pen; *x* is pressed harder.
    CSS lays the letters out at their unscaled size (a transform does not move them) and tracks every letter;
    Matenadar sizes each letter and drops tracking at run edges, so each word's difference goes into the space
    after it, and every line breaks where the book's did.
    Returns {"paras": [[runs]], "size", "lh", "color", "style"}."""
    cfg = engine.HANDS[who]
    rng = random.Random(engine.seed_of(who, key, textsrc))
    amp = cfg["amp"] if amp is None else amp
    body = textsrc.upper() if cfg.get("caps") else textsrc
    paras = [p for p in re.split(r"\n\s*\n", body.strip()) if p.strip()]
    raw = []
    for para in paras:
        lines = []
        for chunk in re.split(r"\s*//\s*", " ".join(ln.strip() for ln in para.splitlines())):
            tokens = []
            drift = 0.0
            for style, run in engine.hand_inline(curl(chunk)):
                for wi, word in enumerate(run.split(" ")):
                    if wi:
                        tokens.append(("space", style))
                    if not word:
                        continue
                    drift = max(-1.3, min(1.3, drift + rng.uniform(-0.45, 0.45)))
                    letters = []
                    for ch in word:
                        rng.uniform(-2.4, 2.4)                 # the letter's turn
                        ty = rng.uniform(-0.04, 0.04) * amp
                        sc = 1 + rng.uniform(-0.045, 0.045) * amp
                        letters.append((ch, ty, sc))
                    rng.uniform(-1.5, 1.5)                     # the word's turn
                    tokens.append(("word", style, letters, drift * amp))
            lines.append(tokens)
        raw.append(lines)
    wobble = 1 + rng.uniform(-0.03, 0.035) if size is None else 1
    fs = (size or cfg["size"]) * wobble
    color = PEN.get(cfg["color"], cfg["color"])
    pen = stroke(STROKE.get(who, 1.05) * PX, color)
    subs = cfg.get("subs", {})
    track = {"walt": 0.02, "coyle": -0.01}.get(who, 0) * fs
    base_file = font_file or cfg["file"]

    def adv(file, ch, sz):
        try:
            return advance(file, ch, sz)
        except KeyError:
            return 0.0

    def mark_for(style):
        if struck:
            return {"kind": "strike", "stroke": pen}
        if style == "u" and marks:
            return {"kind": "underline", "stroke": pen, "offset": r2(0.12 * fs)}
        if style == "s" and marks:
            return {"kind": "strike", "stroke": pen}
        return None

    out = []
    for lines in raw:
        runs = []
        for li, tokens in enumerate(lines):
            if li:
                runs.append({"br": True})
            carry = 0.0                      # a word's extra width with no space before it on this line
            before = None                    # the space run just before the current word
            for tok in tokens:
                if tok[0] == "space":
                    if runs and isinstance(runs[-1], dict) and runs[-1].get("text") == " ":
                        continue
                    run = {"text": " ", "size": r2(fs), "_extra": track + carry}
                    carry = 0.0
                    if struck:
                        run["mark"] = mark_for(tok[1])
                    runs.append(run)
                    before = run
                    continue
                _, style, letters, drift = tok
                debt = 0.0
                for ch, ty, sc in letters:
                    run = {"text": ch}
                    file, css_size, size_ = base_file, fs, fs * sc
                    if ch in subs:
                        file, scale, _skew = subs[ch]
                        fam = family_of(file)
                        if fam != family_of(base_file):
                            run["font"] = fam
                            if "Light" in file:
                                run["weight"] = 300
                        css_size = fs * scale
                        if scale <= 1:
                            size_ = fs * scale * sc
                        else:
                            # a full stop set 1.7 em with line-height 0 in the book: a heavier dot here, so the
                            # line keeps its height (a larger run would open the line)
                            size_ = fs
                            run["outline"] = {"width": r2(0.03 * fs * (scale - 1)), "color": color}
                    run["size"] = r2(size_)
                    debt += adv(file, ch, css_size) - adv(file, ch, r2(size_)) + track
                    shift = -(ty * css_size + drift * PX)
                    if abs(shift) >= 0.01:
                        run["baseline"] = r2(shift)
                    if style == "b":
                        run["outline"] = {"width": 0.2, "color": color}
                    m = mark_for(style)
                    if m:
                        run["mark"] = m
                    runs.append(run)
                # the word's difference widens the space before it, so a last word that would not fit in the
                # book does not fit here either
                if before is not None:
                    before["_extra"] += debt
                    before = None
                else:
                    carry += debt
            for run in runs:
                if isinstance(run, dict) and "_extra" in run:
                    extra = run.pop("_extra")
                    if abs(extra) >= 0.01:
                        run["word_spacing"] = f"{r2(extra)}pt"
        out.append(runs)
    # the paragraph is set at the largest letter size, so no nudged letter opens its line
    style = {"font": family_of(base_file), "size": r2(fs * (1 + 0.045 * amp)), "color": color, "features": [],
             "hyphenate": False, "align": "left", "line_height": f"{r2((lh or cfg['lh']) * fs)}pt",
             "space_after": r2(0.3 * fs), "space_before": 0, "indent": 0}
    return {"paras": out, "size": fs, "lh": (lh or cfg["lh"]) * fs, "color": color, "style": style}


def hand_blocks(h: dict, extra: dict | None = None) -> list[dict]:
    blocks = []
    for i, runs in enumerate(h["paras"]):
        set_ = dict(h["style"])
        if i == len(h["paras"]) - 1:
            set_["space_after"] = 0
        if extra:
            set_.update(extra)
        blocks.append({"type": "p", "set": set_, "content": runs})
    return blocks


def note_rot(page_id: str, index: int, who: str, attrs: dict, placement: str) -> float:
    """engine.note_html: the note's slope, seeded."""
    cfg = engine.HANDS[who]
    rng = random.Random(engine.seed_of("rot", f"{page_id}:{index}"))
    if "rot" in attrs:
        return float(attrs["rot"])
    if placement in ("below", "gap"):
        return rng.uniform(-0.45, 0.45)
    return cfg["slope"] + rng.uniform(-0.9, 0.9)


# ------------------------------------------------------------------ layout.js: its seeded pen marks
def fnv(s: str) -> int:
    h = 2166136261
    for c in s:
        h ^= ord(c)
        h = (h * 16777619) & 0xFFFFFFFF
    return h


def mulberry32(seed: int):
    a = seed & 0xFFFFFFFF

    def imul(x, y):
        return (x * y) & 0xFFFFFFFF

    def r():
        nonlocal a
        a = (a + 0x6D2B79F5) & 0xFFFFFFFF
        t = imul(a ^ (a >> 15), 1 | a)
        t = ((t + imul(t ^ (t >> 7), 61 | t)) & 0xFFFFFFFF) ^ t
        return ((t ^ (t >> 14)) & 0xFFFFFFFF) / 4294967296
    return r


def wobble_path(x1, y1, x2, y2, r, amp):
    """layout.js wobble(): a polyline with a vertex every 14 px, each jittered (px in, px out)."""
    ln = math.hypot(x2 - x1, y2 - y1)
    steps = max(2, round(ln / 14))
    d = f"M{x1:.1f} {y1:.1f}"
    for s in range(1, steps + 1):
        t = s / steps
        d += f" L{x1 + (x2 - x1) * t + (r() - 0.5) * amp:.1f} {y1 + (y2 - y1) * t + (r() - 0.5) * amp * 1.6:.1f}"
    return d


# ------------------------------------------------------------------ design
TOP, COL_H = 0.72 * 72, 537.12
TEXT_W, WIDE_W, MARG_W, GAP = 3.4 * 72, 5 * 72, 1.46 * 72, 0.14 * 72
RECTO_X, VERSO_X = 0.62 * 72 - 3.6, 0.38 * 72 + 3.6     # the trim plane slides 0.05 in toward the spine
FOOT_Y = 648 - 0.47 * 72 - 0.2 * 72 - 3 - PX          # the foot's top rule: bottom 0.47 in, 0.2 in + 3 pt + rule

DESIGN = {
    "colors": C,
    "fonts": {"schola": ["TeX Gyre Schola"], "heros": ["TeX Gyre Heros"], "jost": ["Jost"], "gothic": ["League Gothic"]},
    "base": {"font": "schola", "size": 9.3, "line_height": "13.2pt", "color": "ink", "lang": "en-US"},
    "styles": {
        # 1957 print: Schola 9.3/13.2, justified and hyphenated, old-style figures, 6.6 pt between paragraphs
        "body": {"align": "justify", "hyphenate": True, "features": ["onum"], "space_after": 6.6,
                 # Chrome breaks greedily and only stretches spaces
                 "linebreaks": "simple", "justification": {"spacing": {"min": "100%", "max": "400%"}}},
        "h2": {"font": "jost", "weight": 700, "size": 9.2, "line_height": "12pt", "tracking": "0.06em", "case": "upper",
               "space_before": 11, "space_after": 3.5, "margin_left": 4, "keep_with_next": True, "hyphenate": False},
        "sec-n": {"font": "gothic", "size": 50, "line_height": "40pt", "align": "center"},
        "sec-t": {"font": "gothic", "size": 23, "line_height": "22.54pt", "tracking": "0.03em", "case": "upper"},
        "display": {"font": "gothic", "size": 30, "line_height": "30pt", "tracking": "0.03em", "case": "upper"},
        "slip-rev": {"font": "Special Elite", "size": 7.6, "line_height": "12pt", "tracking": "0.02em", "case": "upper", "align": "left",
                     "underline": True, "space_after": 4},
        "slip": {"font": "Special Elite", "size": 8.3, "line_height": "12pt", "align": "left", "space_after": 4},
        "foot-run": {"font": "jost", "weight": 500, "size": 6.6, "tracking": "0.14em", "case": "upper",
                     "line_height": "13.5pt", "baseline": -0.83},
        "foot-folio": {"font": "jost", "weight": 700, "size": 9, "line_height": "13.5pt"},
        "foot-rev": {"font": "jost", "weight": 500, "size": 6.6, "tracking": "0.06em", "case": "upper", "color": "grey",
                     "line_height": "13.5pt", "baseline": -0.83},
        "key": {"size": 8.4, "line_height": "11.6pt", "features": ["onum"], "align": "left"},
        "key-n": {"font": "jost", "weight": 700, "size": 6.6, "line_height": "10.4pt", "align": "center"},
        # the title page
        "tp-co": {"font": "jost", "weight": 700, "size": 10, "tracking": "0.22em", "case": "upper"},
        "tp-t": {"font": "gothic", "size": 82, "line_height": "70.52pt", "tracking": "0.015em", "case": "upper",
                 "outline": {"width": 2.56, "color": "ink"}},
        "tp-sub": {"italic": True, "size": 12, "features": ["onum"]},
        "tp-place": {"font": "jost", "weight": 500, "size": 7.4, "tracking": "0.2em", "case": "upper"},
        "issue": {"font": "jost", "weight": 500, "size": 7, "tracking": "0.1em", "case": "upper"},
        "issue-lab": {"font": "jost", "weight": 500, "size": 7, "tracking": "0.1em", "case": "upper",
                      "line_height": "13.2pt", "baseline": -8.4},
        "issue-p": {"size": 8.4, "line_height": "11.4pt", "align": "justify", "hyphenate": True, "features": ["onum"],
                    "linebreaks": "simple", "justification": {"spacing": {"min": "100%", "max": "400%"}}},
        # the NOTES page
        "nh-t": {"font": "gothic", "size": 23, "line_height": "23pt", "tracking": "0.05em", "case": "upper"},
        "nh-s": {"italic": True, "size": 9},
        "nh-sec": {"font": "jost", "weight": 500, "size": 6.6, "tracking": "0.14em", "case": "upper", "align": "right"},
        # 1971 forms: Heros
        "f-co": {"font": "heros", "stretch": 82, "weight": 700, "size": 8, "tracking": "0.1em", "case": "upper",
                 "align": "center", "line_height": "10.6pt"},
        "f-title": {"font": "heros", "weight": 700, "size": 11, "tracking": "0.06em", "case": "upper", "align": "center",
                    "line_height": "10.6pt", "space_before": 2, "space_after": 6},
        "f-l": {"font": "heros", "size": 6.4, "tracking": "0.07em", "case": "upper", "line_height": "10.6pt"},
        "f-lab": {"font": "heros", "size": 6.4, "tracking": "0.07em", "case": "upper", "line_height": "10.6pt",
                  "space_before": 4},
        "f-check": {"font": "heros", "size": 7.2, "line_height": "10.6pt"},
        "f-fine": {"font": "heros", "size": 6, "line_height": "8pt", "space_before": 4},
        "f-no": {"font": "Cutive Mono", "size": 8, "line_height": "10.6pt", "color": "serial", "align": "right"},
    },
}


def foot_master(col_x: float, width: float, recto: bool) -> list[dict]:
    """The running foot: a 0.6 pt (1 px) rule over folio, running title and revision, baselines aligned.
    Recto: Rev. at the left, title and folio at the right; verso the mirror. A three-column table: 1fr | auto | auto."""
    rev = {"content": [{"text": "Rev. "}, {"field": "section.rev"}], "style": "foot-rev"}
    run = {"content": [{"field": "section.run"}], "style": "foot-run"}
    folio = {"content": [{"field": "page"}], "style": "foot-folio"}
    when = "recto" if recto else "verso"
    if recto:
        cols, cells = ["1fr", "auto", "auto"], [rev, run, folio]
        align = ["left", "right", "right"]
    else:
        cols, cells = ["auto", "auto", "1fr"], [folio, run, rev]
        align = ["left", "left", "right"]
    return [
        {**line(col_x, FOOT_Y + PX / 2, col_x + width, FOOT_Y + PX / 2, PX), "when": when},
        {"type": "text", "x": r2(col_x - 4), "y": r2(FOOT_Y + PX + 3), "w": r2(width + 8), "when": when,
         "blocks": [{"type": "table", "columns": cols, "stroke": "none", "inset": {"x": 4, "y": 0}, "align": align,
                     "rows": [cells]}]},
    ]


def master(width: float, notes: bool = False, foot: bool = True) -> dict:
    m = {"width": "6in", "height": "9in",
         "margins": {"top": r2(TOP), "bottom": r2(648 - TOP - COL_H), "inside": r2(RECTO_X),
                     "outside": r2(432 - RECTO_X - width)},
         "fill": "paper", "style": "body"}
    if notes:
        m["notes"] = {"width": r2(MARG_W), "gap": r2(GAP), "side": "outside", "spacing": 6.75}
    if foot:
        m["foreground"] = foot_master(RECTO_X, width, True) + foot_master(VERSO_X, width, False)
    return m


MASTERS = {
    "plain": master(WIDE_W, foot=False),
    "rules": master(TEXT_W, notes=True),
    "wide": master(WIDE_W),
}


# ------------------------------------------------------------------ the manuscript
def pages() -> dict:
    out = {}
    for name in (BOOK / "manuscript/order.txt").read_text().split():
        for p in engine.parse_file(BOOK / "manuscript" / name):
            out[p.id] = p
    return out


def notes_of(page) -> list[tuple[int, object]]:
    return [(i, b) for i, b in enumerate(page.blocks) if b.kind == "note"]


# ------------------------------------------------------------------ page 1: the title page
def title_page(pg) -> dict:
    d = next(b for b in pg.blocks if b.kind == "directive" and b.name == "titlepage")
    a = d.attrs
    key = f"{pg.id}:{pg.blocks.index(d)}"
    X, W = RECTO_X, WIDE_W
    els = [rect(X, TOP, W, 0.13 * 72, fill="yellow")]
    y = TOP + 0.13 * 72 + 0.3 * 72
    els.append(text(X, y, W, "Sable River Paper Company", "tp-co"))
    y += 13.2 + 0.28 * 72
    els.append(text(X, y, W, "Third\nShift\nHandbook", "tp-t", id="tp-title"))
    y += 3 * 70.52 + 0.05 * 72 + 0.16 * 72
    els.append(text(X, y, W, curl(a["sub"]), "tp-sub"))
    # the foot of the title page: place and year spread apart, then the issue panel
    fy = TOP + COL_H - 132.2
    els.append(text(X, fy, W, "Carrow Falls, Maine", "tp-place", set={"line_height": "13.2pt"}))
    els.append(text(X, fy, W, a.get("year", "1957"), "tp-place", set={"line_height": "13.2pt", "align": "right"}))
    iy = fy + 13.2 + 0.16 * 72
    ih = TOP + COL_H - iy
    els.append(rect(X + PX / 2, iy + PX / 2, W - PX, ih - PX, stroke_=stroke(PX)))
    ix, iw = X + PX + 11, W - 2 * PX - 22

    def field(name):
        # As printed, the clerk's entries are not in her hand: the book's `.issue p` rule (Schola 8.4/11.4, 9 pt
        # above) also catches the paragraphs of the handwriting, so the PDF shows them in Schola with old-style
        # figures. The rebuild copies the printed page: the same seeded jitter, in Schola.
        v = hand(a.get(name, ""), a.get("hand", "clerk"), key + name, size=8.4, font_file=FILES["TeX Gyre Schola"])
        st = dict(v["style"], space_after=0, line_height="11.4pt", features=["onum"], size=8.4)
        # .i-v .hw: 4 pt in, translateY(-1pt) rotate(-1.2deg); the p's 9 pt margin puts the line 9 pt down
        y = 21.6 - 21.15 + 9 - 1
        return {"blocks": [{"type": "canvas", "h": 21.6, "elements": [
            line(0, 21.6 - PX / 2, "100%", 21.6 - PX / 2, PX),
            {"type": "text", "x": 4, "y": r2(y), "w": 160, "rotate": -1.2,
             "blocks": [{"type": "p", "set": st, "content": v["paras"][0]}]}]}]}

    def label(s):
        # a cell given as blocks widens its auto column, so the label is plain text lowered to the row's foot
        return {"text": s, "style": "issue-lab"}
    for k, (l1, f1, l2, f2) in enumerate((("This copy is issued to", "to", "Clock No.", "no"),
                                          ("Issued by", "by", "Date", "date"))):
        els.append({"type": "text", "x": r2(ix - 3), "y": r2(iy + PX + 8 + 7 + k * (21.6 + 7)), "w": r2(iw + 6),
                    "blocks": [{"type": "table", "columns": ["auto", "1fr", "auto", r2(0.9 * 72 + 4)], "stroke": "none",
                                "inset": {"x": 3, "y": 0}, "rows": [[label(l1), field(f1), label(l2), field(f2)]]}]})
    els.append(text(ix, iy + PX + 8 + 7 + 2 * 21.6 + 7 + 9, iw, curl(d.text.strip()), "issue-p"))
    # Walt's two notes, placed free on the trim (inches), turned about their top-left corners
    for i, b in notes_of(pg):
        h = hand(b.text, b.name, f"{pg.id}:{i}", size=float(b.attrs["size"]) if "size" in b.attrs else None,
                 struck="struck" in b.attrs)
        x, y, w = float(b.attrs["x"]) * 72 - 3.6, float(b.attrs["y"]) * 72, float(b.attrs["w"]) * 72
        n_lines = 5 if "//" in b.text else 3
        hh = n_lines * h["lh"]
        rot = note_rot(pg.id, i, b.name, b.attrs, "free")
        px, py = tl_rotation(x, y, w, hh, rot)
        els.append({"type": "text", "id": f"title-note-{i}", "x": r2(px), "y": r2(py), "w": r2(w), "h": r2(hh), "rotate": r2(rot),
                    "blocks": hand_blocks(h)})
        if "struck" in b.attrs:
            # layout.js goes through every row twice: the second pass is the same text again, invisible, with
            # its strike set lower (a run carries one mark)
            ghost = json.loads(json.dumps(hand_blocks(h)))
            for blk in ghost:
                blk["set"]["color"] = "#00000000"
                for run in blk["content"]:
                    if isinstance(run, dict) and "mark" in run:
                        run["mark"]["offset"] = r2(0.17 * h["size"])
            els.append({"type": "text", "id": f"title-note-{i}-again", "x": r2(px), "y": r2(py), "w": r2(w), "h": r2(hh),
                        "rotate": r2(rot), "blocks": ghost})
            # and then one slash across the lot, bottom-left to top-right
            r = mulberry32(fnv(pg.id + ":struck:0"))
            d_ = wobble_path(-2, hh / PX + 1, w / PX * 0.95 + 2, -1, r, 2)
            els.append({"type": "group", "x": r2(px), "y": r2(py), "w": r2(w), "h": r2(hh), "rotate": r2(rot), "children": [
                {"type": "path", "x": 0, "y": 0, "w": r2(w), "h": r2(hh), "viewbox": [0, 0, r2(w / PX), r2(hh / PX)], "d": d_,
                 "stroke": stroke(STROKE["walt"] * PX, h["color"], cap="round", join="round")}]})
    return {"type": "page", "id": "p1-title", "master": "plain", "page_from": 1, "elements": els}


# ------------------------------------------------------------------ page 7: Section 1 opens
def opener_section(pg) -> dict:
    blocks = pg.blocks
    flow = []
    W = TEXT_W
    # the section header: the yellow number chip and the title, bottoms aligned on a 2.2 pt (2 px) rule
    h1 = blocks[0]
    num, _, title = h1.text.partition("|")
    chip_w, chip_h = 44.63, 49.35
    head_h = chip_h + 0.07 * 72 + 1.5
    flow.append({"type": "canvas", "id": "sec-1-head", "h": r2(head_h), "elements": [
        rect(0, 0, chip_w, chip_h, fill="yellow"),
        text(0, 0.1 * 72, chip_w, num.strip(), "sec-n"),
        text(chip_w + GAP, chip_h - 22.54 + 0.2, W - chip_w - GAP, title.strip(), "sec-t"),
        line(0, head_h - 0.75, W, head_h - 0.75, 1.5)]})
    flow.append({"type": "space", "h": 0.2 * 72})
    # the mascot at the head of the margin, level with the header
    fig = next(b for b in blocks if b.kind == "directive" and b.name == "marginfig")
    flow.insert(1, {"type": "margin", "id": "mascot", "blocks": [
        {"type": "figure", "src": "book/" + fig.attrs["src"], "w": r2(float(fig.attrs["w"]) * 72), "align": "left"}]})
    anchors = {}
    last_printed = len(flow) - 1
    inset_of = {}                     # a margin note meets a box at its first line, not its edge: undo the inset
    for i, b in enumerate(blocks):
        key = f"{pg.id}:{i}"
        if b.kind == "h2":
            n, _, t = b.text.partition("|")
            # the number on a yellow chip (padding 0 4pt), 6 pt before the title
            gap_w = 0.02 * 9.2 + 4 + 6 - advance(FILES["Jost Bold"], " ", 9.2) - 0.06 * 9.2
            flow.append({"type": "p", "id": f"h2-{i}", "style": "h2", "content": [
                {"text": n.strip(), "tracking": "0.02em",
                 "mark": {"kind": "highlight", "fill": "yellow", "extent": 4}},
                {"text": " ", "word_spacing": f"{r2(gap_w)}pt"}, t.strip()]})
        elif b.kind == "para":
            flow.append({"type": "p", "id": b.anchor or f"para-{i}", "content": [curl(b.text)]})
            if b.anchor:
                anchors[b.anchor] = len(flow) - 1
            last_printed = len(flow) - 1
        elif b.kind == "directive" and b.name == "slip":
            # margin-top 17 pt (the paragraph's 6.6 pt collapses into it); a `space` block sets the gap exactly
            flow.append({"type": "space", "h": 17})
            flow.append(slip_block(b, key))
            last_printed = len(flow) - 1
            inset_of[id(flow[last_printed])] = 8 + PX
        elif b.kind == "note":
            placement = next((k for k in ("gap", "below", "lined", "free") if k in b.attrs), "margin")
            rot = note_rot(pg.id, i, b.name, b.attrs, placement)
            h = hand(b.text, b.name, key)
            if placement == "gap":
                # a foreman's entry in the column: margin 2 0 3 6 pt (the slip's 12 pt above wins), nearly level
                flow.append({"type": "space", "h": 12})
                flow.append({"type": "box", "id": f"entry-{i}", "w": r2(W), "inset": {"left": 6}, "rotate": r2(rot),
                             "space_before": 12, "space_after": 3, "blocks": hand_blocks(h)})
            else:
                # a margin note: level with its anchor (at=...) or with the block above it, turned about its corner
                nlines = sum(1 for _ in h["paras"]) * 3
                est_h = 128.0
                dx, dy = tl_rotation(0, 0, MARG_W, est_h, rot)
                note = {"type": "margin", "id": f"note-{i}", "dy": r2(-1.5 + dy),
                        "blocks": [{"type": "box", "w": r2(MARG_W), "rotate": r2(rot), "blocks": hand_blocks(h)}]}
                # engine.render_page: a note without at= answers the last printed block above it
                at = b.attrs.get("at")
                pos = anchors[at] + 1 if at and at in anchors else last_printed + 1
                # (the engine sets a note against a box's edge, not its first line)
                flow.insert(pos, note)
                for k_ in anchors:
                    if anchors[k_] >= pos:
                        anchors[k_] += 1
                if last_printed >= pos:
                    last_printed += 1
    return {"type": "section", "id": "section-1", "master": "rules", "start": "recto", "page_from": 1,
            "vars": {"run": "Third Shift Handbook", "rev": pg.attrs.get("rev", "3-57")}, "flow": flow}


def slip_block(b, key: str) -> dict:
    """parts.d_slip: a typed revision slip pasted over the page, turned, held by seeded tape."""
    a = b.attrs
    rng = random.Random(engine.seed_of("slip", key))
    # the same draws in the same order as parts.d_slip (its defaults are evaluated even when unused)
    rot = float(a.get("rot", f"{rng.uniform(-1.1, 1.1):.2f}"))
    kind = a.get("tape", rng.choice(["corners", "corners", "top", "sides", "one"]))
    spots = {"corners": [("l", "t"), ("r", "t")], "top": [("c", "t")], "sides": [("l", "m"), ("r", "m")],
             "one": [(rng.choice("lr"), "t")], "none": []}[kind]
    W = TEXT_W + 6                   # the book hangs it 3 pt out on both sides; here it starts at the measure
    pad = {"top": 8, "right": 10, "bottom": 6, "left": 10}
    tapes = []
    for hx, vy in spots:
        w = round(rng.uniform(0.44, 0.6), 2)
        angs = {"l": rng.uniform(-16, -5), "r": rng.uniform(5, 16), "c": rng.uniform(-5, 5)}
        ang = angs[hx]
        if vy == "m":
            ang = rng.uniform(82, 98)
        poss = {"l": round(rng.uniform(0.04, 0.3), 2), "r": round(rng.uniform(0.04, 0.3), 2)}
        wpt = w * 72 + 2 * PX
        inner_w = W - 2 * PX
        if hx == "c":
            tx = PX + inner_w / 2 - w * 72 / 2
        elif hx == "l":
            tx = PX + poss["l"] * 72
        else:
            tx = W - PX - poss["r"] * 72 - wpt
        ty = PX - 0.09 * 72
        tapes.append(rect(tx - pad["left"] - PX, ty - pad["top"] - PX, wpt, 0.17 * 72 + 2 * PX, fill="tape",
                          stroke_=stroke(PX, "tape-edge"), rotate=round(ang)))
    label = a.get("label", "")
    paras = [p for p in re.split(r"\n\s*\n", b.text.strip()) if p.strip()]
    inner = [{"type": "canvas", "h": 0, "elements": tapes}]
    if label:
        inner.append({"type": "p", "style": "slip-rev", "text": label})
    for p in paras:
        inner.append({"type": "p", "style": "slip", "text": curl(" ".join(ln.strip() for ln in p.splitlines()))})
    return {"type": "box", "id": "slip-366", "w": r2(W), "rotate": rot, "fill": "slip",
            "stroke": stroke(PX, "slip-edge"), "inset": {**pad, "top": pad["top"] + PX, "bottom": pad["bottom"] + PX,
                                                         "left": pad["left"] + PX, "right": pad["right"] + PX},
            "space_before": 17, "space_after": 12, "margin_left": -3, "margin_right": -3, "blocks": inner}


# ------------------------------------------------------------------ page 25: NOTES, ruled
def notes_page(pg) -> dict:
    X, W = RECTO_X, WIDE_W
    head_h = 0.36 * 72
    nt_base = top_for_baseline(FILES["League Gothic"], 23, 0, 23) * -1 + TOP          # NOTES baseline
    sec = next(b for b in pg.blocks if b.kind == "directive" and b.name == "noteshead").attrs.get("sec", "")
    els = [
        text(X, TOP, 100, "Notes", "nh-t"),
        text(X + advance(FILES["League Gothic"], "NOTES", 23, 0.05 * 23) + 10, top_for_baseline(FILES["TeX Gyre Schola"], 9, nt_base, 13.2),
             200, "For notes of your own", "nh-s", set={"line_height": "13.2pt"}),
        text(X + W - 120, top_for_baseline(FILES["Jost"], 6.6, nt_base, 13.2), 120 + 0.14 * 6.6, sec, "nh-sec",
             set={"line_height": "13.2pt"}),
        line(X, TOP + head_h - 0.75, X + W, TOP + head_h - 0.75, 1.5),
    ]
    pitch = 0.29 * 72
    n = int(next(b for b in pg.blocks if b.kind == "directive" and b.name == "lines").attrs.get("n", 22))
    origin = TOP + head_h
    for k in range(1, n + 1):
        y = origin + pitch * k - PX / 2
        els.append(line(X, y, X + W, y, PX, "rule-blue"))
    # Ruth writes on the rules: line height 0.29 in, lowered 0.045 in, 0.06 in in from either side; a skip is one rule
    blocks = []
    pending_skip = 0.0
    for i, b in enumerate(pg.blocks):
        if b.kind == "directive" and b.name == "skip":
            pending_skip += float(b.attrs.get("n", 1)) * pitch
        elif b.kind == "note":
            h = hand(b.text, b.name, f"{pg.id}:{i}")
            bl = hand_blocks(h, {"line_height": f"{r2(pitch)}pt", "space_after": 0})
            if pending_skip:
                bl[0]["set"]["space_before"] = r2(pending_skip)
            pending_skip = 0
            blocks += bl
    els.append(text(X + 0.06 * 72, origin + 0.045 * 72, W - 0.12 * 72, blocks, id="notes-hand"))
    return {"type": "page", "id": "p25-notes", "master": "wide", "page_from": 19, "layers": True,
            "vars": {"run": "Third Shift Handbook", "rev": pg.attrs.get("rev", "3-57")}, "elements": els}


# ------------------------------------------------------------------ page 30: the machine
def machine_page(pg) -> dict:
    X, W = VERSO_X, WIDE_W
    disp = next(b for b in pg.blocks if b.kind == "directive" and b.name == "display")
    mach_i, mach = next((i, b) for i, b in enumerate(pg.blocks) if b.kind == "directive" and b.name == "machine")
    els = [text(X, TOP, W, disp.attrs["text"], "display"),
           line(X, TOP + 30 + 0.07 * 72 + 0.75, X + W, TOP + 30 + 0.07 * 72 + 0.75, 1.5)]
    svg = diagram.machine_svg(pencil="nopencil" not in mach.attrs)
    svg = svg.replace('font-family="Spartan"', 'font-family="Jost"').replace('font-family="Schola"', 'font-family="TeX Gyre Schola"')
    if "xmlns" not in svg[:200]:
        svg = svg.replace("<svg ", '<svg xmlns="http://www.w3.org/2000/svg" ', 1)
    my = TOP + 30 + 0.07 * 72 + 1.5 + 0.2 * 72
    mh = W * 304 / 372
    els.append({"type": "svg", "id": "machine", "x": r2(X), "y": r2(my), "w": r2(W), "h": r2(mh), "fit": "contain",
                "markup": svg})
    # the key: two columns, 0.3 in apart; numbered black discs, lettered yellow squares
    items = [ln.split("|", 1) for ln in mach.text.strip().splitlines() if "|" in ln]
    left_n = (len(items) + 1) // 2
    colw = (W - 0.3 * 72) / 2

    def marker(k):
        k = k.strip()
        if k.isalpha():
            els_ = [rect(0, 0.6, 10.4, 10.4, fill="yellow"), rect(0.3, 0.9, 9.8, 9.8, stroke_=stroke(0.6))]
        else:
            els_ = [{"type": "ellipse", "x": 0, "y": 0.6, "w": 10.4, "h": 10.4, "fill": "ink"}]
        els_.append(text(0, 0.6, 10.4, k, "key-n", set={"color": "paper" if not k.isalpha() else "ink"}))
        return {"blocks": [{"type": "canvas", "w": 10.4, "h": 11.6, "elements": els_}]}

    def label(v, idx):
        runs = []
        for part in re.split(r"(\{\{s:\w+\|[^}]*\}\})", v.strip()):
            m = re.fullmatch(r"\{\{s:(\w+)\|([^}]*)\}\}", part)
            if m:
                who = m.group(1)
                color = PEN.get(engine.HANDS[who]["color"])
                runs.append({"text": m.group(2), "mark": {"kind": "strike", "stroke": stroke(STROKE[who] * PX, color)}})
            elif part:
                runs.append(part)
        return {"blocks": [{"type": "p", "style": "key", "content": runs}]}
    rows = []
    for r in range(left_n):
        row = [marker(items[r][0]), label(items[r][1], r), ""]
        if r + left_n < len(items):
            k2, v2 = items[r + left_n]
            row += [marker(k2), label(v2, r + left_n)]
        else:
            row += ["", ""]
        rows.append(row)
    ky = my + mh + 0.08 * 72
    els.append({"type": "text", "id": "machine-key", "x": r2(X), "y": r2(ky), "w": r2(W), "blocks": [
        {"type": "table", "columns": [16, r2(colw - 16), r2(0.3 * 72), 16, r2(colw - 16)], "stroke": "none",
         "inset": {"top": 0, "bottom": 2.6, "left": 0, "right": 0}, "rows": rows}]})
    key_h = left_n * 11.6 + (left_n - 1) * 2.6
    for i, b in notes_of(pg):
        placement = next((k for k in ("free", "below", "margin") if k in b.attrs), "margin")
        rot = note_rot(pg.id, i, b.name, b.attrs, placement)
        h = hand(b.text, b.name, f"{pg.id}:{i}")
        if placement == "free":
            x, y, w = float(b.attrs["x"]) * 72 + 3.6, float(b.attrs["y"]) * 72, float(b.attrs["w"]) * 72
            px, py = tl_rotation(x, y, w, h["lh"], rot)
            els.append({"type": "text", "id": f"machine-note-{i}", "x": r2(px), "y": r2(py), "w": r2(w), "rotate": r2(rot),
                        "blocks": hand_blocks(h)})
        else:
            # handwriting below the print, the full 4.9 in measure, 16 pt under it, line height 1.2
            h = hand(b.text, b.name, f"{pg.id}:{i}", lh=1.2)
            els.append({"type": "text", "id": f"machine-below-{i}", "x": r2(X), "y": r2(ky + key_h + 16), "w": r2(4.9 * 72),
                        "rotate": r2(rot), "blocks": hand_blocks(h, {"space_after": r2(0.45 * h["size"])})})
    return {"type": "page", "id": "p30-machine", "master": "wide", "page_from": 24, "layers": True,
            "vars": {"run": "Third Shift Handbook", "rev": pg.attrs.get("rev", "3-57")}, "elements": els}


# ------------------------------------------------------------------ page 75: the Notice of Reprimand
def reprimand_page(pg) -> dict:
    """parts.d_form as Matenadar: the pink form is a turned group (−0.7°) holding a text frame whose blocks are the
    form's rows. Spacing between rows is explicit (`space` blocks), as the CSS margins collapse."""
    X, W = RECTO_X, WIDE_W
    fi, form = next((i, b) for i, b in enumerate(pg.blocks) if b.kind == "directive" and b.name == "form")
    key = f"{pg.id}:{fi}"
    a = form.attrs
    pad = {"top": 9, "right": 11, "bottom": 8, "left": 11}
    FW, FH = W, 319.05
    heros = FILES["TeX Gyre Heros"]
    blocks = []
    margins = {"f-co": (0, 0), "f-title": (2, 6), "f-row": (0, 3.4), "f-lab": (4, 0), "f-area": (2, 5),
               "f-check": (0, 2.2), "f-fine": (4, 0)}
    prev_after = None

    def add(kind, block):
        nonlocal prev_after
        before, after = margins[kind]
        if prev_after is not None:
            blocks.append({"type": "space", "h": r2(max(prev_after, before))})
        blocks.append(block)
        prev_after = after

    for n, ln in enumerate(form.text.splitlines()):
        ln = ln.rstrip()
        if not ln.strip():
            continue
        k = f"{key}:{n}"
        if ln.startswith("## "):
            add("f-title", {"type": "p", "style": "f-title", "text": ln[3:]})
        elif ln.startswith("# "):
            add("f-co", {"type": "p", "style": "f-co", "text": ln[2:]})
        elif ln.startswith("~ "):
            add("f-lab", {"type": "p", "style": "f-lab", "text": curl(ln[2:])})
        elif ln.startswith(".. "):
            add("f-fine", {"type": "p", "style": "f-fine", "text": curl(ln[3:])})
        elif ln.startswith(">>"):
            m = re.match(r">>(\w+)(?:\(([^)]*)\))?:\s*(.*)", ln)
            opts = dict(o.split("=") for o in (m.group(2) or "").split() if "=" in o)
            h = hand(m.group(3).replace(" || ", "\n\n"), m.group(1), k, size=float(opts.get("size", 11)))
            hgt = float(opts.get("h", "1.1")) * 72 + 6 + 2 * PX
            add("f-area", {"type": "box", "h": r2(hgt), "stroke": stroke(PX, "area-edge"),
                           "inset": {"top": 3 + PX, "bottom": 3 + PX, "left": 5 + PX, "right": 5 + PX},
                           "blocks": hand_blocks(h, {"space_after": r2(0.25 * h["size"])})})
        elif re.match(r"\[( |x|X)\] ", ln):
            on = ln[1] != " "
            box = [rect(PX / 2, 0.55 + PX / 2, 8 + PX, 8 + PX, stroke_=stroke(PX, "#222222"))]
            if on:
                # layout.js draws the tick from the box's own corners with two seeded numbers (CSS px)
                r = mulberry32(fnv(pg.id + ":0"))
                ql, qt, qw = PX, 0.55 + PX, 8.0
                d_ = (f"M{ql + 1 * PX:.2f} {qt + qw * 0.55:.2f} L{ql + qw * 0.42:.2f} {qt + qw - 0.5 * PX:.2f} "
                      f"L{ql + qw + (3 + r() * 2) * PX:.2f} {qt - (3 + r() * 2) * PX:.2f}")
                box.append({"type": "path", "x": 0, "y": 0, "w": 9.5, "h": 10.6, "d": d_,
                            "stroke": stroke(STROKE["coyle"] * 1.15 * PX, C["ink-black"], cap="round", join="round")})
            box.append(text(9.5 + 5, 0, 300, curl(ln[4:]), "f-check"))
            add("f-check", {"type": "canvas", "h": 10.6, "elements": box})
        elif ":" in ln:
            label, _, value = ln.partition(":")
            m = re.match(r"@(\w+)(?:\(([^)]*)\))?\s+(.*)", value.strip())
            opts = dict(o.split("=") for o in (m.group(2) or "").split() if "=" in o)
            h = hand(m.group(3), m.group(1), k, size=float(opts.get("size", 11)))
            st = dict(h["style"], space_after=0)
            # label | value: the value on a 0.5 pt (1 px) rule, 3 pt in, written 2.5 pt below its line box
            lw = advance(heros, label.upper(), 6.4, 0.07 * 6.4)
            row_h = 15.6
            add("f-row", {"type": "canvas", "h": row_h, "elements": [
                text(0, row_h - 1 - 10.6, lw + 2, label, "f-l"),
                line(lw + 5, row_h - PX / 2, "100%", row_h - PX / 2, PX, "#333333"),
                {"type": "text", "x": r2(lw + 5 + 3), "y": 2.5, "w": 280,
                 "blocks": [{"type": "p", "set": st, "content": h["paras"][0]}]}]})
    frame = {"type": "text", "id": "reprimand-form", "x": 0, "y": 0, "w": r2(FW), "h": r2(FH), "fill": "pink",
             "stroke": stroke(PX, "form-edge"),
             "inset": {k_: v + PX for k_, v in pad.items()}, "blocks": blocks}
    kids = [frame]
    # the serial, typed in red at the top right
    kids.append(text(FW - PX - 9 - 100, PX + 7, 100, a.get("no", ""), "f-no"))
    # two strips of tape over the top edge
    tw, th = 0.55 * 72 + 2 * PX, 0.17 * 72 + 2 * PX
    kids.append(rect(PX + 0.5 * 72, PX - 0.08 * 72, tw, th, fill="tape", stroke_=stroke(PX, "tape-edge"), rotate=-5))
    kids.append(rect(FW - PX - 0.6 * 72 - tw, PX - 0.08 * 72, tw, th, fill="tape", stroke_=stroke(PX, "tape-edge"), rotate=4))
    rot = float(a.get("rot", 0))
    els = [{"type": "group", "id": "reprimand", "x": r2(X), "y": r2(TOP + 0.1 * 72), "w": r2(FW), "h": r2(FH),
            "rotate": rot, "children": kids}]
    # three notes below, the full 4.9 in measure: margin 16 above, 6 below (collapsing to 16 between them)
    below = []
    for i, b in notes_of(pg):
        rotn = note_rot(pg.id, i, b.name, b.attrs, "below")
        h = hand(b.text, b.name, f"{pg.id}:{i}", lh=1.2)
        if below:
            below.append({"type": "space", "h": 16})
        below.append({"type": "box", "w": r2(4.9 * 72), "rotate": r2(rotn),
                      "blocks": hand_blocks(h, {"space_after": r2(0.45 * h["size"])})})
    els.append(text(X, TOP + 0.1 * 72 + FH + 16, 4.9 * 72, below, id="below-notes"))
    # no printed foot: Ruth writes the number back on, at the right end of the foot
    hf = hand("65", "ruth", pg.id + "folio", size=11.5)
    els.append(text(X + W - 60, 598.77, 60, [{"type": "p", "set": {**hh_set(hf), "align": "right"}, "content": hf["paras"][0]}],
                    id="hand-folio"))
    return {"type": "page", "id": "p75-reprimand", "master": "plain", "page_from": 65, "elements": els}


def hh_set(h):
    return dict(h["style"], space_after=0)


def blank(n: int) -> dict:
    return {"type": "page", "id": f"blank-{n}", "master": "plain", "elements": []}


def main() -> None:
    P = pages()
    doc = {
        "matenadar": "1",
        "meta": {"title": "Third Shift Handbook", "author": "Levon Arakelyan", "lang": "en-US"},
        "fonts": ["book/assets/fonts"],
        "design": {**DESIGN, "masters": MASTERS},
        "content": [
            title_page(P["title"]),
            opener_section(P["p001"]),
            blank(4),
            notes_page(P["p019"]),
            machine_page(P["p024"]),
            reprimand_page(P["p065"]),
        ],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
