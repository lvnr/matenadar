#!/usr/bin/env python3
"""Rebuild five pages of "Christmas Word Search: Hidden Messages & Festive Facts" as a Matenadar document.

    python3 make.py            # writes doc.json (re-runs itself under `uv run --with fonttools --with uharfbuzz` if needed)

Reads only the book (through the `book` symlink, or the path in sample.json):
    planning/content-v2.json       chapters, puzzle titles, words, levels, types, rewards (messages and facts)
    puzzles/v2/layouts.json        every grid and every word's placement
    design/build/pages.py          the book's geometry constants and layout rules (imported read-only, no bytecode)
    design/build/frontmatter.py    the How to Solve text and the worked example (read with `ast`, not run)
    design/build/make_book.py      chapter intros and knit motifs (read with `ast`)
    design/art/knit/*.svg          the knit bands and motifs (the book's own vector art)
    design/art/plates/*.svg        the potrace chapter plates
    /Library/Fonts                 Chaparral Pro and Source Sans Pro, for widths and vertical metrics

The book is TikZ on an absolute canvas: every node is placed by its baseline or its top. make.py turns each node into a
Matenadar frame whose first baseline lands where TeX put it (TeX's box heights come from the glyphs' ink, which
HarfBuzz reports), and leaves the line breaking, kerning, ligatures and leaders to the engine.
"""
from __future__ import annotations

import ast
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

try:
    import uharfbuzz as hb
    from fontTools.ttLib import TTFont
except ImportError:  # re-run with the measuring libraries available
    if os.environ.get("MT_REEXEC"):
        raise
    os.environ["MT_REEXEC"] = "1"
    sys.exit(subprocess.call(["uv", "run", "--quiet", "--with", "fonttools", "--with", "uharfbuzz", "python3", __file__, *sys.argv[1:]]))

HERE = Path(__file__).resolve().parent
SPEC = json.loads((HERE / "sample.json").read_text())
BOOK = HERE / "book" if (HERE / "book").exists() else Path(SPEC["book"])
B = "book/"

sys.dont_write_bytecode = True          # importing the book's module must not write into the book
sys.path.insert(0, str(BOOK / "design/build"))
import pages as P  # noqa: E402  (the book's geometry: BLOCK_W, CELL, GRID_TOP, drops, blanks_layout, LEVEL_DIRS, ...)

IN = 72.0
TP = 72 / 72.27      # TeX's pt: every size, leading and skip in the book is in TeX points, a little under 1/72 in
PAGE_W, PAGE_H = P.PAGE_W, P.PAGE_H
X_RECTO, X_VERSO = P.INSIDE_IN, P.OUTSIDE_IN   # block origin: gutter on the left of a recto
GRAY_SOFT, GRAY_LOOP = "#6b6b6b", "#cccccc"     # DeviceGray 0.42 and 0.80


def pt(v: float) -> str:
    return f"{v:.3f}pt"


# ---- fonts: HarfBuzz widths and ink, and the vertical metrics the engine uses ----

class Face:
    def __init__(self, file: str, family: str, weight: int, italic: bool, features: list[str]):
        path = Path("/Library/Fonts") / file
        self.family, self.weight, self.italic, self.features = family, weight, italic, features
        self.face = hb.Face(hb.Blob.from_file_path(str(path)))
        self.font = hb.Font(self.face)
        self.upem = self.face.upem
        f = TTFont(path)
        os2, hhea = f["OS/2"], f["hhea"]
        typo = bool(os2.fsSelection & (1 << 7))       # as ttf-parser reads them
        self.asc = (os2.sTypoAscender if typo else hhea.ascent) / self.upem
        self.desc = -(os2.sTypoDescender if typo else hhea.descent) / self.upem
        self.x_height = os2.sxHeight / self.upem

    def _shape(self, text: str):
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(self.font, buf, {f: True for f in ["kern", "liga", *self.features]})
        return buf

    def width(self, text: str, size: float, tracking_em: float = 0.0) -> float:
        buf = self._shape(text)
        return sum(p.x_advance for p in buf.glyph_positions) / self.upem * size + tracking_em * size * len(buf.glyph_infos)

    def ink(self, text: str, size: float) -> tuple[float, float]:
        """TeX's box height and depth of a line: the highest and lowest ink of its glyphs, in pt."""
        top, bottom = 0.0, 0.0
        for info in self._shape(text).glyph_infos:
            e = self.font.get_glyph_extents(info.codepoint)
            if e is None:
                continue
            top, bottom = max(top, e.y_bearing), min(bottom, e.y_bearing + e.height)
        return top / self.upem * size, -bottom / self.upem * size


LNUM, SANSNUM = ["lnum"], ["lnum", "pnum"]
FACES = {
    "CH": Face("ChaparralPro-Regular.otf", "Chaparral Pro", 400, False, LNUM),
    "CHB": Face("ChaparralPro-Bold.otf", "Chaparral Pro", 700, False, LNUM),
    "CHI": Face("ChaparralPro-Italic.otf", "Chaparral Pro", 400, True, LNUM),
    "SS": Face("SourceSansPro-Regular.otf", "Source Sans Pro", 400, False, SANSNUM),
    "SSS": Face("SourceSansPro-Semibold.otf", "Source Sans Pro", 600, False, SANSNUM),
    "SSB": Face("SourceSansPro-Bold.otf", "Source Sans Pro", 700, False, SANSNUM),
}
LM_HALF_EX = 4.30554 / 2 * TP   # TikZ `anchor=mid` is 0.5 ex of the node font, \normalfont (Latin Modern 10 pt) here
# \caps: \MakeUppercase plus fontspec LetterSpace=9. XeTeX spaces the letters but not the interword spaces; the
# engine's tracking widens spaces too, so take it back off them
CAPS = {"case": "upper", "tracking": "0.09em", "word_spacing": "-0.09em"}

# name: (face, size, baselineskip, extra)
STYLES = {
    "head": ("CHB", 31, 33, {}),
    "step-head": ("CHB", 16, 19, {}),
    "step-body": ("CH", 13.2, 17.6, {}),
    "row-label": ("SSS", 11.6, 14, {}),
    "row-note": ("CHI", 12.4, 14, {}),
    "label": ("SSS", 9.4, 10, CAPS),
    "label-ex": ("SSS", 8.6, 9, CAPS),
    "example": ("CHI", 12.6, 17, {}),
    "folio": ("CHB", 11.5, 12, {}),
    "foot": ("SS", 7.6, 8, CAPS),
    "foot-ref": ("CHI", 12, 13, {}),
    "chapter-label": ("SSS", 11, 11, {**CAPS, "align": "center"}),
    "chapter-title": ("CHB", 35, 40, {"align": "center"}),
    "intro": ("CHI", 14.2, 19.6, {"align": "center"}),
    "toc-num": ("SSS", 11.5, 12, {"align": "right"}),
    "toc-title": ("CH", 12.2, 13, {}),
    "key": ("CHI", 12.4, 13, {"align": "right"}),
    "puzzle-title": ("CHB", 21.5, 21.5, {}),
    "badge": ("SSB", 12.5, 12.5, {"color": "#ffffff", "align": "center"}),
    "bank": ("SS", 14, 14, {}),
    "message-note": ("CHI", 11.4, 12, {"align": "right"}),
    "fact": ("CH", 13.2, 16, {}),
    "sol-title": ("CHB", 13.4, 15.6, {}),
    "sol-ref": ("CHI", 11.6, 14, {}),
    "sol-label": ("SSS", 8.4, 10, CAPS),
    "sol-message": ("CHI", 12.6, 16, {}),
    "sol-badge": ("SSB", 11, 11, {"color": "#ffffff", "align": "center"}),
}


def face_of(style: str) -> Face:
    return FACES[STYLES[style][0]]


def size_of(style: str) -> float:
    return STYLES[style][1] * TP


def first_baseline(style: str) -> float:
    """Where the engine puts the first baseline below a frame's top (CSS line box: half the leading above)."""
    f, size, lh, _ = STYLES[style]
    face, size, lh = FACES[f], size * TP, lh * TP
    return (face.asc + (lh / size - (face.asc + face.desc)) / 2) * size


def design() -> dict:
    styles = {}
    for name, (f, size, lh, extra) in STYLES.items():
        face = FACES[f]
        s = {"font": face.family, "weight": face.weight, "size": round(size * TP, 4), "line_height": pt(lh * TP), "features": face.features}
        if face.italic:
            s["italic"] = True
        s.update(extra)
        styles[name] = s
    # TikZ nodes with \setpar but no node font take 10 pt Latin Modern's interword space (3.33 pt) for every font in them
    for name in ("sol-title", "sol-ref", "sol-message"):
        styles[name]["word_spacing"] = pt(lm_extra(name))
    styles["sol-label"]["word_spacing"] = pt(lm_extra("sol-label") - 0.09 * size_of("sol-label"))
    folio_w = 1.0
    return {
        "colors": {"ink": "#000000", "soft": GRAY_SOFT, "loop": GRAY_LOOP},
        "base": {"font": "Chaparral Pro", "size": 12, "color": "ink", "hyphenate": False},
        "styles": styles,
        "spans": {
            "run-in": {"font": "Source Sans Pro", "weight": 600, "size": 9.4, **CAPS},
            # the leader dots: a Chaparral period sized and lowered to TikZ's 1.15 pt round dots centred on the baseline
            "dot": dot_span(),
        },
        "masters": {
            "page": {
                "width": "8in", "height": "10in",
                "margins": {"top": "0.41in", "bottom": "0.5in", "inside": f"{X_RECTO}in", "outside": f"{X_VERSO:.4f}in"},
                # the running foot (pages.footer): folio outside, chapter title centred; pages opt in with "layers"
                "foreground": [
                    foot_text(X_VERSO, "folio", [{"field": "page"}], "w", folio_w, when="verso"),
                    foot_text(X_RECTO + P.BLOCK_W, "folio", [{"field": "page"}], "e", folio_w, when="recto"),
                    foot_text(X_VERSO + P.BLOCK_W / 2, "foot", [{"field": "section.title"}], "c", 4.0, when="verso"),
                    foot_text(X_RECTO + P.BLOCK_W / 2, "foot", [{"field": "section.title"}], "c", 4.0, when="recto"),
                ],
            },
        },
    }


def dot_span() -> dict:
    face = FACES["CH"]
    top, depth = face.ink(".", 1.0)          # the period's ink, per pt of size
    size = 1.15 * TP / (top + depth)
    return {"font": "Chaparral Pro", "weight": 400, "size": round(size, 3), "baseline": pt(-(top - depth) / 2 * size)}


def foot_text(x_abs: float, style: str, content: list, anchor: str, w: float, when: str) -> dict:
    el = place_text(x_abs, PAGE_H - P.FOOT_Y, style, content=content, anchor=anchor, w=w)
    el["when"] = when
    return el


# ---- placing TeX nodes ----

OVERHANG = {".": 0.8, ",": 0.8, ":": 0.3, ";": 0.3}   # Typst's hanging punctuation, as fractions of the glyph


def place_text(x_abs: float, y_base_from_top: float, style: str, *, text: str | None = None, content: list | None = None,
               anchor: str = "w", w: float = 6.0, **extra) -> dict:
    """A frame whose first baseline is at (x, y) in inches from the page's top-left; anchor w / e / c, as TikZ's base."""
    left = {"w": x_abs, "e": x_abs - w, "c": x_abs - w / 2}[anchor]
    if text and anchor in "ec" and text[-1] in OVERHANG:
        # the engine lets end punctuation hang into the margin; TeX's single-line nodes don't
        hang = OVERHANG[text[-1]] * face_of(style).width(text[-1], size_of(style)) / IN
        left -= hang if anchor == "e" else hang / 2
    el = {"type": "text", "x": f"{left:.4f}in", "y": pt(y_base_from_top * IN - first_baseline(style)), "w": f"{w:.4f}in", "style": style}
    if anchor == "e":
        el["set"] = {"align": "right"}
    elif anchor == "c":
        el["set"] = {"align": "center"}
    if text is not None:
        el["text"] = text
    if content is not None:
        el["content"] = content
    el.update(extra)
    return el


class Canvas:
    """One page in the book's block coordinates: inches from the block's bottom-left, y up."""

    def __init__(self, recto: bool):
        self.x0 = X_RECTO if recto else X_VERSO
        self.els: list[dict] = []

    def X(self, bx: float) -> float:
        return self.x0 + bx

    def Y(self, by: float) -> float:
        return PAGE_H - by

    def base(self, bx: float, by: float, style: str, text: str | None = None, *, anchor: str = "w", w: float = 6.0, **kw) -> None:
        self.els.append(place_text(self.X(bx), self.Y(by), style, text=text, anchor=anchor, w=w, **kw))

    def north(self, bx: float, by: float, width: float, style: str, text: str, *, anchor: str = "nw",
              space: tuple[float, float, float] | None = None, head: list | None = None) -> list[str]:
        """A TikZ node with `text width`, anchored at its top: TeX puts the first baseline one ink height down.
        `head` is a run-in that goes before the text (it is measured as part of the first line)."""
        em = size_of(style)
        lines = tex_set(text, style, width * IN, space=space or setpar(style), skip=2 * em, centred=anchor == "n")
        h1, _ = face_of(style).ink(lines[0][0], size_of(style))
        slack = 6 / IN if TEX_BREAKS else 0          # room for TeX's hanging punctuation and shrunk spaces
        left = bx - width / 2 - slack / 2 if anchor == "n" else bx
        content = (head or []) + joined(lines)
        self.els.append(place_text(self.X(left), self.Y(by) + h1 / IN, style, content=content, anchor="w", w=width + slack))
        return [line for line, _ in lines]

    def line(self, a: tuple[float, float], b: tuple[float, float], width: float = 0.8) -> None:
        self.els.append({"type": "line", "x1": f"{self.X(a[0]):.4f}in", "y1": f"{self.Y(a[1]):.4f}in", "x2": f"{self.X(b[0]):.4f}in",
                         "y2": f"{self.Y(b[1]):.4f}in", "stroke": {"width": round(width * TP, 3), "color": "ink"}})

    def rect(self, a: tuple[float, float], b: tuple[float, float], width: float, radius: float | None = None) -> None:
        (x1, y1), (x2, y2) = a, b
        el = {"type": "rect", "x": f"{self.X(x1):.4f}in", "y": f"{self.Y(y2):.4f}in", "w": f"{x2 - x1:.4f}in", "h": f"{y2 - y1:.4f}in",
              "stroke": {"width": round(width * TP, 3), "color": "ink"}}
        if radius:
            el["radius"] = pt(radius * TP)
        self.els.append(el)

    def disc(self, bx: float, by: float, r: float) -> None:
        self.els.append({"type": "ellipse", "x": f"{self.X(bx - r):.4f}in", "y": f"{self.Y(by + r):.4f}in", "w": f"{2 * r}in",
                         "h": f"{2 * r}in", "fill": "ink"})

    def badge(self, bx: float, by: float, r: float, number: int, style: str) -> None:
        """A black disc with a white number set at TikZ's `mid` anchor (baseline 0.5 ex below the centre)."""
        self.disc(bx, by, r)
        self.base(bx, by - LM_HALF_EX / IN, style, str(number), anchor="c", w=1.0)

    def svg(self, src: str, bx: float, by_top: float, w: float, h: float, fit: str = "stretch") -> None:
        self.els.append({"type": "svg", "src": src, "x": f"{self.X(bx):.4f}in", "y": f"{self.Y(by_top):.4f}in",
                         "w": f"{w:.4f}in", "h": f"{h:.4f}in", "fit": fit, "decorative": True})

    def band(self, chapter: int, by: float = P.TOP_BAND_Y) -> None:
        self.svg(f"{B}design/art/knit/band-{chapter:02d}.svg", 0, by + P.BAND_H, P.BLOCK_W, P.BAND_H)

    def arrow(self, a: tuple[float, float], b: tuple[float, float]) -> None:
        """TikZ `line width=1.05pt, line cap=round, -{Stealth[length=4.2pt, width=4.2pt, line width=0.8pt]}`."""
        ax, ay, bx, by = self.X(a[0]) * IN, self.Y(a[1]) * IN, self.X(b[0]) * IN, self.Y(b[1]) * IN
        n = math.hypot(bx - ax, by - ay)
        ux, uy = (bx - ax) / n, (by - ay) / n
        vx, vy = -uy, ux
        # pgflibraryarrows.meta's Stealth, transcribed: the tip's outline is stroked (miter join) inside length x width
        L, W, inset = 4.2, 4.2, 0.325 * 4.2
        lw = min(0.8, 0.25 * (L - inset))                      # line width capped at 1/4 of inset-to-tip
        xc = 0.5 * lw * math.sqrt(1 + 4 * (L / W) ** 2)          # front miter
        a_in = math.atan2(inset, W / 2)
        half = (math.atan2(L, W / 2) - a_in) / 2
        yc = 0.5 * lw / math.tan(half)
        ya, yb = math.sin(a_in + half) * yc, math.cos(a_in + half) * yc   # back and top miters
        inset_m = 0.5 * lw * math.sqrt(1 + 4 * (inset / W) ** 2)
        inner_l, inner_w = L - xc - ya, W / 2 - yb
        at = lambda x, y: (bx - ux * (L - x) * TP + vx * y * TP, by - uy * (L - x) * TP + vy * y * TP)   # arrow coords, back at 0
        pts = [at(inner_l + ya, 0), at(ya, inner_w), at(inset + inset_m, 0), at(ya, -inner_w)]
        notch = at(inset + inset_m - 0.25 * lw, 0)                 # where the shaft stops
        lw *= TP
        xs = [ax, bx] + [p[0] for p in pts]
        ys = [ay, by] + [p[1] for p in pts]
        ox, oy = min(xs) - 2, min(ys) - 2
        w, h = max(xs) + 2 - ox, max(ys) + 2 - oy
        f = lambda x, y: f"{x - ox:.3f} {y - oy:.3f}"
        head = "M " + " L ".join(f(*p) for p in pts) + " Z"
        svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w:.3f}pt" height="{h:.3f}pt" viewBox="0 0 {w:.3f} {h:.3f}">'
               f'<path d="M {f(ax, ay)} L {f(*notch)}" fill="none" stroke="#000" stroke-width="{1.05 * TP:.3f}" stroke-linecap="round"/>'
               f'<path d="{head}" fill="#000" stroke="#000" stroke-width="{lw}" stroke-linejoin="miter"/></svg>')
        self.els.append({"type": "svg", "markup": svg, "x": pt(ox), "y": pt(oy), "w": pt(w), "h": pt(h)})

    def arrows(self, x: float, y: float, dirs: list[str], size: float, gap: float) -> float:
        """pages.arrows: a direction key drawn left to right from (x, y); returns its width."""
        cx = x
        for d in dirs:
            dx, dy = P.DIR_VEC[d]
            n = math.hypot(dx, dy)
            hx, hy = dx / n * size / 2, dy / n * size / 2
            mx = cx + size / 2
            self.arrow((mx - hx, y - hy), (mx + hx, y + hy))
            cx += size + gap
        return cx - gap - x


# TeX's paragraph builder, for the few multi-line nodes. TikZ `text width` nodes are ragged with finite glue
# (\rightskip 0pt plus 2em, and \leftskip too when centred), so TeX evens their lines out instead of filling them
# greedily; \raggedright parboxes have fil glue but may shrink their spaces; microtype lets end punctuation hang.
# The engine can do neither (see GAPS.md), so make.py finds TeX's breaks and hands the engine explicit line breaks.
TEX_BREAKS = os.environ.get("MT_NATIVE_BREAKS") != "1"    # MT_NATIVE_BREAKS=1 writes build/doc-native.json instead
PROTRUDE = {".": 0.7, ",": 0.5, ":": 0.5, ";": 0.5, "-": 0.7}


def tex_lines(text: str, style: str, width: float, **kw) -> list[str]:
    return [line for line, _ in tex_set(text, style, width, **kw)]


def tex_set(text: str, style: str, width: float, *, space: tuple[float, float, float] | None = None,
            skip: float | None = None, centred: bool = False) -> list[tuple[str, float]]:
    r"""Knuth-Plass over word boundaries. `space` is (natural, stretch, shrink) of the interword glue (default: the
    font's space with XeTeX's stretch 1/2 and shrink 1/3); `skip` the finite stretch of \rightskip (None: fil);
    `centred` adds the same \leftskip and a zero \parfillskip."""
    face, size = face_of(style), size_of(style)
    words = text.split()
    if not TEX_BREAKS:
        return [(line, 0.0) for line in greedy_lines(words, face, size, width, space[0] if space else face.width(" ", size))]
    sp = space or (face.width(" ", size), face.width(" ", size) / 2, face.width(" ", size) / 3)
    ww = [face.width(w, size) for w in words]
    hang = [PROTRUDE.get(w[-1], 0) * face.width(w[-1], size) for w in words]
    n = len(words)

    def badness(i: int, j: int) -> tuple[float, int] | None:
        k = j - i
        nat = sum(ww[i:j]) + (k - 1) * sp[0] - hang[j - 1]
        last = j == n
        if nat > width:
            shrink = (k - 1) * sp[2]
            if shrink <= 0 or nat - width > shrink:
                return None
            r = (nat - width) / shrink
            b = min(10000.0, 100 * r ** 3)
            return b, 3 if b > 12 else 2
        if (last and not centred) or skip is None:
            return 0.0, 2
        stretch = (k - 1) * sp[1] + skip * (2 if centred else 1)
        b = min(10000.0, 100 * ((width - nat) / stretch) ** 3) if stretch > 0 else 10000.0
        return b, 0 if b > 99 else 1 if b > 12 else 2

    for tolerance in (100, 200, 10000):
        best = {(0, 2): (0.0, None)}
        for j in range(1, n + 1):
            for (i, fit), (dem, _) in list(best.items()):
                if i >= j:
                    continue
                bf = badness(i, j)
                if bf is None or bf[0] > tolerance:
                    continue
                b, f = bf
                d = dem + (10 + b) ** 2 + (10000 if abs(f - fit) > 1 and i > 0 else 0)
                key = (j, f)
                # ties go to the later predecessor, as in TeX's try_break (`d <= minimal_demerits`)
                if key not in best or d <= best[key][0] + 1e-9:
                    best[key] = (d, (i, fit))
        ends = [(v[0], k) for k, v in best.items() if k[0] == n]
        if ends:
            break
    _, key = min(ends)
    breaks = []
    while key[0] > 0:
        breaks.append(key[0])
        key = best[key][1]
    breaks = [0] + breaks[::-1]
    out = []
    for a, b in zip(breaks, breaks[1:]):
        # the glue set: how much TeX stretched (or shrank) each interword space of the line
        k, nat = b - a, sum(ww[a:b]) + (b - a - 1) * sp[0] - hang[b - 1]
        extra = 0.0
        if k > 1 and nat > width and sp[2]:
            extra = -(nat - width) / ((k - 1) * sp[2]) * sp[2]
        elif k > 1 and skip is not None and not (b == n and not centred):
            extra = (width - nat) / ((k - 1) * sp[1] + skip * (2 if centred else 1)) * sp[1]
        out.append((" ".join(words[a:b]), extra))
    return out


def greedy_lines(words: list[str], face: Face, size: float, width: float, sp: float) -> list[str]:
    lines, cur = [], []
    for w in words:
        trial = cur + [w]
        if cur and sum(face.width(x, size) for x in trial) + sp * (len(trial) - 1) > width:
            lines.append(" ".join(cur))
            cur = [w]
        else:
            cur = trial
    lines.append(" ".join(cur))
    return lines


def setpar(style: str) -> tuple[float, float, float]:
    r"""\setpar with the node font: the font's space, stretch 0.4 x fontdimen3 (half a space in XeTeX), no shrink."""
    s = face_of(style).width(" ", size_of(style))
    return s, 0.2 * s, 0.0


LM_SPACE = (3.33333 * TP, 0.0, 0.0)   # measured in the book: these spaces never stretch


def lm_extra(style: str) -> float:
    """What to add to `style`'s own space to make it Latin Modern's."""
    return LM_SPACE[0] - face_of(style).width(" ", size_of(style))   # \setpar in a node without a font: 10 pt Latin Modern's space


def joined(lines: list, refs: dict[str, dict] | None = None, base_ws: float = 0.0) -> list:
    """Lines as runs with explicit breaks, each with TeX's glue set as extra word spacing; `refs` turns stand-in
    numbers back into page references."""
    out: list = []
    for k, item in enumerate(lines):
        line, extra = item if isinstance(item, tuple) else (item, 0.0)
        if k:
            out.append({"br": True})
        if abs(extra) > 0.01:
            # a run's word spacing replaces the style's, so carry the style's along
            out.append({"text": line, "word_spacing": pt(base_ws + extra)})
            continue
        for token, ref in (refs or {}).items():
            if token in line:
                a, b = line.split(token, 1)
                out += [a, ref]
                line = b
        if line:
            out.append(line)
    return out


def leader_spacer(style: str, gap_in: float) -> dict:
    """A space that is `gap_in` wide in `style` (TikZ's +0.08 in either side of a leader)."""
    return {"text": " ", "word_spacing": pt(gap_in * IN - face_of(style).width(" ", size_of(style)))}


def leader() -> dict:
    face = FACES["CH"]
    span = dot_span()
    return {"leader": ".", "span": "dot", "gap": pt(4.01 * TP - face.width(".", span["size"]))}


# ---- the book's text, read from its sources ----

def module_ast(name: str) -> ast.Module:
    return ast.parse((BOOK / "design/build" / name).read_text())


def find_func(tree: ast.Module, name: str) -> ast.FunctionDef:
    return next(n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == name)


def assigned(node: ast.AST, name: str) -> ast.expr:
    return next(n.value for n in ast.walk(node) if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in n.targets))


def first_const(e: ast.expr) -> str:
    """A string constant, or the leading constant of `"..." + table`."""
    while isinstance(e, ast.BinOp):
        e = e.left
    return e.value


def unref(text: str) -> tuple[list, str]:
    """TeX `\\pageref{x}` -> a Matenadar page reference; also the text with a stand-in number, for measuring."""
    parts, plain = [], text
    for i, piece in enumerate(re.split(r"\\pageref\{([^}]*)\}", text)):
        if i % 2:
            parts.append({"ref": piece})
            plain = plain.replace(f"\\pageref{{{piece}}}", "127")
        elif piece:
            parts.append(piece)
    return parts, plain


def typo(text: str) -> str:
    """pages.tex() without the escaping: curly quotes."""
    text = re.sub(r'"([^"]*)"', "\u201c\\1\u201d", text)
    return text.replace("'", "\u2019")


# ---- pages ----

def howto_page(fm: ast.Module) -> list[dict]:
    c = Canvas(recto=False)
    func = find_func(fm, "howto_page")
    steps = [(s.elts[0].value, first_const(s.elts[1])) for s in assigned(func, "steps").elts]
    rows = [tuple(x.value for x in r.elts) for r in assigned(func, "rows").elts]
    head = re.search(r"at \(0,8\.7\) \{\{([^}]*)\}\}", ast.unparse(func)).group(1)
    c.band(0)
    c.base(0, 8.7, "head", head)

    # \stepitem: a 0.62 in box with the badge (raised -0.02 in, baseline -0.055 in, r 0.2 in), then a 5.38 in parbox[t]
    BADGE_UP, BADGE_DOWN = 0.235 * IN, 0.165 * IN          # the badge's height and depth around the step's baseline
    body_x, body_w = 0.62, 5.38
    base = 8.26 * IN                                         # the top of the outer \parbox[t]
    prev_depth = None
    for i, (title, body) in enumerate(steps, 1):
        h_head, _ = face_of("step-head").ink(title, size_of("step-head"))
        height = max(BADGE_UP, h_head)
        base = base - height if prev_depth is None else base - (prev_depth + (8.5 + 1.0) * TP) - height   # \vspace{8.5pt} + \lineskip
        c.badge(0.2, base / IN + 0.035, 0.2, i, "badge")
        c.base(body_x, base / IN, "step-head", title, w=body_w)
        content, plain = unref(body)
        lines = tex_lines(plain, "step-body", body_w * IN)     # \raggedright: fil stretch, the font's own spaces
        if TEX_BREAKS:
            content = joined(lines, {"127": {"ref": "sol1"}})
        b0 = base - (2.5 + 17.6) * TP                              # \vspace{2.5pt} + the body's \baselineskip
        c.base(body_x, b0 / IN, "step-body", content=content, w=body_w + (6 / IN if TEX_BREAKS else 0))
        last = b0 - 17.6 * TP * (len(lines) - 1)
        depth = base - last + face_of("step-body").ink(lines[-1], size_of("step-body"))[1]
        if i == 3:                                            # the arrow table inside the third step
            for k, (label, level, note) in enumerate(rows):
                rb = last - (8 + 17.6) * TP - k * (4 + 17.6) * TP
                c.base(body_x, rb / IN, "row-label", label, w=1.42)
                for j, d in enumerate(P.LEVEL_DIRS[level]):
                    dx, dy = P.DIR_VEC[d]
                    n = math.hypot(dx, dy)
                    vx, vy = dx / n * 0.085, dy / n * 0.085
                    cx, cy = body_x + 1.42 + 0.1 + j * 0.23, rb / IN + 0.035
                    c.arrow((cx - vx, cy - vy), (cx + vx, cy + vy))
                c.base(body_x + 1.42 + 1.92, rb / IN, "row-note", note, w=2.0)
            depth = base - rb + 0.065 * IN
        prev_depth = max(BADGE_DOWN, depth)

    # frontmatter.example(page, 1.1)
    ex = find_func(fm, "example")
    ex_rows = [x.value for x in assigned(ex, "rows").elts]
    ex_text = assigned(ex, "text").value
    y, cell, x0 = 1.1, 0.33, 0.62
    c.line((0, y + 0.62), (P.BLOCK_W, y + 0.62))
    c.base(0, y + 0.36, "label-ex", "For example")
    top = y + 0.2
    rad = cell * 0.42
    c.els.append({"type": "letter-grid", "x": f"{c.X(x0):.4f}in", "y": f"{c.Y(top):.4f}in", "cell": f"{cell}in", "rows": ex_rows,
                  "set": {"font": "Source Sans Pro", "weight": 600, "size": round(16 * TP, 4)},
                  "highlights": [{"from": [row, start], "to": [row, start + length - 1], "stroke": {"width": round(0.8 * TP, 3), "color": "ink"},
                                  "thickness": round(2 * rad / cell, 4)} for row, start, length in ((0, 0, 5), (1, 1, 4))]})
    tx = x0 + 6 * cell + 0.35
    c.north(tx, top - 0.02, P.BLOCK_W - tx, "example", ex_text)
    return c.els


def plate_page(chapter: dict) -> list[dict]:
    c = Canvas(recto=False)
    c.band(chapter["number"], P.TOP_BAND_Y)
    c.band(chapter["number"], P.BOTTOM_BAND_Y)
    top, bottom, w = 9.24, 0.76, P.BLOCK_W
    c.rect((0, bottom), (w, top), 1.2)
    c.rect((0.06, bottom + 0.06), (w - 0.06, top - 0.06), 0.8)
    # \includegraphics[width=w-0.3, height=top-bottom-0.3, keepaspectratio] centred in the frame
    c.svg(f"{B}design/art/plates/chapter-{chapter['number']:02d}.svg", 0.15, top - 0.15, w - 0.3, top - bottom - 0.3, fit="contain")
    return c.els


def chapter_page(chapter: dict, puzzles: list[dict], label: str) -> list[dict]:
    c = Canvas(recto=True)
    n, mid = chapter["number"], P.BLOCK_W / 2
    c.band(n)
    c.base(mid, 8.42, "chapter-label", f"Chapter {P.NUMBER_WORDS[n]}", anchor="c", w=5.0)
    long = len(chapter["title"]) > 22
    lines = c.north(mid, 8.12, 5.4, "chapter-title", typo(chapter["title"]), anchor="n")
    if long:
        c.els[-1]["set"] = {"size": round(31 * TP, 4), "line_height": pt(36 * TP)}
    y = 7.02 if long else 7.42
    # pages.ornament_rule
    c.line((mid - 1.1, y), (mid - 0.2, y))
    c.line((mid + 0.2, y), (mid + 1.1, y))
    c.svg(f"{B}design/art/knit/motif-{chapter['motif']}.svg", mid - 0.085, y + 0.085, 0.17, 0.17)
    c.north(mid, y - 0.3, 4.6, "intro", typo(chapter["intro"]), anchor="n")
    # the ten puzzles, with page numbers and dotted leaders
    top, step, left, right = 4.86, 0.285, 0.7, P.BLOCK_W - 0.7
    c.line((left, top + 0.33), (right, top + 0.33))
    for i, p in enumerate(puzzles):
        yb = top - i * step
        c.base(left + 0.36, yb, "toc-num", str(p["number"]), anchor="e", w=0.5)
        # the first puzzle is in this sample, so its number is a live reference; the others are on pages 9-17 of the book
        number = {"ref": "puz1"} if p["number"] == 1 else str(chapter_page_of(p))
        c.base(left + 0.52, yb, "toc-title", content=[typo(p["title"]), leader_spacer("toc-title", 0.08), leader(),
                                                     leader_spacer("toc-title", 0.08), number], w=right - left - 0.52)
    bottom = top - 9 * step - 0.22
    c.line((left, bottom), (right, bottom))
    dirs = P.LEVEL_DIRS[chapter["level"]]
    width = len(dirs) * 0.2 + (len(dirs) - 1) * 0.09
    key_y = bottom - 0.5
    x = mid + 0.95 - width / 2
    c.arrows(x, key_y + 0.05, dirs, size=0.2, gap=0.09)
    c.base(x - 0.14, key_y, "key", label, anchor="e", w=3.0)
    return c.els


def chapter_page_of(p: dict) -> int:
    """Physical page of puzzle n in the book: chapter c starts with a plate at 6 + 12 (c - 1), its opener, then ten puzzles."""
    return 6 + 12 * (p["chapter"] - 1) + 2 + (p["number"] - 1) % 10


def bank_columns(words: list[str]) -> tuple[list[list[str]], list[float], float]:
    """pages.bank: three alphabetized columns, starting on thirds unless a long entry needs the room."""
    words = sorted(words)
    per = math.ceil(len(words) / 3)
    cols = [words[i * per:(i + 1) * per] for i in range(3)]
    right, size = P.BLOCK_W - 0.02, P.BANK_SIZE
    while True:
        w = [max(P.cap_width(x, size) for x in col) for col in cols]
        xs = [0.02, P.BLOCK_W / 3 + 0.05, 2 * P.BLOCK_W / 3 + 0.08]
        xs[1] = max(xs[1], xs[0] + w[0] + P.BANK_GAP)
        xs[2] = max(xs[2], xs[1] + w[1] + P.BANK_GAP)
        if xs[2] + w[2] > right:
            gap = (right - 0.02 - sum(w)) / 2
            xs = [0.02, 0.02 + w[0] + gap, 0.02 + w[0] + w[1] + 2 * gap]
            if gap >= P.BANK_GAP - 0.04 or size <= P.BANK_MIN_SIZE:
                break
            size -= 0.2
            continue
        break
    return cols, xs, size


def puzzle_page(p: dict, chapter: dict, recto: bool) -> list[dict]:
    c = Canvas(recto=recto)
    c.band(chapter["number"])
    # pages.title_row
    y = 9.05
    c.badge(0.2, y + 0.085, 0.2, p["number"], "badge")
    c.base(0.54, y, "puzzle-title", typo(p["title"]), w=5.5, **({"set": {"size": round(19.5 * TP, 4)}} if len(p["title"]) > 33 else {}))
    # the grid: Source Sans Pro Semibold 17.5 centred in 0.355 in cells, then the rounded frame
    rows = ["".join(r) for r in p["grid"]]
    h, w = len(rows) * P.CELL, len(rows[0]) * P.CELL
    gtop = P.GRID_TOP
    c.els.append({"type": "letter-grid", "id": f"grid{p['number']}", "x": f"{c.X(0):.4f}in", "y": f"{c.Y(gtop):.4f}in", "cell": f"{P.CELL}in",
                  "rows": rows, "set": {"font": "Source Sans Pro", "weight": 600, "size": round(17.5 * TP, 4)}})
    c.rect((-0.045, gtop - h - 0.045), (w + 0.045, gtop + 0.045), 0.9, radius=4.5)
    bottom = gtop - h
    # pages.bank_label
    label_y = bottom - P.LABEL_DROP
    c.base(0.02, label_y, "label", f"Find these {len(p['words'])} words")
    dirs = P.LEVEL_DIRS[p["level"]]
    width = len(dirs) * 0.17 + (len(dirs) - 1) * 0.07
    x = P.BLOCK_W - width - 0.02
    c.arrows(x, label_y + 0.04, dirs, size=0.17, gap=0.07)
    c.base(x - 0.1, label_y, "label", "Words run", anchor="e", w=2.0)
    # pages.bank
    cols, xs, size = bank_columns(p["words"])
    top_baseline = label_y - P.BANK_DROP
    for col, items in enumerate(cols):
        for row, word in enumerate(items):
            c.base(xs[col], top_baseline - row * P.BANK_STEP, "bank", word, w=2.2,
                   **({"set": {"size": round(size * TP, 4)}} if size != P.BANK_SIZE else {}))
    last = top_baseline - (len(cols[0]) - 1) * P.BANK_STEP
    rule = last - P.RULE_DROP
    c.line((0, rule), (P.BLOCK_W, rule))
    if p["type"] == "fact":
        # pages.fact_block: a run-in label, 0.55 em, the fact; at most two lines
        c.north(0, rule - 0.11, P.BLOCK_W, "fact", typo(p["reward"]),
                head=[{"text": "Festive fact", "span": "run-in"}, {"text": " ", "word_spacing": pt(0.55 * size_of("fact") - face_of("fact").width(" ", size_of("fact")))}])
    else:
        message_slots(c, p, rule)
    # the inside of the foot: "Solution on page N"
    if recto:
        c.base(0, P.FOOT_Y, "foot-ref", content=["Solution on page ", {"ref": f"sol{p['number']}"}])
    else:
        c.base(P.BLOCK_W, P.FOOT_Y, "foot-ref", content=["Solution on page ", {"ref": f"sol{p['number']}"}], anchor="e", w=3.0)
    return c.els


def message_slots(c: Canvas, p: dict, top: float) -> None:
    """pages.message_slots: one 0.8 pt dash per leftover letter, grouped by word, lines centred."""
    label_y = top - P.MESSAGE_LABEL_DROP
    c.base(0.02, label_y, "label", "Hidden message")
    c.base(P.BLOCK_W - 0.02, label_y, "message-note", "Write the leftover letters here, row by row, left to right.", anchor="e", w=4.5)
    area_top = label_y - P.MESSAGE_AREA_DROP
    bottom = P.BLANKS_FLOOR
    lengths = [len(re.sub(r"[^A-Z]", "", w.upper())) for w in p["reward"].split() if re.sub(r"[^A-Z]", "", w.upper())]
    slot, gap, line, lines = P.blanks_layout(lengths, area_top - bottom)
    line = max(line, min(0.43, (area_top - bottom) / len(lines)))
    free = (area_top - bottom) - len(lines) * line
    y = area_top - min(free / 2, 0.3)
    d = []
    for words in lines:
        y -= line
        width = sum(n * slot for n in words) + gap * (len(words) - 1)
        x = (P.BLOCK_W - width) / 2
        for n in words:
            for i in range(n):
                x1, x2 = c.X(x + i * slot + 0.024) * IN, c.X(x + (i + 1) * slot - 0.024) * IN
                d.append(f"M{x1:.2f} {c.Y(y) * IN:.2f}H{x2:.2f}")
            x += n * slot + gap
    # all the blanks as one path on the page (coordinates in pt from the page's corner)
    c.els.append({"type": "path", "x": 0, "y": 0, "w": "8in", "h": "10in", "d": "".join(d), "stroke": {"width": round(0.8 * TP, 3), "color": "ink"}})


def solutions_page(group: list[dict]) -> list[dict]:
    c = Canvas(recto=True)
    c.band(0)
    for p, top in zip(group, P.SOL_TOPS):
        solution_block(c, p, top)
    return c.els


def solution_block(c: Canvas, p: dict, top: float) -> None:
    grid = p["grid"]
    cell = P.SOL_CELL
    width, h = len(grid[0]) * cell, len(grid) * cell
    used = set()
    highlights = []
    for pl in p["placements"].values():
        (x1, y1), (x2, y2) = pl["start"], pl["end"]
        n = max(abs(x2 - x1), abs(y2 - y1))
        dx, dy = (x2 - x1) // n, (y2 - y1) // n
        for i in range(n + 1):
            used.add((x1 + dx * i, y1 + dy * i))
        # a capsule of radius 0.4 cell: grey 0.80, a white 0.9 pt edge so crossings stay apart
        highlights.append({"from": [y1, x1], "to": [y2, x2], "fill": "loop", "stroke": {"width": round(0.9 * TP, 3), "color": "#ffffff"},
                           "thickness": 0.8})
    message = p["type"] == "message"
    rows = []
    for r, row in enumerate(grid):
        cells = []
        for col, ch in enumerate(row):
            if (col, r) in used:
                cells.append(ch)                                   # \gr: Regular, black
            elif message:
                cells.append({"text": ch, "weight": 700})          # \gm: Bold, the hidden message
            else:
                cells.append({"text": ch, "color": "soft"})        # \gf: Regular, 42% grey filler
        rows.append(cells)
    c.els.append({"type": "letter-grid", "id": f"sol{p['number']}", "x": f"{c.X(0):.4f}in", "y": f"{c.Y(top):.4f}in", "cell": f"{cell}in",
                  "rows": rows, "set": {"font": "Source Sans Pro", "weight": 400, "size": round(12.4 * TP, 4)}, "highlights": highlights})
    c.rect((-0.035, top - h - 0.035), (width + 0.035, top + 0.035), 0.8, radius=3.5)
    # the side column
    x = width + 0.3
    side = P.BLOCK_W - x
    c.badge(x + 0.17, top - 0.17, 0.17, p["number"], "sol-badge" if p["number"] < 100 else "sol-badge")
    # one node, north west at (x, top - 0.44): \setpar with no node font, so 10 pt Latin Modern's spaces throughout
    slack = 6 / IN if TEX_BREAKS else 0
    title_lines = tex_set(typo(p["title"]), "sol-title", side * IN, space=LM_SPACE, skip=20 * TP)
    h1, _ = face_of("sol-title").ink(title_lines[0][0], size_of("sol-title"))
    b = (top - 0.44) * IN - h1
    c.base(x, b / IN, "sol-title", content=joined(title_lines, base_ws=lm_extra("sol-title")), w=side + slack)
    b -= (15.6 * (len(title_lines) - 1) + 3 + 14) * TP           # \vspace{3pt} + the next \baselineskip
    in_sample = p["number"] == 1                          # puzzle 1 is in this sample; the others are pages of the book
    c.base(x, b / IN, "sol-ref", content=["Puzzle on page ", {"ref": "puz1"} if in_sample else str(chapter_page_of(p))], w=side + slack)
    if message:
        b -= (11 + 10) * TP
        c.base(x, b / IN, "sol-label", "Hidden message", w=side + slack)
        b -= (2 + 16) * TP
        lines = tex_set(typo(p["reward"]), "sol-message", side * IN, space=LM_SPACE, skip=20 * TP)
        c.base(x, b / IN, "sol-message", content=joined(lines, base_ws=lm_extra("sol-message")), w=side + slack)


def main() -> None:
    content = json.loads((BOOK / "planning/content-v2.json").read_text())
    layouts = json.loads((BOOK / "puzzles/v2/layouts.json").read_text())
    mb = module_ast("make_book.py")
    motif, intro = ast.literal_eval(assigned(mb, "MOTIF")), ast.literal_eval(assigned(mb, "INTRO"))
    key_label = ast.literal_eval(assigned(find_func(module_ast("pages.py"), "chapter_page"), "label"))
    fm = module_ast("frontmatter.py")
    chapters = [dict(ch, motif=motif[ch["number"]], intro=intro[ch["number"]]) for ch in content["chapters"]]
    puzzles = [dict(p, **layouts[str(p["number"])]) for p in content["puzzles"]]
    ch1 = chapters[0]
    ch1_puzzles = [p for p in puzzles if p["chapter"] == 1]

    # The book's pages keep their sides: rectos are odd. `start: "verso"` is ignored on a document's first page and
    # follows the page counter rather than the sheet (see GAPS.md), so the two fillers are explicit blank pages.
    blank = lambda n: {"type": "page", "id": f"filler{n}", "master": "page", "elements": []}
    pages = [
        blank(1),
        {"type": "page", "id": "howto", "master": "page", "page_from": 4, "layers": True, "elements": howto_page(fm)},
        blank(2),
        {"type": "page", "id": "plate1", "master": "page", "page_from": 6, "elements": plate_page(ch1)},
        {"type": "page", "id": "chap1", "master": "page", "layers": True, "elements": chapter_page(ch1, ch1_puzzles, key_label)},
        {"type": "page", "id": "puz1", "master": "page", "layers": True, "title": ch1["title"],
         "elements": puzzle_page(puzzles[0], ch1, recto=False)},
        {"type": "page", "id": "solutions1", "master": "page", "page_from": 127, "layers": True, "title": "Solutions",
         "elements": solutions_page(puzzles[0:2])},
    ]
    doc = {
        "matenadar": "1",
        "meta": {"title": "Christmas Word Search: Hidden Messages & Festive Facts", "author": "Nell Bramley", "lang": "en-US"},
        "design": design(),
        "content": pages,
    }
    out = HERE / "doc.json" if TEX_BREAKS else HERE / "build/doc-native.json"
    out.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print("wrote", out)


if __name__ == "__main__":
    main()
