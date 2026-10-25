#!/usr/bin/env python3
"""Build doc.json: five pages of A Field Guide to Software Bugs, rebuilt in Matenadar.

Reads only the book (through the `book` symlink): the species texts (book/text/*.txt), the orders
(book/orders.json), the title (book/meta.json), the page map of the finished build
(build/book-report.json, for the opener's page numbers), the composited plates and grounds
(build/plates, build/pagebg) and the handwriting fonts (assets/fonts). Adobe Caslon Pro, Big Caslon
and JetBrains Mono are the Mac's installed fonts, as they were for the book.

    python3 make.py && python3 ../../scripts/review.py .

Pages (original → rebuilt), every one laid by hand as the book's were (one fixed page per HTML section):
    11 → 1  Order 0 opener                         13 → 3  The Integer Overflow (species page)
    25 → 5  The Buffer Overflow (ground + blot)    26 → 6  The Stack Overflow (the page inside its plate)
    37 → 7  The Infinite Loop (text on a circle)
(rebuilt pages 2 and 4 are blanks that keep the sides right)
"""
from __future__ import annotations

import json
import math
import random
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
BOOK = HERE / "book"

# ---------------------------------------------------------------- geometry (pt; trim coordinates)
IN = 72.0
TRIM_W, TRIM_H, BLEED = 6 * IN, 9 * IN, 0.125 * IN
GUTTER, OUTER, TOP, BOTTOM = 0.72 * IN, 0.48 * IN, 0.46 * IN, 0.50 * IN
BW = TRIM_W - GUTTER - OUTER                     # text block 4.8 in
BH = TRIM_H - TOP - BOTTOM                       # 8.04 in
HEAD_H = 0.24 * IN
PW, PH = BW, 3.36 * IN                           # the plate
CAP_Y = 3.66 * IN                                # caption box top, block-relative
SP_Y = 4.04 * IN                                 # species column top
SP_BOTTOM = BH - 0.31 * IN

C = {
    "ink": "#1d1a17", "accent": "#8c2f23", "second": "#5e5348", "notes": "#3a322a", "rule": "#6b5a48",
    "hair": "#b9a98e", "tint": "#F5EFE1",
}
CASLON, BIG, MONO = "Adobe Caslon Pro", "Big Caslon", "JetBrains Mono"
HAND = {"font": ["Field Hand L", "Caveat"], "weight": 500}   # every capital L from the one-letter companion font


def block_x(side: str) -> float:
    return GUTTER if side == "recto" else OUTER


# ---------------------------------------------------------------- the book's text

def curl(seg: str) -> str:
    """scripts/pagekit.py inline(): typographic quotes."""
    seg = re.sub(r'(^|[\s(\[])"', r"\1“", seg)
    seg = seg.replace('"', "”")
    seg = re.sub(r"(^|[\s(\[“])'", r"\1‘", seg)
    return seg.replace("'", "’")


def runs(s: str) -> list:
    """`code`, *italic*, curly quotes; <span class="sig">…</span> kept together."""
    out: list = []
    for i, part in enumerate(re.split(r"`([^`]+)`", s)):
        if i % 2:
            out.append({"text": part, "span": "code"})
            continue
        part = re.sub(r'<span class="sig">(.*?)</span>', lambda m: m.group(1).replace(" ", " "), part)
        for j, bit in enumerate(re.split(r"\*([^*]+)\*", curl(part))):
            if bit:
                out.append({"text": bit, "italic": True} if j % 2 else bit)
    return out


META = {"cap", "sight", "note", "mono", "hand", "ed", "opt", "ring"}


def load_species() -> dict:
    """book/text/*.txt, read as scripts/content.py reads it."""
    out = {}
    for path in sorted((BOOK / "book/text").glob("*.txt")):
        sp = None
        for raw in path.read_text(encoding="utf-8").splitlines():
            if not raw.strip() or raw.startswith("#"):
                continue
            if raw.startswith("=== "):
                n, sid, name, latin, gloss, status = [x.strip() for x in raw[4:].split("|")]
                sp = {"n": int(n), "id": sid, "name": name, "latin": latin, "status": status, "cap": "", "items": []}
                out[sid] = sp
                continue
            key, val = re.match(r"^([A-Za-z][A-Za-z ]*?): ?(.*)$", raw).groups()
            if key in META:
                if key == "cap":
                    sp["cap"] = val
                elif key == "ring":
                    sp["ring"] = val + " "
                elif key == "opt":
                    sp.update(json.loads(val))
                else:
                    sp["items"].append((key, "", val))
            else:
                sp["items"].append(("field", key, val))
    return out


SPECIES = load_species()
ORDERS = json.loads((BOOK / "book/orders.json").read_text(encoding="utf-8"))
TITLE = json.loads((BOOK / "book/meta.json").read_text(encoding="utf-8"))["title"]
PAGE_OF = {k["key"]: k["number"] for k in json.loads((BOOK / "build/book-report.json").read_text())["kinds"]}


def order_of(n: int) -> dict:
    for lo, hi, o in [(1, 9, 0), (10, 14, 1), (15, 19, 2), (20, 29, 3), (30, 39, 4), (40, 44, 5), (45, 55, 6), (56, 63, 7)]:
        if lo <= n <= hi:
            return ORDERS[o]
    raise ValueError(n)


def name_style(text: str) -> str:
    n = len(text)
    return "name" if n <= 19 else "name-n2" if n <= 22 else "name-n3" if n <= 24 else "name-n4"


# ---------------------------------------------------------------- the design

def styles() -> dict:
    return {
        "rh": {"font": CASLON, "size": 7.6, "line_height": 1.2, "tracking": "0.16em", "case": "upper", "color": "second"},
        "rh-pl": {"font": MONO, "size": 6.6, "line_height": 1.32, "tracking": "0.08em", "color": "accent"},
        "foot": {"font": CASLON, "size": 7.4, "line_height": 1.2, "tracking": "0.16em", "case": "upper", "color": "second"},
        "folio": {"font": CASLON, "size": 9.5, "line_height": 1.2, "tracking": "0.02em", "features": ["onum"]},
        "folio-mono": {"font": MONO, "size": 7.4, "line_height": 1.32, "tracking": "0.02em"},
        "cap": {"font": CASLON, "italic": True, "size": 9, "line_height": "11.2pt", "color": "notes", "align": "center"},
        "fig": {"font": CASLON, "italic": True, "size": 8.4, "line_height": 1.2, "color": "notes", "align": "center"},
        "name": {"font": BIG, "weight": 500, "size": 21, "line_height": "23pt", "tracking": "0.085em", "case": "upper",
                 "space_before": 0, "space_after": 0, "hyphenate": False},
        "name-n2": {"inherits": "name", "size": 18.5, "tracking": "0.075em"},
        "name-n3": {"inherits": "name", "size": 17, "tracking": "0.07em"},
        "name-n4": {"inherits": "name", "size": 15.5, "tracking": "0.065em"},
        "latin": {"font": CASLON, "italic": True, "size": 12.5, "line_height": 1.2},
        "status": {"font": MONO, "weight": 500, "size": 6.9, "line_height": 1.32, "tracking": "0.12em", "case": "upper",
                   "color": "accent"},
        "entry": {"font": CASLON, "size": 12.2, "line_height": "15.2pt", "space_before": 0, "space_after": 4.6,
                  "features": ["onum"], "hyphenate": False, "align": "left"},
        # Chrome turns optional ligatures off in any letter-spaced text, so the sighting has no Th or fi
        "sight": {"inherits": "entry", "tracking": "-0.003em", "ligatures": False, "space_before": 6,
                  "rule_above": {"stroke": {"width": 0.75, "color": "hair"}, "gap": 5 + 0.75}},
        "note": {"inherits": "entry", "italic": True, "size": 9.6, "line_height": "12.2pt", "color": "notes",
                 "space_before": 7},
        "hand": {**HAND, "size": 13.4, "line_height": "14pt", "color": "accent", "space_before": 0, "space_after": 0,
                 "hyphenate": False},
        # the order opener
        # Chrome centres a tracked line with the tracking after its last letter; the right margin does the same
        "ord": {"font": MONO, "size": 8, "line_height": 1.32, "tracking": "0.2em", "color": "accent", "align": "center",
                "space_before": 1.25 * IN, "space_after": 0, "margin_right": 0.2 * 8},
        "oh2": {"font": BIG, "weight": 500, "size": 27, "line_height": 1.209, "tracking": "0.13em", "case": "upper",
                "align": "center", "space_before": 10, "space_after": 2, "margin_right": 0.13 * 27},
        "eng": {"font": CASLON, "italic": True, "size": 14, "line_height": 1.2, "align": "center", "space_before": 2,
                "space_after": 0},
        "desc": {"font": CASLON, "size": 12.3, "line_height": "16.4pt", "features": ["onum"], "hyphenate": False,
                 "margin_left": (BW - 3.9 * IN) / 2, "margin_right": (BW - 3.9 * IN) / 2, "space_before": 0,
                 "space_after": 0},
        "otd": {"font": CASLON, "size": 10.8, "line_height": "14.6pt", "features": ["onum"], "space_before": 0,
                "space_after": 0, "hyphenate": False},
        "otd-n": {"inherits": "otd", "font": MONO, "size": 7.6, "color": "accent", "features": []},
        "otd-l": {"inherits": "otd", "italic": True, "size": 9.8, "color": "second", "align": "right"},
        "otd-p": {"inherits": "otd", "align": "right"},
    }


def components() -> dict:
    """The running head and the foot, mirrored on a verso. Used on the pages and inside the recursive plate."""
    rh_h = 0
    foot_y = BH - 1.2 * 7.4                      # the foot is bottom-aligned on the block's foot
    out = {}
    for side in ("recto", "verso"):
        out_side, in_side = ("right", "left") if side == "recto" else ("left", "right")
        out[f"head-{side}"] = {"params": {"head": None, "plate": None}, "w": BW, "h": HEAD_H, "elements": [
            {"type": "text", "x": 0, "y": rh_h, "w": BW, "style": "rh", "text": "{{head}}", "set": {"align": in_side}},
            {"type": "text", "x": 0, "y": rh_h + 0.6, "w": BW, "style": "rh-pl", "text": "{{plate}}",
             "set": {"align": out_side}},
        ]}
        out[f"foot-{side}"] = {"params": {"folio": None, "folio_style": "folio", "folio_y": BH - 1.2 * 9.5},
                               "w": BW, "h": 0.16 * IN, "elements": [
            {"type": "text", "x": 0, "y": foot_y, "w": BW, "style": "foot", "text": TITLE, "set": {"align": in_side}},
            {"type": "text", "x": 0, "y": "{{folio_y}}", "w": BW, "style": "{{folio_style}}", "text": "{{folio}}",
             "set": {"align": out_side}},
        ]}
    return out


def bw(pt: float, k: float = 1) -> float:
    """A CSS border width as Chrome lays it out and prints it: snapped to whole CSS pixels (0.75 pt), never
    under one, at the zoom k it is drawn at; returned in the element's own (unzoomed) units. So 0.35 and 0.5 pt
    rules print 0.75 pt, and inside the recursive plate every frame is one device pixel or more."""
    px = pt * k / 0.75
    return (1 if px < 1 else math.floor(px)) * 0.75 / k


# ---------------------------------------------------------------- page parts (block-relative elements)

def plate(src: str | None, labels: list | None = None, inner: list | None = None, k: float = 1) -> list:
    """A plate: the picture, a 0.35 pt hairline at its edge and a 0.5 pt neat line 5 pt in."""
    els = []
    if src:
        els.append({"type": "image", "src": src, "x": 0, "y": HEAD_H, "w": PW, "h": PH, "fit": "stretch"})
    e, n = bw(0.35, k), bw(0.5, k)               # CSS borders lie inside their boxes
    els += [
        {"type": "rect", "x": e / 2, "y": HEAD_H + e / 2, "w": PW - e, "h": PH - e,
         "stroke": {"width": e, "color": "hair"}},
        {"type": "rect", "x": 5 + n / 2, "y": HEAD_H + 5 + n / 2, "w": PW - 10 - n, "h": PH - 10 - n,
         "stroke": {"width": n, "color": "rule"}},
    ]
    for lab in labels or []:
        cx, cy = lab["x"] * PW, HEAD_H + lab["y"] * PH
        els.append({"type": "text", "x": cx - 20, "y": cy - 1.2 * 8.4 / 2, "w": 40, "style": "fig", "text": lab["t"]})
    return els + (inner or [])


def caption(text: str, y: float = CAP_Y, align: str = "center") -> dict:
    return {"type": "text", "x": 0, "y": y, "w": BW, "style": "cap", "content": runs(text), "set": {"align": align}}


def mono_width(text: str, size: float, tracking_em: float) -> float:
    """JetBrains Mono is monospaced: 600 units an em, plus the tracking after every letter (as Chrome adds it)."""
    return len(text) * (0.6 + tracking_em) * size


# the label's 0.28 em right margin, plus the 0.11 em tracking Chrome keeps after its last letter
LAB_GAP = 0.28 * 12 + 0.11 * 12


def entry_blocks(sp: dict, side: str, k: float = 1) -> list:
    blocks = []
    first = True
    for kind, label, text in sp["items"]:
        if kind == "field":
            b = {"type": "p", "style": "entry", "content": [{"text": f"{label}.", "span": "lab"},
                                                            {"space": LAB_GAP}, " "] + runs(text)}
        elif kind == "sight":
            b = {"type": "p", "style": "sight", "content": [{"text": "Notable sighting.", "span": "lab"},
                                                            {"space": LAB_GAP}, " "] + runs(text)}
            b["set"] = {"rule_above": {"stroke": {"width": bw(0.35, k), "color": "hair"}, "gap": 5 + bw(0.35, k)}}
        elif kind == "note":
            b = {"type": "p", "style": "note", "content": runs(text)}
        elif kind == "hand":
            # rotated -1.2° about its right end (left end on a verso); the box turns about its centre, so it is
            # lowered by half its width times sin 1.2° to put that end back
            # (a rotated box drops its own space_before, so the space is a block of its own)
            drop = BW / 2 * math.sin(math.radians(1.2))
            blocks.append({"type": "space", "h": 7 + drop})
            b = {"type": "box", "rotate": -1.2, "space_before": 0, "space_after": 0, "blocks": [
                {"type": "p", "style": "hand", "content": runs(text),
                 "set": {"align": "right" if side == "recto" else "left"}}]}
        else:
            continue
        if first:
            b["set"] = {**b.get("set", {}), "space_before": 6.5}
            first = False
        blocks.append(b)
    return blocks


def species_column(sp: dict, side: str, y: float = SP_Y, width: float = BW, fit_id: str | None = None,
                   style: str | None = None, k: float = 1) -> dict:
    """Name, Latin name with the status pill over a rule, then the labelled entry."""
    st = style or name_style(sp.get("heading", sp["name"]))
    pill = sp["status"]
    pw = mono_width(pill, 6.9, 0.12) + 10 + 1.5   # only to keep the Latin name clear of the pill
    ph = 1.32 * 6.9 + 4 + 1.5
    row_h = 1.2 * 12.5
    rule = bw(0.5, k)
    sub = {"type": "canvas", "w": width, "h": row_h + 6 + rule, "space_before": 3.5, "space_after": 0, "elements": [
        {"type": "text", "x": 0, "y": 0, "w": width - pw - 6, "style": "latin", "content": runs(sp["latin"])},
        {"type": "text", "x": width, "y": (row_h - ph) / 2, "anchor": "top-right", "style": "status", "text": pill,
         "stroke": {"width": bw(0.75, k), "color": "accent"}, "radius": 2.5,
         # padding plus border; on the right also the tracking Chrome keeps after the last letter
         "inset": {"top": 2.2 + 0.75, "right": 5 + 0.75 + 0.12 * 6.9, "bottom": 1.8 + 0.75, "left": 5 + 0.75}},
        {"type": "line", "x1": 0, "y1": row_h + 6 + rule / 2, "x2": width, "y2": row_h + 6 + rule / 2,
         "stroke": {"width": rule, "color": "rule"}},
    ]}
    blocks = [{"type": "h", "level": 1, "style": st, "content": runs(sp.get("heading", sp["name"]))}, sub]
    blocks += entry_blocks(sp, side, k)
    frame = {"type": "text", "x": 0, "y": y, "w": width, "blocks": blocks, "style": "entry"}
    if fit_id:                                   # the book failed its build when an entry overflowed; so does this
        frame.update({"id": fit_id, "h": SP_BOTTOM - y})
    return frame


def head_of(sp: dict) -> tuple[str, str]:
    o = order_of(sp["n"])
    return f'Order {o["n"]} · {o["latin"]}', sp.get("plate_no", f'PLATE {sp["n"]}')


def frame_uses(sp: dict | None, side: str, folio: str, mono: bool = False) -> list:
    out = []
    if sp is not None:
        head, plate_no = head_of(sp)
        out.append({"type": "use", "component": f"head-{side}", "x": 0, "y": 0,
                    "props": {"head": head, "plate": plate_no}})
    props = {"folio": folio}
    if mono:
        props.update({"folio_style": "folio-mono", "folio_y": BH - 1.32 * 7.4})
    out.append({"type": "use", "component": f"foot-{side}", "x": 0, "y": 0, "props": props})
    return out


def on_block(side: str, children: list, **kw) -> dict:
    """Block-relative elements, placed on the page."""
    return {"type": "group", "x": block_x(side), "y": TOP, "w": BW, "h": BH, "children": children, **kw}


def blot(level: int, side: str) -> list:
    """The Memory Leak's ink (scripts/species_pages.py blot), recomputed with the same seeded generator:
    one 240-point outline around the gutter-foot corner of the media box, plus drops. Units are 0.01 in."""
    R = [0.36, 0.62, 0.95, 1.3][level - 1] * 100
    rnd = random.Random(11)
    harm = [(k, rnd.uniform(0.03, 0.1) / math.sqrt(k), rnd.uniform(0, 2 * math.pi)) for k in range(2, 10)]
    media_w = (TRIM_W + BLEED) / IN * 100
    cx = 2.0 if side == "recto" else media_w - 2.0
    cy = (BLEED + TRIM_H) / IN * 100
    sx = 1 if side == "recto" else -1
    pts = []
    for i in range(240):
        th = 2 * math.pi * i / 240
        r = R * (1 + sum(a * math.sin(k * th + ph) for k, a, ph in harm))
        pts.append(f"{cx + sx * r * math.cos(th):.1f},{cy + r * math.sin(th):.1f}")
    d = "M" + " L".join(pts) + " Z"
    media_h = (TRIM_H + 2 * BLEED) / IN * 100
    u = (TRIM_W + BLEED) / media_w                # pt per unit
    x0 = -BLEED if side == "verso" else 0
    out = [{"type": "path", "x": x0, "y": -BLEED, "w": TRIM_W + BLEED, "h": TRIM_H + 2 * BLEED,
            "viewbox": [0, 0, round(media_w, 1), round(media_h, 1)], "d": d, "fill": "ink", "decorative": True}]
    drnd = random.Random(40 + level)
    for _ in range(3 + 2 * level):
        th = drnd.uniform(-0.47 * math.pi, -0.03 * math.pi)
        dist = R * drnd.uniform(1.12, 1.55)
        rr = round(R * drnd.uniform(0.018, 0.06), 1)
        x, y = round(cx + sx * dist * math.cos(th), 1), round(cy + dist * math.sin(th), 1)
        out.append({"type": "ellipse", "x": x0 + (x - rr) * u, "y": -BLEED + (y - rr) * u, "w": 2 * rr * u,
                    "h": 2 * rr * u, "fill": "ink", "decorative": True})
    return out


# ---------------------------------------------------------------- the pages

def page11() -> dict:
    o = ORDERS[0]
    rows = []
    for sp in sorted((s for s in SPECIES.values() if 1 <= s["n"] <= 9), key=lambda s: s["n"]):
        rows.append([{"blocks": [{"type": "p", "style": "otd-n", "text": str(sp["n"])}]},
                     {"blocks": [{"type": "p", "style": "otd", "content": runs(sp["name"])}]},
                     {"blocks": [{"type": "p", "style": "otd-l", "text": sp["latin"]}]},
                     {"blocks": [{"type": "p", "style": "otd-p", "text": str(PAGE_OF[sp["id"]])}]}])
    m = (BW - 3.9 * IN) / 2
    blocks = [
        {"type": "p", "style": "ord", "text": f'ORDER {o["n"]}'},
        {"type": "h", "level": 2, "style": "oh2", "text": o["latin"]},
        {"type": "p", "style": "eng", "text": o["english"]},
        {"type": "canvas", "w": 0.9 * IN, "h": bw(0.5), "align": "center", "space_before": 15, "space_after": 15,
         "elements": [{"type": "line", "x1": 0, "y1": bw(0.5) / 2, "x2": 0.9 * IN, "y2": bw(0.5) / 2,
                       "stroke": {"width": bw(0.5), "color": "rule"}}]},
        {"type": "p", "style": "desc", "content": runs(o["desc"])},
        {"type": "box", "inset": {"left": m, "right": m, "top": 0, "bottom": 0}, "space_before": 20, "space_after": 0,
         "blocks": [{"type": "table", "columns": [0.32 * IN, 1.8 * IN, "1fr", 0.34 * IN], "rows": rows, "stroke": "none",
                     "inset": 0, "space_before": 0, "space_after": 0}]},
    ]
    return {"type": "page", "id": "order-0", "master": "page", "elements": [
        on_block("recto", frame_uses(None, "recto", str(PAGE_OF["order-0"])) +
                 # a frame's first block loses its space above, so the 1.25 in drop is the frame's own y
                 [{"type": "text", "x": 0, "y": 1.25 * IN, "w": BW, "blocks": blocks}])]}


def page13() -> dict:
    sp = SPECIES["integer-overflow"]
    side = "recto"
    children = (frame_uses(sp, side, sp["folio"], mono=True) +
                plate(f'book/build/plates/{sp["id"]}.jpg', sp.get("plate", {}).get("labels")) +
                [caption(sp["cap"]), species_column(sp, side, fit_id="entry-13")])
    return {"type": "page", "id": sp["id"], "master": "page", "elements": [on_block(side, children)]}


def page25() -> dict:
    """The centipede leaves its plate: a whole-page ground (paper and centipede composited by the book's
    plates.py), the plate's frame drawn over it, and a measure narrowed by 1.1 in so the text stops short."""
    sp = SPECIES["buffer-overflow"]
    side = "recto"
    style = sp.get("heading_class", "").replace("name n", "name-n") or None
    children = (frame_uses(sp, side, str(PAGE_OF[sp["id"]])) + plate(None) +
                [caption(sp["cap"], align="left"),
                 species_column(sp, side, width=BW - 1.1 * IN, fit_id="entry-25", style=style)])
    return {"type": "page", "id": sp["id"], "master": "page", "elements": [
        {"type": "image", "src": f'book/build/pagebg/{sp["id"]}-{side}.jpg', "x": 0, "y": -BLEED,
         "w": TRIM_W + BLEED, "h": TRIM_H + 2 * BLEED, "fit": "stretch", "decorative": True},
        on_block(side, children),
        *blot(sp["leak"], side)]}


def mini_page(sp: dict, side: str, folio: str, depth: int, scale: float = 1) -> list:
    """The Stack Overflow's plate holds the page itself, which holds its plate, four levels deep
    (species_pages.droste): a 6 x 9 in page with a 1.6 pt outline, scaled by (3.36 - 0.2) / 9."""
    if depth == 0:
        return []
    s = round((PH / IN - 0.2) / (TRIM_H / IN), 4)
    left = (PW - TRIM_W * s) / 2
    top = (PH - TRIM_H * s) / 2
    # Chrome draws a zoomed border no thinner than one CSS pixel (0.75 pt), so the frames of the small
    # pages print heavy (bw with this level's zoom)
    k = scale * s
    inner = (frame_uses(sp, side, folio) +
             plate("book/build/plates/_blank.jpg", inner=mini_page(sp, side, folio, depth - 1, k), k=k) +
             [caption(sp["cap"]), species_column(sp, side, k=k)])
    o = bw(1.6, k)                               # the outline is drawn outside the page
    return [{"type": "group", "x": left, "y": HEAD_H + top, "w": TRIM_W, "h": TRIM_H, "scale": s, "children": [
        {"type": "rect", "x": -o / 2, "y": -o / 2, "w": TRIM_W + o, "h": TRIM_H + o, "fill": "tint",
         "stroke": {"width": o, "color": "rule"}},
        {"type": "group", "x": block_x(side), "y": TOP, "w": BW, "h": BH, "children": inner},
    ]}]


def page26() -> dict:
    sp = SPECIES["stack-overflow"]
    side = "verso"
    folio = str(PAGE_OF[sp["id"]])
    children = (frame_uses(sp, side, folio) +
                plate("book/build/plates/_blank.jpg", inner=mini_page(sp, side, folio, 4)) +
                [caption(sp["cap"]), species_column(sp, side, fit_id="entry-26")])
    return {"type": "page", "id": sp["id"], "master": "page", "elements": [on_block(side, children),
                                                                           *blot(sp["leak"], side)]}


def page37() -> dict:
    """The Infinite Loop: the colony on a round plate, and its caption set on a circle eight times over.
    Matenadar has no text on a path, so the ring is SVG markup (the book's own construction); textLength
    makes the eight repeats close exactly, which the book had to tune by hand in Chrome."""
    sp = SPECIES["infinite-loop"]
    side = "recto"
    ring_w, ring_h = BW, 3.9 * IN
    text = (sp["ring"].strip() + " ") * 8
    circ = 2 * math.pi * 147
    svg = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 480 390">'
           '<defs><path id="ringpath" d="M 240 185 m -147 0 a 147 147 0 1 1 294 0 a 147 147 0 1 1 -294 0"/></defs>'
           '<circle cx="240" cy="185" r="172" fill="none" stroke="#6b5a48" stroke-width="0.5"/>'
           f'<text font-family="{CASLON}" font-style="italic" font-size="9.39" fill="#3a322a">'
           f'<textPath href="#ringpath" textLength="{circ:.2f}" lengthAdjust="spacing">{text}</textPath></text></svg>')
    children = (frame_uses(sp, side, str(PAGE_OF[sp["id"]])) + [
        {"type": "image", "src": f'book/build/plates/{sp["id"]}.png', "x": 0.55 * IN, "y": HEAD_H, "w": 3.7 * IN,
         "h": 3.7 * IN, "fit": "stretch"},
        {"type": "svg", "x": 0, "y": HEAD_H, "w": ring_w, "h": ring_h, "markup": svg, "fit": "stretch"},
        caption(sp["cap"], y=4.0 * IN),
        species_column(sp, side, y=4.42 * IN, fit_id="entry-37"),
    ])
    return {"type": "page", "id": sp["id"], "master": "page", "elements": [on_block(side, children)]}


def blank(n: int) -> dict:
    return {"type": "page", "id": f"blank-{n}", "master": "page", "elements": []}


def main() -> None:
    doc = {
        "matenadar": "1",
        "meta": {"title": f"{TITLE} (sample pages)", "author": "Levon Arakelyan", "lang": "en-US"},
        "fonts": ["book/assets/fonts"],
        "design": {
            "colors": C,
            "fonts": {},
            "base": {"font": CASLON, "size": 12.2, "line_height": "15.2pt", "color": "ink", "lang": "en-US",
                     "hyphenate": False},
            "styles": styles(),
            "spans": {
                "lab": {"font": CASLON, "weight": 600, "size": 12, "case": "all-small-caps", "tracking": "0.11em",
                        "color": "accent"},
                "code": {"font": MONO, "size": "0.78em", "tracking": "-0.01em", "features": ["-onum"]},
            },
            "masters": {
                "page": {"width": "6in", "height": "9in", "bleed": "0.125in", "fill": "tint",
                         "margins": {"top": TOP, "bottom": BOTTOM, "inside": GUTTER, "outside": OUTER}},
            },
            "components": components(),
        },
        "content": [page11(), blank(2), page13(), blank(4), page25(), page26(), page37()],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
