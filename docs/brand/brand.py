#!/usr/bin/env python3
"""The README's banner and logo, typeset by Matenadar itself.

Writes one Matenadar document per drawing and theme, then lays each out to
SVG with the engine, so the letters arrive as outlines and GitHub needs no
fonts. Run from the repository root after `cargo build`:

    python3 docs/brand/brand.py [--bin target/debug/matenadar]
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent

THEMES = {
    "light": {
        "ground": "#f4ede1", "ink": "#1d1914", "rubric": "#b5321f", "muted": "#7a6e5f",
        "page": "#fffaf2", "shade": "#1d191414", "guide": "#b5321f66", "line": "#1d19142e", "tile_ink": "#fbf3e6",
    },
    "dark": {
        "ground": "#15120e", "ink": "#efe6d6", "rubric": "#e0593b", "muted": "#958a7a",
        "page": "#231e18", "shade": "#00000059", "guide": "#e0593b80", "line": "#efe6d633", "tile_ink": "#fbf3e6",
    },
}

W, H = 960, 300  # points: 1280 × 400 px


def tile(x, y, size, c):
    """The mark: Men (Մ), Matenadar's initial in Armenian, on a rubricated leaf."""
    inset = size * 0.055
    return [
        {"type": "rect", "x": x, "y": y, "w": size, "h": size, "fill": c["rubric"], "radius": size * 0.04},
        {"type": "rect", "x": x + inset, "y": y + inset, "w": size - 2 * inset, "h": size - 2 * inset,
         "stroke": {"width": size * 0.006, "color": c["tile_ink"]}, "radius": size * 0.02, "opacity": 0.55},
        {"type": "line", "x1": x + inset, "y1": y + size * 0.815, "x2": x + size - inset, "y2": y + size * 0.815,
         "stroke": {"width": size * 0.004, "color": c["tile_ink"], "dash": [2, 2]}, "opacity": 0.5},
        {"type": "lettering", "text": "Մ", "font": "Noto Sans Armenian", "weight": "bold", "size": size * 0.86,
         "color": c["tile_ink"], "x": x, "w": size, "align": "center", "y": y - size * 0.105, "decorative": True},
    ]


def bars(x, y, w, rows, step, c, short=None):
    """Lines of set text, as a layout report sees them."""
    out = []
    for i in range(rows):
        width = w * (short if (short and i == rows - 1) else 1)
        out.append({"type": "rect", "x": x, "y": y + i * step, "w": width, "h": step * 0.42, "fill": c["line"], "radius": 0.6})
    return out


def spread(x, y, c):
    """A spread with its guides showing, one node picked out as `locate` finds it."""
    pw, ph, gap = 124, 172, 6
    m = {"top": 16, "bottom": 20, "inside": 12, "outside": 18}
    guide = {"width": 0.5, "color": c["guide"], "dash": [2, 1.6]}
    out = [
        {"type": "rect", "x": x + 3, "y": y + 4, "w": 2 * pw + gap, "h": ph, "fill": c["shade"], "radius": 2},
    ]
    for side in (0, 1):
        px = x + side * (pw + gap)
        inner_l = m["outside"] if side == 0 else m["inside"]
        inner_r = m["inside"] if side == 0 else m["outside"]
        fx, fy, fw, fh = px + inner_l, y + m["top"], pw - inner_l - inner_r, ph - m["top"] - m["bottom"]
        out += [
            {"type": "rect", "x": px, "y": y, "w": pw, "h": ph, "fill": c["page"], "radius": 1.5},
            {"type": "rect", "x": fx, "y": fy, "w": fw, "h": fh, "stroke": guide},
            {"type": "text", "x": px, "y": y + ph - 12, "w": pw, "text": str(2 * 21 + side),
             "set": {"font": "JetBrains Mono", "size": 4.6, "color": c["muted"], "align": "center"}},
        ]
        if side == 0:
            out += [
                {"type": "text", "x": fx, "y": fy + 4, "w": fw, "text": "CHAPTER THREE",
                 "set": {"font": "JetBrains Mono", "size": 4.2, "tracking": "0.2em", "color": c["rubric"], "align": "center"}},
                {"type": "lettering", "text": "The Colophon", "font": "Libertinus Serif", "size": 11, "color": c["ink"],
                 "x": fx, "w": fw, "align": "center", "y": fy + 12, "decorative": True},
                {"type": "lettering", "text": "T", "font": "Libertinus Serif", "size": 27, "color": c["rubric"],
                 "x": fx - 1, "y": fy + 31, "decorative": True},
            ]
            out += bars(fx + 17, fy + 39, fw - 17, 3, 7, c)
            out += bars(fx, fy + 60, fw, 10, 7, c, short=0.55)
        else:
            out += [
                {"type": "rect", "x": fx, "y": fy + 2, "w": fw, "h": 52, "fill": c["line"], "radius": 1},
                {"type": "path", "x": fx, "y": fy + 2, "w": fw, "h": 52, "viewbox": [0, 0, 100, 52],
                 "d": "M 0 52 L 30 22 L 48 38 L 66 16 L 100 46 L 100 52 Z", "fill": c["line"]},
                {"type": "ellipse", "x": fx + fw - 22, "y": fy + 9, "w": 9, "h": 9, "fill": c["page"], "opacity": 0.8},
            ]
            out += bars(fx, fy + 62, fw, 4, 7, c)
            # The node an agent asked about, with what `layout` reports for it.
            sx, sy, sw, sh = fx - 2.5, fy + 88, fw + 5, 25
            out += bars(fx, fy + 91.5, fw, 3, 7, c, short=0.7)
            out += [
                {"type": "rect", "x": sx, "y": sy, "w": sw, "h": sh, "stroke": {"width": 0.8, "color": c["rubric"]}, "radius": 1.2},
                {"type": "text", "x": sx, "y": sy - 9.5, "text": "flow[14] · p.43 · 88×25pt", "inset": {"x": 3, "y": 1.6},
                 "fill": c["rubric"], "radius": 1.2,
                 "set": {"font": "JetBrains Mono", "size": 4.4, "color": c["tile_ink"], "line_height": 1}},
                {"type": "path", "x": sx + sw - 16, "y": sy + sh - 6, "w": 11, "h": 15,
                 "d": "M 0 0 L 0 12.5 L 3.2 9.6 L 5.4 14.6 L 7.4 13.7 L 5.3 8.9 L 9.6 8.6 Z",
                 "fill": c["ink"], "stroke": {"width": 0.7, "color": c["page"], "join": "round"}},
            ]
            out += bars(fx, fy + 120, fw, 2, 7, c, short=0.4)
    return out


def banner(c):
    mono = {"font": "JetBrains Mono", "color": c["muted"]}
    left = 236
    elements = [
        *tile(48, 66, 168, c),
        {"type": "lettering", "text": "Matenadar", "font": "Libertinus Serif", "size": 82, "color": c["ink"],
         "tracking": "-0.01em", "x": left - 4, "y": 74, "decorative": True},
        {"type": "line", "x1": left, "y1": 172, "x2": left + 40, "y2": 172, "stroke": {"width": 1.6, "color": c["rubric"]}},
        {"type": "text", "x": left, "y": 184, "w": 380, "text": "A book and page layout engine\nfor people and agents",
         "set": {"font": "Libertinus Serif", "italic": True, "size": 17, "line_height": 1.25, "color": c["ink"]}},
        {"type": "text", "x": left, "y": 232, "w": 400, "text": "JSON MODEL  →  TYPST  →  PDF · SVG · PNG · EPUB · WASM",
         "set": {**mono, "size": 7.2, "tracking": "0.14em"}},
        *spread(660, 52, c),
    ]
    return {
        "matenadar": "1",
        "meta": {"title": "Matenadar", "author": "Matenadar", "lang": "en-US"},
        "design": {"masters": {"art": {"width": W, "height": H, "margins": {"top": 0, "bottom": 0, "left": 0, "right": 0}}}},
        "content": [{"type": "page", "id": "banner", "master": "art", "fill": c["ground"], "elements": elements}],
    }


def logo(c):
    size = 256
    return {
        "matenadar": "1",
        "meta": {"title": "Matenadar", "author": "Matenadar", "lang": "en-US"},
        "design": {"masters": {"art": {"width": size, "height": size, "margins": {"top": 0, "bottom": 0, "left": 0, "right": 0}}}},
        "content": [{"type": "page", "id": "logo", "master": "art", "fill": "none", "elements": tile(0, 0, size, c)}],
    }


def build(binary, name, doc):
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "doc.json"
        src.write_text(json.dumps(doc, ensure_ascii=False, indent=1))
        out = Path(tmp) / "svg"
        subprocess.run([binary, "build", str(src), "-o", str(Path(tmp) / "out.pdf"), "--svg", str(out), "--check"], check=True)
        pages = sorted(out.glob("*.svg"))
        shutil.copy(pages[0], HERE / f"{name}.svg")
        print(f"docs/brand/{name}.svg")


def main():
    binary = sys.argv[sys.argv.index("--bin") + 1] if "--bin" in sys.argv else "target/debug/matenadar"
    for theme, colors in THEMES.items():
        build(binary, f"banner-{theme}", banner(colors))
    build(binary, "logo", logo(THEMES["light"]))


if __name__ == "__main__":
    main()
