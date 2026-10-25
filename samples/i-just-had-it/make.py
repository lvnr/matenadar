#!/usr/bin/env python3
"""I Just Had It: four pages rebuilt in Matenadar.

    python3 samples/i-just-had-it/make.py        # writes doc.json (and build/tick.svg) beside this file

Reads only the book: manuscript/scenes.json and manuscript/matter.json (every word), planning/layouts.json (each
scene's palette and wall), build/scenes/<id>.{svg,json} (the finished scenes and where the things to find are),
visuals/sprites/<id>.json (the traced drawings, re-inked here exactly as design/build/book.py does) and the fonts in
design/fonts. The geometry is the book's CSS (design/book.css) in points: trim 7 x 10 in, bleed 0.125 in on the
top, bottom and outside. Coordinates are from the trim's top-left corner.
"""
from __future__ import annotations

import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
SCENES = json.loads((BOOK / "manuscript/scenes.json").read_text())["scenes"]
MATTER = json.loads((BOOK / "manuscript/matter.json").read_text())
LAYOUTS = json.loads((BOOK / "planning/layouts.json").read_text())

IN = 72.0
TRIM_W, TRIM_H, BLEED = 7 * IN, 10 * IN, 0.125 * IN
GUTTER, OUTER = 0.78 * IN, 0.6 * IN
TITLE = "I Just Had It"

# design/build/scene.py PALETTES: k key ink, a b c fills, p paper, g ground
PALETTES = {
    "plum": {"k": "#2b2140", "a": "#e56b6f", "b": "#f2c14e", "c": "#6d6aa8", "p": "#fcf6ee", "g": "#efe6ee"},
    "mint": {"k": "#1e2a38", "a": "#ef6f5e", "b": "#f5cd57", "c": "#48a391", "p": "#fdfaf0", "g": "#d9efe6"},
    "harbor": {"k": "#1c2440", "a": "#e8604c", "b": "#f4c95d", "c": "#4f7cac", "p": "#fdf8ee", "g": "#dfe8ea"},
    "ink": {"k": "#151a2b", "a": "#e5553f", "b": "#efb53e", "c": "#3d8b9a", "p": "#faf5e8", "g": "#fbf6ea"},
}
MONO = 0.6                                  # Courier Prime: every letter 0.6 em wide


def r2(v: float) -> float:
    return round(v, 2)


def palette_of(cfg: dict) -> dict:
    pal = dict(PALETTES[cfg.get("palette", "tomato")])
    pal.update(cfg.get("inks", {}))
    if cfg.get("ground"):
        pal["g"] = cfg["ground"]
    return pal


def ink_map(pal: dict, perm: str) -> dict:
    m = {"k": pal["k"], "p": pal["p"]}
    for slot, src in zip("abc", perm):
        m[slot] = pal[src]
    return m


def sprite(sid: str) -> dict:
    return json.loads((BOOK / f"visuals/sprites/{sid}.json").read_text())


def sprite_markup(sid: str, pal: dict, perm: str = "abc", extra: str = "") -> tuple[str, int, int]:
    """A traced drawing re-inked with a palette (design/build/book.py sprite_svg)."""
    meta = sprite(sid)
    inks = ink_map(pal, perm)
    body = "".join(f'<path fill="{inks[layer["ink"]]}" d="{layer["d"]}"/>' for layer in meta["layers"])
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {meta["w"]} {meta["h"]}">'
           f'<g transform="{meta["transform"]}">{body}</g>{extra}</svg>')
    return svg, meta["w"], meta["h"]


def sprite_element(sid: str, pal: dict, max_w_in: float, max_h_in: float, cx: float, cy: float, perm: str = "abc", extra: str = "") -> dict:
    """The drawing at the book's size rule (fit inside max_w x max_h inches), centred on (cx, cy)."""
    svg, w, h = sprite_markup(sid, pal, perm, extra)
    k = min(max_w_in / w, max_h_in / h) * IN
    return {"type": "svg", "markup": svg, "x": r2(cx), "y": r2(cy), "w": r2(w * k), "h": r2(h * k), "anchor": "center"}


def wrap_lines(text: str, chars: int) -> int:
    """Lines a monospace paragraph takes at `chars` letters a line (greedy, as browsers break)."""
    lines, cur = 1, 0
    for word in text.split():
        need = len(word) if cur == 0 else cur + 1 + len(word)
        if need <= chars:
            cur = need
        else:
            lines, cur = lines + 1, len(word)
    return lines


# ---------------------------------------------------------------- the design
def design() -> dict:
    ink = PALETTES["ink"]
    courier = "Courier Prime"
    styles = {
        # how this works
        "h1": {"font": "SILead", "size": 52, "line_height": 0.98, "tracking": "-0.015em", "space_after": "0.32in"},
        "how-p": {"font": "SIText", "size": 14, "line_height": "20pt", "space_after": "0.2in"},
        "how-em": {"font": "SITextItalic", "size": 14, "line_height": "20pt"},
        # the numeral hangs 0.1 in above its step (CSS top: -0.1in at line-height 1): a 35.6 pt line puts its
        # baseline 35.9 pt below the step's top
        "how-num": {"font": "SILead", "size": 50, "line_height": "35.6pt", "color": ink["a"]},
        "lg-label": {"font": courier, "weight": 700, "size": 7.4, "line_height": 1, "tracking": "0.09em", "case": "upper", "align": "center"},
        "tally": {"font": "SITextItalic", "size": 12.5, "line_height": 1.3},
        # notice
        "meta": {"font": courier, "weight": 700, "size": 7.6, "line_height": 1, "tracking": "0.09em", "case": "upper"},
        "kicker": {"font": "League Gothic", "size": 150, "line_height": 0.84, "tracking": "0.005em", "case": "upper"},
        "head": {"font": "League Gothic", "size": 66, "line_height": 0.9, "tracking": "0.01em", "case": "upper"},
        "fact-label": {"font": courier, "weight": 700, "size": 7.4, "line_height": "12.6pt", "tracking": "0.07em", "case": "upper"},
        "fact": {"font": courier, "size": 10.2, "line_height": "12.8pt"},
        "also-label": {"font": courier, "weight": 700, "size": 7.4, "line_height": "12.4pt", "tracking": "0.07em", "case": "upper"},
        "item": {"font": courier, "size": 9.2, "line_height": "12.4pt", "hanging": "0.2in"},
        "tab": {"font": courier, "weight": 700, "size": 6.7, "line_height": 1, "tracking": "0.03em", "align": "center"},
        # scene
        # the pill is a box mark round the run; a 5 pt line makes its outset (the pill's right padding) the right height
        "tag": {"font": "SILead", "size": 21, "line_height": "5.03pt", "tracking": "-0.005em"},
        "tag-no": {"font": "SILabel", "size": 11.5, "line_height": "30.24pt", "tracking": "0.02em", "align": "center"},
        "foot": {"font": "SILabel", "size": 6.6, "line_height": 1, "tracking": "0.2em", "case": "upper", "align": "right"},
        # answers
        "found": {"font": "League Gothic", "size": 54, "line_height": 0.86, "case": "upper"},
        "ans-p": {"font": courier, "size": 10.4, "line_height": "14.6pt", "space_before": "0.16in"},
        "key": {"font": courier, "size": 9.4, "line_height": "12pt"},
        "cap": {"font": courier, "weight": 700, "size": 7.2, "line_height": 1, "tracking": "0.08em", "case": "upper", "margin_right": "0.08em"},
    }
    spans = {"cap-no": {"font": "League Gothic", "weight": 400, "size": 13, "tracking": "0.03em"}}
    return {
        "colors": {"ink-k": ink["k"], "ink-a": ink["a"], "ink-p": ink["p"], "ink-g": ink["g"]},
        "base": {"font": "SIText", "size": 14, "line_height": "20pt", "color": "ink-k", "hyphenate": False, "lang": "en"},
        "styles": styles,
        "spans": spans,
        "masters": {
            "sheet": {"width": "7in", "height": "10in", "bleed": "0.125in",
                      "margins": {"top": "0.72in", "bottom": "0.7in", "inside": "0.78in", "outside": "0.6in"}},
        },
    }


# ---------------------------------------------------------------- p. 5: how this works (recto)
def how_page() -> dict:
    pal = PALETTES["ink"]
    h = MATTER["how"]
    x, top, bottom = GUTTER, 0.72 * IN, TRIM_H - 0.7 * IN
    w = TRIM_W - GUTTER - OUTER
    # steps: the numeral in a 0.78 in column, the text beside it; 0.26 in between steps
    rows = [[{"style": "how-num", "text": str(i + 1)},
             {"style": "how-p", "text": step}] for i, step in enumerate(h["steps"])]
    blocks = [{"type": "p", "style": "h1", "text": h["head"]}]
    blocks += [{"type": "p", "style": "how-p", "text": p} for p in h["intro"]]
    blocks.append({"type": "table", "columns": [r2(0.78 * IN), "1fr"], "rows": rows, "stroke": "none",
                   "inset": {"top": 0, "left": 0, "right": 0, "bottom": r2(0.26 * IN)}, "space_before": "0.2in", "space_after": 0})
    blocks += [{"type": "p", "style": "how-em", "text": p} for p in h["outro"]]
    els = [{"type": "text", "id": "how", "x": x, "y": r2(top), "w": r2(w), "blocks": blocks}]
    els += legend(x, w, bottom - 0.58 * IN, pal, h["legend"])
    els.append({"type": "text", "id": "tally", "x": x, "y": r2(bottom - 40), "w": r2(w), "h": 40, "valign": "bottom", "style": "tally",
                "text": h["legend"]["tally"]})
    return {"type": "page", "id": "how", "master": "sheet", "fill": pal["p"], "elements": els}


def legend(x: float, w: float, bottom: float, pal: dict, lg: dict) -> list:
    """Three cells 1 : 1.5 : 1 inside a 1.5 pt frame: pictures over a ruled label row."""
    border = 1.5
    pic_h, label_h = 1.42 * IN, 0.09 * IN + 7.4 + 0.085 * IN + border
    height = border * 2 + pic_h + label_h
    top = bottom - height
    inner = w - 2 * border
    widths = [inner * f / 3.5 for f in (1, 1.5, 1)]
    xs = [x + border]
    for cw in widths[:-1]:
        xs.append(xs[-1] + cw)
    k = pal["k"]
    els = [{"type": "rect", "x": r2(x), "y": r2(top), "w": r2(w), "h": r2(height), "fill": pal["g"]},
           {"type": "rect", "x": r2(x), "y": r2(top + border + pic_h), "w": r2(w), "h": r2(label_h + border), "fill": pal["p"]}]
    pic_top = top + border
    cy = pic_top + pic_h / 2
    # one lost thing, four others (a 2 x 2 grid, gaps 0.1 in x 0.3 in), one cat
    els.append(sprite_element(lg["lost"][0], pal, 1.0, 1.0, xs[0] + (widths[0] - border) / 2, cy))
    four = [(sid, *sprite_size(sid, 0.56, 0.5)) for sid in lg["others"]]
    col_w = [max(four[0][1], four[2][1]), max(four[1][1], four[3][1])]
    row_h = [max(four[0][2], four[1][2]), max(four[2][2], four[3][2])]
    gx, gy = 0.3 * IN, 0.1 * IN
    gw, gh = col_w[0] + gx + col_w[1], row_h[0] + gy + row_h[1]
    x0 = xs[1] + (widths[1] - border) / 2 - gw / 2
    y0 = cy - gh / 2
    for i, (sid, _, _) in enumerate(four):
        c, r = i % 2, i // 2
        ccx = x0 + (col_w[0] + gx if c else 0) + col_w[c] / 2
        ccy = y0 + (row_h[0] + gy if r else 0) + row_h[r] / 2
        els.append(sprite_element(sid, pal, 0.56, 0.5, ccx, ccy))
    els.append(sprite_element(lg["cat"][0], pal, 0.9, 1.0, xs[2] + widths[2] / 2, cy))
    # dividers, the label row's rule, the frame (CSS borders lie inside the box; strokes are centred on the path)
    for xi, cw in zip(xs[:-1], widths[:-1]):
        els.append({"type": "line", "x1": r2(xi + cw - border / 2), "y1": r2(top), "x2": r2(xi + cw - border / 2), "y2": r2(top + height),
                    "stroke": {"width": border, "color": k}})
    rule_y = top + border + pic_h + border / 2
    els.append({"type": "line", "x1": r2(x), "y1": r2(rule_y), "x2": r2(x + w), "y2": r2(rule_y), "stroke": {"width": border, "color": k}})
    els.append({"type": "rect", "x": r2(x + border / 2), "y": r2(top + border / 2), "w": r2(w - border), "h": r2(height - border),
                "stroke": {"width": border, "color": k}})
    for i, (xi, cw) in enumerate(zip(xs, widths)):
        label_w = cw - (border if i < 2 else 0)
        els.append({"type": "text", "id": f"lg-{i + 1}", "x": r2(xi), "y": r2(top + border + pic_h + border + 0.09 * IN), "w": r2(label_w),
                    "style": "lg-label", "text": lg["labels"][i]})
    return els


def sprite_size(sid: str, max_w_in: float, max_h_in: float) -> tuple[float, float]:
    meta = sprite(sid)
    k = min(max_w_in / meta["w"], max_h_in / meta["h"]) * IN
    return meta["w"] * k, meta["h"] * k


# ---------------------------------------------------------------- p. 6: the notice (verso)
def notice_page(n: int, copy: dict) -> dict:
    cfg = LAYOUTS["scenes"][copy["id"]]
    pal = palette_of(cfg)
    wall = pal.get(cfg.get("wall", "c"), cfg.get("wall", "c"))
    state = json.loads((BOOK / f"build/scenes/{copy['id']}.json").read_text())
    k = pal["k"]
    rule = lambda wd=0.75: {"width": wd, "color": k}
    # the card: 0.6 in from the outside edge, 0.78 in from the spine, 0.62 in from top and bottom; 1.5 pt border inside
    cx0, cy0 = OUTER, 0.62 * IN
    cw, ch = TRIM_W - OUTER - GUTTER, TRIM_H - 2 * 0.62 * IN
    b = 1.5
    pad_x, pad_top = 0.36 * IN, 0.3 * IN
    x, w = cx0 + b + pad_x, cw - 2 * b - 2 * pad_x
    top = cy0 + b + pad_top
    els = [{"type": "rect", "x": r2(cx0 + b / 2), "y": r2(cy0 + b / 2), "w": r2(cw - b), "h": r2(ch - b), "fill": pal["p"], "stroke": rule(b)}]
    # the pin: 0.36 in across with its border, 0.19 in above the card's padding box, a paper highlight
    pin = 0.36 * IN
    pin_x, pin_y = cx0 + b + (cw - 2 * b) / 2 - pin / 2, cy0 + b - 0.19 * IN
    els.append({"type": "ellipse", "x": r2(pin_x + b / 2), "y": r2(pin_y + b / 2), "w": r2(pin - b), "h": r2(pin - b), "fill": pal["a"], "stroke": rule(b)})
    els.append({"type": "ellipse", "x": r2(pin_x + b + 0.07 * IN), "y": r2(pin_y + b + 0.06 * IN), "w": r2(0.08 * IN), "h": r2(0.08 * IN), "fill": pal["p"]})
    # meta row: three items spread, a rule 0.075 in below
    els.append({"type": "text", "id": "meta", "x": r2(x), "y": r2(top), "w": r2(w), "style": "meta",
                "content": [f"No. {n}", {"leader": ""}, copy["when"], {"leader": ""}, copy["where"]],
                "set": {"rule_below": {"stroke": rule(), "gap": r2(0.075 * IN + 0.375)}, "margin_right": "0.09em"}})
    y = top + 7.6 + 0.075 * IN + 0.75
    # poster type: fitted to the column, up to its largest size (a one-line frame that shrinks if it would wrap)
    ksize, hsize = copy.get("kicker_max", 150), copy.get("head_max", 66)
    y += 0.13 * IN
    els.append({"type": "text", "id": "kicker", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(ksize * 0.84), "fit": "shrink", "min_size": 8,
                "style": "kicker", "set": {"size": ksize, "color": pal["a"]}, "text": copy["kicker"]})
    y += ksize * 0.84 + 0.05 * IN
    els.append({"type": "text", "id": "head", "x": r2(x), "y": r2(y), "w": r2(w), "h": r2(hsize * 0.9), "fit": "shrink", "min_size": 8,
                "style": "head", "set": {"size": hsize, "color": k}, "text": copy["head"]})
    y += hsize * 0.9 + 0.16 * IN
    pic_top = y
    # bottom up: the tabs fill the card's last 1.12 in; the also-list ends 0.15 in above them
    card_in_bottom = cy0 + ch - b
    tabs_top = card_in_bottom - 1.12 * IN
    stack_bottom = tabs_top - 0.15 * IN
    facts_h, also_h = facts_height(copy), also_height(copy)
    # The picture takes the height left over (CSS flex: 1). Matenadar has no "fill what is left", so it is computed here
    # from the monospace rows below it (GAPS.md).
    pic_bottom = stack_bottom - also_h - 0.13 * IN - facts_h - 0.17 * IN
    pic_h = pic_bottom - pic_top
    els += picture(x, pic_top, w, pic_h, copy, state, pal)
    # facts and the also-list, set from the bottom
    label_col = 1.42 * IN + 0.14 * IN
    fact_rows = []
    for label, value in copy["lines"]:
        fact_rows.append([
            {"blocks": [{"type": "p", "style": "fact-label", "text": label,
                         "set": {"rule_above": {"stroke": rule(), "gap": r2(0.375 + 0.055 * IN + 0.012 * IN)}}}]},
            {"blocks": [{"type": "p", "style": "fact", "text": value, "set": {"rule_above": {"stroke": rule(), "gap": r2(0.375 + 0.055 * IN)}}}]},
        ])
    tick = tick_svg(pal)
    item = lambda name: {"style": "item", "content": [{"image": tick, "h": 7.56, "w": 7.56, "baseline": 1.36},
                                                     {"text": " ", "word_spacing": "1.32pt"}, name]}
    names = [e["name"] for e in copy["extras"]]
    also_rows = [[{"text": "Also in here", "style": "also-label"}, item(names[0]), item(names[1])],
                 [{"text": ""}, item(names[2]), item(names[3])],
                 [{"text": ""}, dict(item("the cat"), colspan=2)]]
    item_col = (w - label_col - 0.14 * IN) / 2
    blocks = [
        {"type": "table", "columns": [r2(label_col), "1fr"], "rows": fact_rows, "stroke": "none",
         "inset": {"top": 0, "left": 0, "right": 0, "bottom": r2(0.055 * IN + 0.375)}, "space_before": 0, "space_after": 0},
        {"type": "canvas", "h": 0, "space_before": 0, "space_after": 0,
         "elements": [{"type": "line", "x1": 0, "y1": 0, "x2": r2(w), "y2": 0, "stroke": rule()}]},
        {"type": "table", "columns": [r2(label_col), r2(item_col + 0.14 * IN), "1fr"], "rows": also_rows, "stroke": "none",
         "inset": {"top": 0, "left": 0, "right": 0, "bottom": r2(0.02 * IN)}, "space_before": r2(0.13 * IN + 0.375), "space_after": 0},
    ]
    frame_top = pic_bottom + 0.17 * IN
    frame_bottom = stack_bottom + 0.02 * IN            # the last row's gap
    els.append({"type": "text", "id": "facts", "x": r2(x), "y": r2(frame_top + 0.375), "w": r2(w), "h": r2(frame_bottom - frame_top),
                "blocks": blocks})
    els += tabs(n, copy, cx0 + b, tabs_top, cw - 2 * b, card_in_bottom, pal, wall)
    return {"type": "page", "id": "notice", "master": "sheet", "fill": wall, "elements": els}


def facts_height(copy: dict) -> float:
    value_w = TRIM_W - OUTER - GUTTER - 3 - 0.72 * IN - 1.56 * IN
    chars = int(value_w / (MONO * 10.2))
    h = 0.75                                           # the rule above the first row
    for _, value in copy["lines"]:
        h += 0.055 * IN * 2 + max(12.6 + 0.012 * IN, 12.8 * wrap_lines(value, chars)) + 0.75
    return h


def also_height(copy: dict) -> float:
    w = TRIM_W - OUTER - GUTTER - 3 - 0.72 * IN - 1.56 * IN
    col = (w - 0.14 * IN) / 2
    chars, wide = int((col - 0.2 * IN) / (MONO * 9.2)), int((w - 0.2 * IN) / (MONO * 9.2))
    names = [e["name"] for e in copy["extras"]]
    rows = [max(wrap_lines(names[0], chars), wrap_lines(names[1], chars)), max(wrap_lines(names[2], chars), wrap_lines(names[3], chars)),
            wrap_lines("the cat", wide)]
    return sum(12.4 * r for r in rows) + 0.02 * IN * (len(rows) - 1)


def tick_svg(pal: dict) -> str:
    """The tick box (0.105 in square, 0.75 pt key border, paper inside) as a small file for inline use."""
    out = HERE / "build"
    out.mkdir(exist_ok=True)
    s = 0.105 * IN
    (out / "tick.svg").write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="{s:.2f}pt" height="{s:.2f}pt" viewBox="0 0 {s:.2f} {s:.2f}">'
                                  f'<rect x="0.375" y="0.375" width="{s - 0.75:.2f}" height="{s - 0.75:.2f}" fill="{pal["p"]}" stroke="{pal["k"]}" stroke-width="0.75"/></svg>')
    return "build/tick.svg"


def picture(x: float, y: float, w: float, h: float, copy: dict, state: dict, pal: dict) -> list:
    """The framed picture: the lost thing as in its scene (same drawing and inks), centred, clipped by the frame."""
    b = 1.5
    inst = next(i for i in state["instances"] if i["role"] == "target")
    meta = sprite(inst["sid"])
    extra = ""
    for a in (i for i in state["instances"] if i["role"] == "attach"):   # anything attached, drawn in the target's frame
        am = sprite(a["sid"])
        ang = math.radians(-inst["rot"])
        dx, dy = a["x"] - inst["x"], a["y"] - inst["y"]
        lx, ly = dx * math.cos(ang) - dy * math.sin(ang), dx * math.sin(ang) + dy * math.cos(ang)
        unit = max(meta["w"], meta["h"]) / inst["long"]
        s = a["long"] * unit / max(am["w"], am["h"])
        inks = ink_map(pal, "abc")
        body = "".join(f'<path fill="{inks[layer["ink"]]}" d="{layer["d"]}"/>' for layer in am["layers"])
        extra += (f'<g transform="translate({meta["w"] / 2 + lx * unit:.1f},{meta["h"] / 2 + ly * unit:.1f}) rotate({a["rot"] - inst["rot"]:.1f}) '
                  f'scale({s:.4f}) translate({-am["w"] / 2:.1f},{-am["h"] / 2:.1f})"><g transform="{am["transform"]}">{body}</g></g>')
    cfg = LAYOUTS["scenes"][copy["id"]]
    thing = sprite_element(inst["sid"], pal, 3.3, cfg.get("poster_h", 2.0), (w - 2 * b) / 2, (h - 2 * b) / 2, inst["perm"], extra)
    return [
        {"type": "group", "id": "pic", "x": r2(x + b), "y": r2(y + b), "w": r2(w - 2 * b), "h": r2(h - 2 * b), "clip": True, "fill": pal["g"],
         "children": [thing]},
        {"type": "rect", "x": r2(x + b / 2), "y": r2(y + b / 2), "w": r2(w - b), "h": r2(h - b), "stroke": {"width": b, "color": pal["k"]}},
    ]


def tabs(n: int, copy: dict, x: float, top: float, w: float, bottom: float, pal: dict, wall: str) -> list:
    """Nine tear-off tabs under a dashed fold, text reading bottom to top; one tab torn away."""
    k = pal["k"]
    dash = {"width": 0.75, "color": k, "dash": [2.2, 1.7]}
    tw = w / 9
    h = bottom - top
    text = copy["tabs"]
    size = 71.5 / (MONO * len(text)) if len(text) > 17 else 6.7
    gone = 1 + (n * 3) % 7
    els = [{"type": "line", "x1": r2(x), "y1": r2(top + 0.375), "x2": r2(x + w), "y2": r2(top + 0.375), "stroke": dash}]
    for i in range(9):
        left = x + i * tw
        if i < 8 and i != gone and i + 1 != gone:
            els.append({"type": "line", "x1": r2(left + tw - 0.375), "y1": r2(top + 0.75), "x2": r2(left + tw - 0.375), "y2": r2(bottom), "stroke": dash})
        if i == gone:
            continue
        cx, cy = left + tw / 2, top + 0.75 + (h - 0.75) / 2
        els.append({"type": "text", "id": f"tab-{i + 1}", "x": r2(cx - h / 2), "y": r2(cy - size / 2), "w": r2(h), "style": "tab", "rotate": -90,
                    "set": {"size": r2(size), **({"tracking": 0} if len(text) > 17 else {})}, "text": text})
    # the torn tab: wall colour through the card's edge, a 1.5 pt key edge on three sides
    gx, gw = x + gone * tw - 1.0, tw + 1.0
    gy, gh = top - 1.3 + 0.75 - 0.75, h + 1.3 + 1.6
    edge = {"width": 1.5, "color": k}
    els += [
        {"type": "rect", "x": r2(gx), "y": r2(gy), "w": r2(gw), "h": r2(gh), "fill": wall},
        {"type": "line", "x1": r2(gx + 0.75), "y1": r2(gy), "x2": r2(gx + 0.75), "y2": r2(gy + gh - 1.7), "stroke": edge},
        {"type": "line", "x1": r2(gx + gw - 0.75), "y1": r2(gy), "x2": r2(gx + gw - 0.75), "y2": r2(gy + gh - 1.7), "stroke": edge},
        {"type": "line", "x1": r2(gx), "y1": r2(gy + 0.75), "x2": r2(gx + gw), "y2": r2(gy + 0.75), "stroke": edge},
    ]
    return els


# ---------------------------------------------------------------- p. 7: the scene (recto)
def scene_page(n: int, copy: dict) -> dict:
    cfg = LAYOUTS["scenes"][copy["id"]]
    pal = palette_of(cfg)
    k, p = pal["k"], pal["p"]
    els = [{"type": "svg", "id": "scene", "src": f"book/build/scenes/{copy['id']}.svg", "x": 0, "y": -BLEED, "w": TRIM_W + BLEED,
            "h": TRIM_H + 2 * BLEED, "fit": "stretch", "decorative": True}]
    # the tag: a pill (box mark round the run, its outset the right padding) 0.56 in in, 0.5 in down; the number disc in its left end
    left, top = 0.56 * IN, 0.5 * IN
    b, outset = 1.5, 0.26 * IN + 0.75
    disc = 0.42 * IN
    pill_h = 0.085 * IN * 2 + disc + 2 * b
    text_x = left + b + 0.1 * IN + disc + 0.13 * IN
    run_x = left + b / 2 + outset
    spacer = round((text_x - run_x) / (0.1725 * 21 - 0.005 * 21))       # no-break spaces of SILead 21 pt
    lh = pill_h - 2 * outset - b
    baseline = top + b + 0.085 * IN + (disc - (21 + 0.035 * IN)) / 2 + (21 - 1.233 * 21) / 2 + 0.978 * 21
    top_edge = (0.978 + (lh / 21 - 1.233) / 2) * 21
    frame_y = baseline - top_edge + (pill_h / 2 - (baseline - top_edge + lh / 2 - top)) / 2      # split the centring error
    els.append({"type": "text", "id": "tag", "x": r2(run_x), "y": r2(frame_y), "w": 400, "style": "tag", "set": {"line_height": f"{lh:.2f}pt", "color": k},
                "content": [{"text": " " * spacer + copy["tag"],
                             "mark": {"kind": "box", "fill": p, "stroke": {"width": b, "color": k}, "radius": r2(0.34 * IN), "inset": r2(outset)}}]})
    disc_x, disc_y = left + b + 0.1 * IN, top + (pill_h - disc) / 2
    els.append({"type": "ellipse", "x": r2(disc_x), "y": r2(disc_y), "w": r2(disc), "h": r2(disc), "fill": k})
    els.append({"type": "text", "id": "tag-no", "x": r2(disc_x), "y": r2(disc_y), "w": r2(disc), "style": "tag-no", "set": {"color": p}, "text": str(n)})
    # the foot: a small pill 0.5 in from the outside edge, 0.42 in above the trim bottom
    fb = 0.75
    f_out = (14.93 - fb) / 2 - 6.6 / 2                  # outset that gives the pill its 0.05 / 0.045 in padding
    right, bottom = TRIM_W - 0.5 * IN, TRIM_H - 0.42 * IN
    pad = "  "
    els.append({"type": "text", "id": "foot", "x": r2(right - fb / 2 - f_out - 300), "y": r2(bottom - fb / 2 - f_out - 6.6), "w": 300,
                "style": "foot", "set": {"color": k},
                "content": [{"text": pad + TITLE + pad, "mark": {"kind": "box", "fill": p, "stroke": {"width": fb, "color": k}, "radius": r2(0.14 * IN),
                                                               "inset": r2(f_out)}}]})
    return {"type": "page", "id": "scene", "master": "sheet", "fill": pal["g"], "elements": els}


# ---------------------------------------------------------------- p. 66: answers (verso)
def answers_page(numbers: list[int]) -> dict:
    pal = PALETTES["ink"]
    left, right, top, bottom = 0.5 * IN, TRIM_W - 0.66 * IN, 0.5 * IN, TRIM_H - 0.46 * IN
    gap = 0.2 * IN
    cw, ch = (right - left - gap) / 2, (bottom - top - gap) / 2
    cells = [(left + c * (cw + gap), top + r * (ch + gap)) for r in range(2) for c in range(2)]
    els = text_slot(*cells[0], cw, ch, pal)
    for (sx, sy), n in zip(cells[1:], numbers):
        els += answer_slot(sx, sy, cw, ch, n, SCENES[n - 1], pal)
    return {"type": "page", "id": "answers", "master": "sheet", "fill": pal["g"], "elements": els}


def ring_canvas(r: float, colour: str, width: float, dash=None) -> dict:
    s = 0.72                                           # the key's 30-unit icons at 0.3 in
    stroke = {"width": r2(width * s), "color": colour, **({"dash": [r2(d * s) for d in dash], "cap": "round"} if dash else {})}
    return {"type": "canvas", "w": r2(0.3 * IN), "h": r2(0.3 * IN), "space_before": 0, "space_after": 0,
            "elements": [{"type": "ellipse", "x": r2((15 - r) * s), "y": r2((15 - r) * s), "w": r2(2 * r * s), "h": r2(2 * r * s), "stroke": stroke}]}


def text_slot(x: float, y: float, w: float, h: float, pal: dict) -> list:
    a = MATTER["answers"]
    pad_y, pad_x = 0.1 * IN, 0.12 * IN
    centre = (0.3 * IN - 12) / 2                       # the labels sit level with the middle of their icons
    rings = [ring_canvas(11, pal["a"], 4.5), ring_canvas(9, pal["k"], 2.4), ring_canvas(9, pal["k"], 2.6, [1.5, 4.6])]
    rows = [[{"blocks": [ring]}, {"blocks": [{"type": "space", "h": r2(centre), "weak": False}, {"type": "p", "style": "key", "text": label}]}]
            for ring, label in zip(rings, a["key"])]
    row_gap = 0.11 * IN
    blocks = [{"type": "p", "style": "found", "text": a["head"], "set": {"color": pal["a"]}},
              {"type": "p", "style": "ans-p", "text": a["intro"]},
              {"type": "table", "columns": [r2(0.4 * IN), "1fr"], "rows": rows, "stroke": "none",
               "inset": {"top": 0, "left": 0, "right": 0, "bottom": r2(row_gap)}, "space_before": "0.2in", "space_after": 0}]
    # centred in the slot (justify-content: center). The height is the column's, worked out here: Courier is monospace,
    # and a frame's valign would also centre the key's table cells (GAPS.md).
    inner_w, inner_h = w - 2 * pad_x, h - 2 * pad_y
    intro_lines = wrap_lines(a["intro"], int(inner_w / (MONO * 10.4)))
    content_h = 54 * 0.86 + 0.16 * IN + 14.6 * intro_lines + 0.2 * IN + 3 * 0.3 * IN + 2 * row_gap
    return [{"type": "text", "id": "found", "x": r2(x + pad_x), "y": r2(y + pad_y + (inner_h - content_h) / 2), "w": r2(inner_w), "blocks": blocks}]


def answer_slot(x: float, y: float, w: float, h: float, n: int, copy: dict, page_pal: dict) -> list:
    cfg = LAYOUTS["scenes"][copy["id"]]
    pal = palette_of(cfg)
    state = json.loads((BOOK / f"build/scenes/{copy['id']}.json").read_text())
    k = page_pal["k"]
    cap_top_edge = (1 + -1.123) / 2 * 7.2 / 7.2 + 0.78125      # the caption line's top edge, in em of the paragraph
    no_baseline = (13 - 1.2 * 13) / 2 + 0.9675 * 13           # where "No. N" sits in its own 13 pt line
    els = [{"type": "text", "id": f"cap-{n}", "x": r2(x), "y": r2(y + no_baseline - cap_top_edge * 13), "w": r2(w), "style": "cap",
            "content": [{"text": f"No. {n}", "span": "cap-no"}, {"leader": ""}, copy["head"]]}]
    cap_h = 13 + 0.05 * IN
    # the picture: a 0.75 pt frame on paper; inside, the scene without its top bleed, scaled to fill and centred
    b = 0.75
    fy, fh = y + cap_h + 0.2, h - cap_h - 0.2
    iw, ih = w - 2 * b, fh - 2 * b
    media_w, media_h = TRIM_W + BLEED, TRIM_H + 2 * BLEED
    # book.py shows the scene from y = 9 pt (no top bleed) through 747, "xMidYMid slice": the same drawing with that
    # viewBox, set with fit "cover" (an svg element cannot take a viewBox of its own: GAPS.md)
    src = thumb_svg(copy["id"])
    s = max(iw / media_w, ih / media_h)
    dx = (iw - media_w * s) / 2
    dy = -BLEED * s
    children = [
        {"type": "svg", "src": src, "x": 0, "y": 0, "w": r2(iw), "h": r2(ih), "fit": "cover"},
        # faded halfway to the scene's own paper (scene.py palette_of(fade=0.5)): paper at 50 % over it mixes the same
        {"type": "rect", "x": 0, "y": 0, "w": r2(iw), "h": r2(ih), "fill": pal["p"] + "80"},
    ]
    # rings in the scene's unfaded inks (scene.py svg_from_state marks=True)
    dark = sum(int(pal["g"][i:i + 2], 16) for i in (1, 3, 5)) < 300
    ring_ink = pal["p"] if dark else pal["k"]
    for f in state["find"]:
        cx, cy, r = f["x"] * IN * s + dx, f["y"] * IN * s + dy, max(0.3, f["long"] * 0.66) * IN * s
        if f["role"] == "target":
            r += 7 * s
            children += [ring(cx, cy, r, {"width": r2(15 * s), "color": pal["p"]}), ring(cx, cy, r, {"width": r2(9 * s), "color": pal["a"]})]
        elif f["role"] == "extra":
            children.append(ring(cx, cy, r, {"width": r2(4.5 * s), "color": ring_ink}))
        else:
            children.append(ring(cx, cy, r, {"width": r2(5 * s), "color": ring_ink, "dash": [r2(3 * s), r2(9 * s)], "cap": "round"}))
    els.append({"type": "group", "id": f"thumb-{n}", "x": r2(x + b), "y": r2(fy + b), "w": r2(iw), "h": r2(ih), "clip": True, "fill": page_pal["p"],
                "children": children})
    els.append({"type": "rect", "x": r2(x + b / 2), "y": r2(fy + b / 2), "w": r2(w - b), "h": r2(fh - b), "stroke": {"width": b, "color": k}})
    return els


def thumb_svg(scene_id: str) -> str:
    """The finished scene with the answer pages' viewBox (0 9 513 738), written under build/."""
    out = HERE / "build/thumbs"
    out.mkdir(parents=True, exist_ok=True)
    svg = (BOOK / f"build/scenes/{scene_id}.svg").read_text()
    head, rest = svg.split(">", 1)
    head = head.replace('viewBox="0 0 ', 'viewBox="0 9 ', 1)
    (out / f"{scene_id}.svg").write_text(head + ">" + rest)
    return f"build/thumbs/{scene_id}.svg"


def ring(cx: float, cy: float, r: float, stroke: dict) -> dict:
    return {"type": "ellipse", "x": r2(cx - r), "y": r2(cy - r), "w": r2(2 * r), "h": r2(2 * r), "stroke": stroke}


def main() -> None:
    built = {5: how_page(), 6: notice_page(1, SCENES[0]), 7: scene_page(1, SCENES[0]), 66: answers_page([1, 2, 3])}
    content = []
    for p in SAMPLE["pages"]:
        page = built[p["original"]]
        page["id"] = f"p{p['original']}"
        for e in page["elements"]:
            if "id" in e:
                e["id"] = f"{page['id']}-{e['id']}"
        content.append(page)
    doc = {
        "matenadar": "1",
        "meta": {"title": TITLE, "author": "", "lang": "en-US"},
        "fonts": ["book/design/fonts"],
        "design": design(),
        "content": content,
    }
    (HERE / "doc.json").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("doc.json:", ", ".join(f"p{p['original']}" for p in SAMPLE["pages"]))


if __name__ == "__main__":
    main()
