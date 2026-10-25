#!/usr/bin/env python3
"""Rebuild five pages of "Reverse Coloring for People Who Can't Draw" as a Matenadar document.

    python3 make.py            # writes doc.json (re-runs itself under `uv run --with fonttools` if needed)

Reads only the book (through the `book` symlink, or the path in sample.json):
    manuscript/copy.json      title, subtitle, author, front matter, marks, per-painting nudges and "stuck" ideas
    planning/order.json       painting order (page 8 + 2i faces painting i)
    planning/pages.json       painting -> part (mark)
    planning/tints.json       the four colour tokens of every page
    planning/art-paths.json   the accepted JPEG of every painting
    design/fonts/RC*.ttf      the book's own static font instances (advance widths and vertical metrics)

The geometry is the book's CSS (design/book.css) and design/build/book.py, translated into placed elements: every
CSS box becomes a frame at the same trim coordinates, with its first baseline where Chrome put it.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

try:
    from fontTools.ttLib import TTFont
except ImportError:  # re-run with fontTools available
    if os.environ.get("MT_REEXEC"):
        raise
    os.environ["MT_REEXEC"] = "1"
    sys.exit(subprocess.call(["uv", "run", "--quiet", "--with", "fonttools", "python3", __file__, *sys.argv[1:]]))

HERE = Path(__file__).resolve().parent
SPEC = json.loads((HERE / "sample.json").read_text())
BOOK = HERE / "book" if (HERE / "book").exists() else Path(SPEC["book"])
B = "book/"  # asset paths are relative to the sample root

IN = 72.0
OUTER, GUTTER, TOP, BOTTOM = 0.66, 0.95, 0.72, 0.62   # book.css :root (a verso: outside on the left)
# `balance: true` fails in this engine build ("unknown variable: mt-balanced", see GAPS.md), so make.py balances the
# lines itself: it narrows the frame to the balanced width. Flip this once the engine's own balance works.
ENGINE_BALANCE = False
ORDINALS = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"]


def inch(v: float) -> str:
    return f"{v:.4f}in"


def pt(v: float) -> float:
    return round(v, 3)


# ---- fonts: the book's own files, for the few places where CSS boxes depend on glyph widths ----

class Face:
    def __init__(self, path: Path):
        f = TTFont(path)
        self.upem = f["head"].unitsPerEm
        os2, hhea = f["OS/2"], f["hhea"]
        use_typo = bool(os2.fsSelection & (1 << 7))
        self.asc = (os2.sTypoAscender if use_typo else hhea.ascent) / self.upem
        self.desc = -(os2.sTypoDescender if use_typo else hhea.descent) / self.upem
        self.cmap = f.getBestCmap()
        self.hmtx = f["hmtx"].metrics

    def advance(self, text: str, size: float, tracking_em: float = 0.0) -> float:
        """Width in pt as Chrome sets it: advances plus letter-spacing after every character."""
        w = sum(self.hmtx[self.cmap[ord(c)]][0] for c in text) / self.upem * size
        return w + tracking_em * size * len(text)

    def lines(self, text: str, size: float, tracking_em: float, width: float) -> int:
        """Greedy line count of `text` in `width` pt (no kerning; spaces as in the font)."""
        words, n, cur = text.split(" "), 1, 0.0
        space = self.advance(" ", size, tracking_em)
        for w in words:
            ww = self.advance(w.replace("\u00a0", " "), size, tracking_em)
            if cur and cur + space + ww > width:
                n, cur = n + 1, ww
            else:
                cur = cur + space + ww if cur else ww
        return n

    def balanced(self, text: str, size: float, tracking_em: float, width: float) -> float:
        """CSS text-wrap: balance, done here: the narrowest width that keeps the number of lines."""
        full = self.lines(text, size, tracking_em, width)
        lo, hi = width * 0.3, width
        for _ in range(30):
            mid = (lo + hi) / 2
            if self.lines(text, size, tracking_em, mid) > full:
                lo = mid
            else:
                hi = mid
        return hi + 0.5   # a hair of slack for kerning, which this measure leaves out

    def baseline(self, size: float, lh: float) -> float:
        """Distance from the top of a CSS line box (line-height lh x size) to its baseline, in pt."""
        return (self.asc + (lh - (self.asc + self.desc)) / 2) * size


FONTS = BOOK / "design/fonts"
SERIF, SANS, SANSB = Face(FONTS / "RCSerif.ttf"), Face(FONTS / "RCSans.ttf"), Face(FONTS / "RCSansBold.ttf")


# ---- book.py's text helpers ----

def typog(text: str) -> str:
    text = re.sub(r"(\d) x (\d)", "\\1 \u00d7 \\2", text)
    text = re.sub(r"(?<=\w)'(?=\w)", "\u2019", text)
    text = re.sub(r'"([^"]*)"', "\u201c\\1\u201d", text)
    return text


def bind_a(text: str) -> str:
    # a one-letter word never ends a line: bind it to the word after it
    return re.sub(r"\b([Aa]) ", "\\1\u00a0", text)


def pretty(text: str) -> str:
    """CSS text-wrap: pretty keeps a lone short word off the last line: bind the last two words (see GAPS.md)."""
    head, sep, last = text.rpartition(" ")
    return f"{head}\u00a0{last}" if sep and len(last) <= 8 else text


# ---- the design ----

def design() -> dict:
    serif, sans, sansb = "RCSerif", "RCSans", "RCSansBold"
    return {
        "colors": {"heading": "#26221f", "text": "#3a3531"},
        "base": {"font": sans, "size": 11, "line_height": 1.2, "hyphenate": False, "ligatures": True},
        "styles": {
            # title page
            "title": {"font": serif, "size": 68, "line_height": 0.94, "tracking": "-0.018em", "color": "heading", "balance": ENGINE_BALANCE},
            "subtitle": {"font": sans, "size": 13.2, "line_height": "19pt", "color": "text", "balance": ENGINE_BALANCE},
            "author": {"font": sansb, "size": 8.6, "line_height": 1, "tracking": "0.24em", "case": "upper", "color": "text"},
            # seven small marks
            "marks-h1": {"font": serif, "size": 56, "line_height": 0.96, "tracking": "-0.018em"},
            "marks-intro": {"font": sans, "size": 12, "line_height": "17.6pt"},
            "mark-n": {"font": serif, "size": 19, "line_height": 1},
            "mark-name": {"font": serif, "size": 27, "line_height": 1, "tracking": "-0.01em",
                          "space_before": "0.04in", "space_after": "0.08in"},
            "mark-short": {"font": sans, "size": 10.4, "line_height": "14.8pt"},
            # lesson opener and nudge
            "over": {"font": sansb, "size": 7.7, "line_height": 1, "tracking": "0.2em", "case": "upper"},
            "lesson-h1": {"font": serif, "size": 74, "line_height": 0.92, "tracking": "-0.02em"},
            "teach": {"font": sans, "size": 11.4, "line_height": "16.6pt"},
            "caption": {"font": sansb, "size": 7.2, "line_height": 1, "tracking": "0.18em", "case": "upper"},
            "no": {"font": serif, "size": 50, "line_height": 0.8, "tracking": "-0.01em"},
            "part": {"font": sansb, "size": 7.7, "line_height": 1, "tracking": "0.2em", "case": "upper"},
            "say": {"font": serif, "size": 33, "line_height": 1.12, "tracking": "-0.008em", "balance": ENGINE_BALANCE},
            "stuck": {"font": sans, "size": 9.2, "line_height": 1.45},
            "cutnote": {"font": sans, "size": 6.2, "line_height": 1, "tracking": "0.14em", "case": "upper"},
        },
        "spans": {
            "stuck-head": {"font": sansb, "size": 7.2, "tracking": "0.14em", "case": "upper"},
        },
        "masters": {
            # the trim is 8 x 10 in; bleed on the top, bottom and outside edges, as the book's sheets are
            "sheet": {"width": "8in", "height": "10in", "bleed": "0.125in",
                      "margins": {"top": inch(TOP), "bottom": inch(BOTTOM), "inside": inch(GUTTER), "outside": inch(OUTER)}},
        },
    }


def bal(face: Face, text: str, size: float, tracking_em: float, w: float) -> float:
    """The frame width (in) that sets `text` balanced, as Chrome's text-wrap: balance does."""
    return w if ENGINE_BALANCE else face.balanced(text, size, tracking_em, w * IN) / IN


def frame(x: float, y: float, w: float, style: str, *, text: str | None = None, content: list | None = None,
          color: str | None = None, **extra) -> dict:
    el = {"type": "text", "x": inch(x), "y": inch(y), "w": inch(w), "style": style}
    if text is not None:
        el["text"] = text
    if content is not None:
        el["content"] = content
    if color:
        el["set"] = {"color": color}
    el.update(extra)
    return el


def bleed_image(src: str) -> dict:
    """A painting placed like Bookforge does: the whole sheet, stretched, no aspect lock (recto: bleed at the right)."""
    return {"type": "image", "src": src, "x": 0, "y": "-0.125in", "w": "8.125in", "h": "10.25in", "fit": "stretch",
            "decorative": True}


def label_row(x: float, top: float, number: int, part: str, size: float, tint: dict) -> list:
    """`.label`: a flex row aligned on the baseline, 0.16 in gap: a big serif number and a tracked caps label."""
    no_w = SERIF.advance(str(number), size, -0.01)
    base = SERIF.baseline(size, 0.8)
    part_top = top * IN + base - SANSB.baseline(7.7, 1)
    return [
        frame(x, top, 1.5, "no", text=str(number), color=tint["ink"], set={"size": size, "color": tint["ink"]}),
        {"type": "text", "x": inch(x + (no_w / IN) + 0.16), "y": f"{pt(part_top)}pt", "w": "3in", "style": "part",
         "text": typog(part), "set": {"color": tint["soft"]}},
    ]


def stuck_block(ideas: list[str], tint: dict) -> dict:
    """`.stuck`: the hint paragraph, rotated 180 degrees about its own centre, its bottom 0.62 in above the trim."""
    space = SANS.advance(" ", 9.2)
    content: list = [{"text": "Stuck?", "span": "stuck-head", "color": tint["ink"]},
                     # margin-right .08in after the run-in head, plus the letter-spacing Chrome leaves after its last letter
                     {"text": " ", "word_spacing": f"{pt(0.08 * IN + 0.14 * 7.2 - space)}pt"}]
    for i, idea in enumerate(ideas):
        if i:
            # " <i>·</i> " with .07 in margins either side of the dot
            content.append({"text": " \u00b7 ", "color": tint["mid"], "word_spacing": "0.07in"})
        content.append(typog(bind_a(idea)))
    return {"type": "text", "x": inch(OUTER), "y": inch(10 - BOTTOM), "anchor": "bottom-left", "w": inch(8 - OUTER - GUTTER),
            "rotate": 180, "style": "stuck", "set": {"color": tint["soft"]}, "content": content}


def cut_line(tint: dict) -> list:
    """`.cut` + `.cutnote`: the dashed cut line with scissors (book.py's SVG, 1 unit = 1 in) and the vertical note."""
    svg = ('<svg xmlns="http://www.w3.org/2000/svg" width="0.2in" height="9in" viewBox="0 0 0.2 9">'
           f'<line x1=".1" y1=".42" x2=".1" y2="9" stroke="{tint["mid"]}" stroke-width=".011" stroke-dasharray=".028 .062" stroke-linecap="round"/>'
           f'<g stroke="{tint["soft"]}" fill="none" stroke-width=".012" stroke-linecap="round">'
           '<circle cx=".06" cy=".06" r=".032"/><circle cx=".14" cy=".06" r=".032"/>'
           '<line x1=".075" y1=".088" x2=".128" y2=".33"/><line x1=".125" y1=".088" x2=".072" y2=".33"/></g></svg>')
    # vertical-rl: the line box (6.2 pt wide) has its right edge 0.54 in from the trim, and starts 1.00 in down.
    # A horizontal frame W long, turned 90 degrees clockwise about its centre, lands there.
    W, lh = 7.0 * IN, 6.2
    cx, cy = (8 - 0.54) * IN - lh / 2, 1.0 * IN + W / 2
    return [
        {"type": "svg", "markup": svg, "x": inch(8 - 0.37 - 0.2), "y": "0.5in", "w": "0.2in", "h": "9in", "decorative": True},
        {"type": "text", "x": f"{pt(cx - W / 2)}pt", "y": f"{pt(cy - lh / 2)}pt", "w": f"{pt(W)}pt", "h": f"{lh}pt", "rotate": 90,
         "style": "cutnote", "set": {"color": tint["mid"]}, "text": "Cut here to take out the painting on the other side",
         "decorative": True},
    ]


ARROW = ('<svg xmlns="http://www.w3.org/2000/svg" width="0.3in" height="0.16in" viewBox="0 0 0.3 0.16">'
         '<g stroke="{soft}" fill="none" stroke-width=".012" stroke-linecap="round" stroke-linejoin="round">'
         '<line x1=".01" y1=".08" x2=".28" y2=".08"/><polyline points=".21,.02 .28,.08 .21,.14"/></g></svg>')


def main() -> None:
    copy = json.loads((BOOK / "manuscript/copy.json").read_text())
    spec = json.loads((BOOK / "planning/pages.json").read_text())
    order = json.loads((BOOK / "planning/order.json").read_text())
    tints = json.loads((BOOK / "planning/tints.json").read_text())
    art_paths = json.loads((BOOK / "planning/art-paths.json").read_text())
    part_of = {p["id"]: p["part"] for p in spec["pages"]}
    marks = {m["id"]: m for m in copy["marks"]}
    seven = [m for m in copy["marks"] if m["id"] != "free"]
    front = copy["front"]
    art = lambda pid: B + (art_paths.get(pid) or f"visuals/revisions/{pid}/{pid}-print.jpg")
    content = []

    # ---- page 1: title (recto) ----
    content.append({"type": "page", "id": "p1", "master": "sheet", "page_from": 1, "elements": [
        bleed_image(art("title-wash")),
        frame(0.82, 1.05, bal(SERIF, typog(copy["title"]), 68, -0.018, 8 - 0.82 - 0.70), "title", text=typog(copy["title"]), id="title"),
        frame(0.84, 4.25, bal(SANS, typog(copy["subtitle"]), 13.2, 0, 8 - 0.84 - 1.60), "subtitle", text=typog(copy["subtitle"]), id="subtitle"),
        frame(0.84, 5.50, 4.0, "author", text=typog(copy["author"]), id="author"),
    ]})

    # ---- page 4: seven small marks, rows 1-3 (verso, tinted with the first painting's colours) ----
    t = tints[order[0]]
    els = [
        frame(OUTER, 0.78, 8 - OUTER - GUTTER, "marks-h1", text=typog(front["marks_title"]), color=t["ink"]),
        frame(OUTER, 1.98, 8 - OUTER - GUTTER - 0.9, "marks-intro", text=pretty(typog(front["marks_intro"])), color=t["ink"]),
    ]
    row_h, gap = 1.72, 0.17
    for i, m in enumerate(seven[:3]):
        top = 3.30 + i * (row_h + 0.30)
        x = OUTER
        els.append({"type": "image", "src": f"{B}visuals/examples/{m['id']}-before.jpg", "x": inch(x), "y": inch(top),
                    "w": inch(row_h), "h": inch(row_h), "fit": "stretch"})
        x += row_h + gap
        els.append({"type": "svg", "markup": ARROW.format(soft=t["soft"]), "x": inch(x), "y": inch(top + row_h / 2 - 0.08),
                    "w": "0.26in", "h": "0.16in", "fit": "contain"})
        x += 0.26 + gap
        els.append({"type": "image", "src": f"{B}visuals/examples/{m['id']}-after.jpg", "x": inch(x), "y": inch(top),
                    "w": inch(row_h), "h": inch(row_h), "fit": "stretch"})
        x += row_h + gap + 0.08
        els.append({"type": "text", "x": inch(x), "y": inch(top), "w": inch(8 - GUTTER - x), "h": inch(row_h),
                    "valign": "middle", "set": {"color": t["ink"]}, "blocks": [
                        {"type": "p", "style": "mark-n", "text": str(i + 1), "set": {"color": t["mid"]}},
                        {"type": "p", "style": "mark-name", "text": typog(m["name"])},
                        {"type": "p", "style": "mark-short", "text": pretty(typog(m["short"]))},
                    ]})
    content.append({"type": "page", "id": "p4", "master": "sheet", "start": "verso", "page_from": 4, "fill": t["tint"],
                    "elements": els})

    # ---- page 8: the lesson opener for the first mark (verso; faces painting 1) ----
    pid = order[0]
    t, part = tints[pid], part_of[pid]
    m, entry = marks[part], copy["pages"][pid]
    content.append({"type": "page", "id": "p8", "master": "sheet", "start": "verso", "page_from": 8, "fill": t["tint"], "elements": [
        frame(OUTER, TOP, 4, "over", text=f"The {ORDINALS[0]} mark", color=t["soft"]),
        frame(OUTER - 0.03, 0.98, 8 - OUTER - GUTTER, "lesson-h1", text=typog(m["name"]), color=t["ink"]),
        frame(OUTER, 2.42, 3.02, "teach", text=pretty(typog(m["lesson"])), color=t["ink"], id="teach"),
        {"type": "image", "src": f"{B}visuals/examples/{part}-after-full.jpg", "x": inch(8 - GUTTER - 2.88), "y": "2.47in",
         "w": "2.88in", "h": "3.6in", "fit": "stretch"},
        frame(8 - GUTTER - 2.88, 2.47 + 3.6 + 0.10, 2.88, "caption", text="One way to use it", color=t["soft"]),
        # .rule: a 0.8 pt top border at 6.72 in
        {"type": "line", "x1": inch(OUTER), "y1": f"{pt(6.72 * IN + 0.4)}pt", "x2": inch(8 - GUTTER), "y2": f"{pt(6.72 * IN + 0.4)}pt",
         "stroke": {"width": 0.8, "color": t["mid"]}},
        *label_row(OUTER, 7.02, 1, m["name"], 34, t),
        frame(OUTER, 7.66, bal(SERIF, typog(entry["nudge"]), 21.5, -0.008, 8 - OUTER - GUTTER), "say", text=typog(entry["nudge"]), color=t["ink"],
              set={"size": 21.5, "line_height": 1.16, "color": t["ink"]}),
        stuck_block(entry["stuck"], t),
    ]})

    # ---- page 9: painting 1 (recto): one JPEG, the whole sheet ----
    content.append({"type": "page", "id": "p9", "master": "sheet", "elements": [bleed_image(art(pid))]})

    # ---- page 10: the nudge facing painting 2 (verso) ----
    pid = order[1]
    t, part = tints[pid], part_of[pid]
    m, entry = marks[part], copy["pages"][pid]
    nudge = entry["nudge"]
    content.append({"type": "page", "id": "p10", "master": "sheet", "fill": t["tint"], "elements": [
        *label_row(OUTER, TOP, 2, m["name"], 50, t),
        frame(OUTER, 3.05, bal(SERIF, typog(nudge), 29 if len(nudge) > 86 else 33, -0.008, 8 - OUTER - GUTTER - 0.5), "say", text=typog(nudge), color=t["ink"],
              **({"set": {"size": 29, "color": t["ink"]}} if len(nudge) > 86 else {})),
        stuck_block(entry["stuck"], t),
        *cut_line(t),
    ]})

    doc = {
        "matenadar": "1",
        "meta": {"title": typog(copy["title"]), "author": copy["author"], "lang": "en-US"},
        "fonts": ["book/design/fonts"],
        "design": design(),
        "content": content,
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print("wrote", HERE / "doc.json")


if __name__ == "__main__":
    main()
