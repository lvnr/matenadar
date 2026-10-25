#!/usr/bin/env python3
"""Idioms of the World, Drawn Literally: five pages rebuilt in Matenadar.

    python3 samples/idioms-of-the-world/make.py        # writes doc.json beside this file

Reads only the book: manuscript/idioms.json (every word), book.json (title), the plates under visuals/art/print and
the fonts under design/fonts. The page numbers (folios, index) come from replaying the book's own pagination
(scripts/build.py: six front pages, then per chapter an opener, one page per plate and an "also said" page, with
the interlude's four pages before its chapter).

The geometry is the book's CSS (scripts/book.css) turned into points: 6 x 9 in trim, text column 0.56 in from the
spine and 0.44 in from the outside. Coordinates are from the trim's top-left corner, as Matenadar measures them.
"""
from __future__ import annotations

import json
import os
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
DATA = json.loads((BOOK / "manuscript/idioms.json").read_text(encoding="utf-8"))
CONFIG = json.loads((BOOK / "book.json").read_text(encoding="utf-8"))
TITLE = CONFIG["title"]

IN = 72.0
TRIM_H = 9 * IN
INSIDE, OUTSIDE = 0.56 * IN, 0.44 * IN
BOX_W = 6 * IN - INSIDE - OUTSIDE                      # 360 pt
RTL = {"ar", "fa", "he", "yi", "ur"}
CJK = {"zh": "Noto Serif SC", "yue": "Noto Serif SC", "ja": "Noto Serif JP", "ko": "Noto Serif KR"}
# Headlines are balanced in the book (text-wrap: balance). The engine's `balance` does not compile yet ("unknown
# variable: mt-balanced", GAPS.md), so it is left out unless BALANCE=1; set it once the engine is fixed.
BALANCE = os.environ.get("BALANCE") == "1"
HAIR = {"width": 0.75, "color": "hair"}                 # Chrome draws the 0.6 pt hairlines at one CSS px
INK_RULE = {"width": 0.75, "color": "ink"}


def r2(v: float) -> float:
    return round(v, 2)


def box_x(side: str) -> float:
    """Left edge of the text column, in trim coordinates."""
    return INSIDE if side == "recto" else OUTSIDE


# ---------------------------------------------------------------- the book's text conventions (scripts/build.py)
def curl(text: str) -> str:
    text = re.sub(r'(^|[\s(\[{\u2014\u2013-])"', "\\1\u201c", text)
    text = text.replace('"', "\u201d")
    text = re.sub(r"(^|[\s(\[{\u201c])'(?=\w)", "\\1\u2018", text)
    return text.replace("'", "\u2019")


def apos(text: str) -> str:
    return re.sub(r"(?<=\w)'|'(?=s\b)", "\u2019", text)


def nobreak_end(text: str) -> str:
    return re.sub(r" (\S{1,2})$", "\u00a0\\1", text)


def plain(text: str) -> str:
    text = re.sub(r"\[\[(.+?)\|[a-z-]+\]\]", r"\1", text)
    return re.sub(r"\*(.+?)\*", r"\1", text).replace(" -- ", " \u2014 ")


def runs(text: str, emph: dict | None = None) -> list:
    """The book's esc(): curly quotes, *italic*, [[native script|lang]] and ' -- ' as runs."""
    text = curl(text).replace(" -- ", "\u2009\u2014\u2009")
    out, pos = [], 0
    for m in re.finditer(r"\[\[(.+?)\|([a-z-]+)\]\]|\*(.+?)\*", text):
        if m.start() > pos:
            out.append(text[pos:m.start()])
        if m.group(1):
            run = {"text": m.group(1), "lang": m.group(2)}
            if m.group(2) in CJK:
                run["font"] = [CJK[m.group(2)], "serif"]
            out.append(run)
        else:
            out.append(dict(emph or {"italic": True}, text=m.group(3)))
        pos = m.end()
    if pos < len(text):
        out.append(text[pos:])
    return out


def head_runs(label: str, head: str) -> list:
    """Running head: "No. 5" in bold ink, then the chapter head with its <b> part in bold ink."""
    # .no { margin-right: 0.6em }; the engine already tracks the label's last letter
    out = [{"text": label, "span": "head-b"}, {"text": " ", "word_spacing": "0.38em"}]
    pos = 0
    head = apos(head)
    for m in re.finditer(r"<b>(.+?)</b>", head):
        if m.start() > pos:
            out.append(head[pos:m.start()])
        out.append({"text": m.group(1), "span": "head-b"})
        pos = m.end()
    if pos < len(head):
        out.append(head[pos:])
    return out


def luminance(hex_colour: str) -> float:
    h = hex_colour.lstrip("#")
    lin = lambda v: v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)


def mix_black(hex_colour: str, keep: float) -> str:
    """CSS color-mix(in srgb, c keep%, #000)."""
    h = hex_colour.lstrip("#")
    return "#" + "".join(f"{round(int(h[i:i + 2], 16) * keep):02x}" for i in (0, 2, 4))


# ---------------------------------------------------------------- pagination, replayed from scripts/build.py
def paginate() -> tuple[dict[str, int], dict[str, list[int]]]:
    pages: dict[str, int] = {}
    things: dict[str, list[int]] = {}
    n = 6                                   # half title, epigraph, title, copyright, how to read, contents

    def tag(entries, number):
        for e in entries:
            for t in e.get("things", []):
                things.setdefault(t, []).append(number)

    inter = DATA.get("interlude")
    for ch in DATA["chapters"]:
        if inter and inter.get("before") == ch["id"]:
            n += 1
            pages["greek-opener"] = n
            for i, _ in enumerate(inter["chart"]):
                n += 1
                pages[f"greek-chart-{i + 1}"] = n
            n += 1
            pages["greek-also"] = n
            tag(inter["also"], n)
        n += len(ch.get("before", []))
        n += 1
        pages[f"{ch['id']}-opener"] = n
        for e in ch["pages"]:
            n += 1
            pages[e["art"]] = n
            tag([e], n)
        n += 1
        pages[f"{ch['id']}-also"] = n
        tag(ch["also"], n)
    for block in DATA["back"]:
        pages[block["id"]] = n + 1
        n += block.get("pages", 1) if block.get("index") else 1
    return pages, things


PAGES, THINGS = paginate()


# ---------------------------------------------------------------- the design
def design() -> dict:
    label = "label"
    styles = {
        # running head and foot
        "head": {"font": label, "weight": 600, "size": 7.3, "tracking": "0.17em", "case": "upper", "color": "grey", "line_height": 1},
        "folio": {"font": label, "weight": 700, "size": 8.6, "tracking": "0.04em", "color": "ink", "line_height": 1},
        "runfoot": {"font": label, "weight": 600, "size": 6.6, "tracking": "0.2em", "case": "upper", "color": "foot", "line_height": 1},
        # plate page
        "lang": {"font": label, "weight": 700, "size": 9.2, "tracking": "0.2em", "case": "upper", "line_height": 1},
        "literal-s1": {"font": "display", "size": 37, "line_height": 1.02, "tracking": "-0.005em", "balance": True},
        "literal-s2": {"inherits": "literal-s1", "size": 30, "line_height": 1.05},
        "literal-s3": {"inherits": "literal-s1", "size": 25, "line_height": 1.08},
        "literal-s4": {"inherits": "literal-s1", "size": 20, "line_height": 1.12},
        "literal-s5": {"inherits": "literal-s1", "size": 16.5, "line_height": 1.17},
        "orig": {"weight": 600, "size": 12.4, "line_height": 1.4, "balance": True, "space_before": "0.17in"},
        "word": {"weight": 600, "size": 13, "line_height": 1.32},
        "word-long": {"weight": 600, "size": 11, "line_height": 1.32},
        # the gloss sits 0.012 in under the word: a table cell has no own margin, so the gap is extra leading
        "gloss": {"italic": True, "size": 7.7, "line_height": f"{7.7 * 1.25 + 2 * 0.012 * IN:.3f}pt", "color": "grey"},
        "gloss-long": {"italic": True, "size": 7, "line_height": f"{7 * 1.25 + 2 * 0.012 * IN:.3f}pt", "color": "grey"},
        "roman": {"italic": True, "size": 9.3, "line_height": 1.35, "color": "grey", "balance": True, "space_before": "0.02in"},
        # the rule above is 0.75 pt; the text starts 0.13 in + the empty 0.075 in kicker below it
        "note": {"size": 9.9, "line_height": 1.47, "rule_above": {"stroke": INK_RULE, "gap": r2(0.205 * IN + 0.375)}},
        # chapter opener
        "opener-num": {"font": label, "weight": 700, "size": 10.5, "tracking": "0.26em", "case": "upper", "line_height": 1},
        "opener-title": {"font": "display", "size": 60, "line_height": 0.98, "tracking": "-0.012em", "balance": True},
        "lede": {"size": 11.6, "line_height": 1.5, "space_after": "0.1in"},
        "cast-label": {"font": label, "weight": 700, "size": 7.6, "line_height": 1.75, "tracking": "0.24em", "case": "upper", "space_after": "0.03in"},
        "cast": {"font": label, "weight": 600, "size": 7.6, "line_height": 1.75, "tracking": "0.17em", "case": "upper"},
        # "also said" and chart pages
        "kicker": {"font": label, "weight": 700, "size": 9.2, "tracking": "0.24em", "case": "upper", "line_height": 1},
        "also-h2": {"font": "display", "size": 23, "line_height": 1.08, "balance": True},
        "row-label": {"font": label, "weight": 700, "size": 7.4, "line_height": 1.3, "tracking": "0.17em", "case": "upper"},
        "row-where": {"font": label, "weight": 500, "size": 6.6, "line_height": 1.3, "tracking": "0.14em", "case": "upper", "color": "grey"},
        "saying": {"font": "display", "size": 12.6, "line_height": 1.2, "balance": True},
        "row-orig": {"size": 8.9, "line_height": 1.4, "color": "orig", "space_before": "0.025in"},
        "row-note": {"size": 8.5, "line_height": 1.4, "italic": True, "color": "grey", "space_before": "0.02in"},
        "tail": {"size": 9.6, "line_height": 1.45, "italic": True},
        "what": {"font": "display", "line_height": 1, "tracking": "-0.01em"},
        "says": {"font": label, "weight": 600, "size": 7.7, "line_height": 1.7, "tracking": "0.16em", "case": "upper", "space_before": "0.07in"},
        "chart-note": {"size": 8.5, "line_height": 1.4, "italic": True, "color": "grey", "space_before": "0.03in", "margin_right": r2(BOX_W - 4.3 * IN)},
        # index
        "index-kicker": {"font": label, "weight": 700, "size": 8.6, "tracking": "0.24em", "case": "upper", "color": "grey", "line_height": 1, "space_after": "0.16in"},
        "index-h2": {"font": "display", "size": 25, "line_height": 1.08, "space_after": "0.18in"},
        "index-sec": {"font": label, "weight": 700, "size": 8.6, "tracking": "0.24em", "case": "upper", "line_height": 1,
                      "rule_above": {"stroke": INK_RULE, "gap": r2(0.12 * IN + 0.375)}, "space_before": r2(0.18 * IN + 0.375), "space_after": "0.1in"},
        "index-entry": {"size": 10.6, "line_height": 1.52, "hanging": "0.12in", "inset": {"bottom": "0.12in"}, "keep_together": True},
    }
    for n, size in {10: 62, 4: 40, 3: 33, 2: 27, 1: 21}.items():
        styles[f"what-n{n}"] = {"inherits": "what", "size": size}
    for lang, (size, lh) in WORD.items():
        styles[f"word-{lang}"] = {"inherits": "word", "size": size, "line_height": lh, "lang": lang,
                                  **({"font": [CJK[lang], "serif"]} if lang in CJK else {})}
    if not BALANCE:
        for st in styles.values():
            st.pop("balance", None)
    spans = {
        "head-b": {"weight": 700, "color": "ink"},
        "says-word": {"weight": 500, "color": "grey"},
        "index-key": {"font": label, "weight": 700, "size": 7.6, "tracking": "0.12em", "case": "upper", "color": "ink"},
        "pp": {"color": "grey"},
        "flip": {"italic": False, "color": "orig"},           # italic inside italic turns upright (.n i)
        "roman": {"italic": True, "color": "grey"},
    }
    foot_y = TRIM_H - 0.4 * IN                               # bottom of the foot box
    folio = lambda side, colour=None: {
        "type": "text", "x": box_x(side), "y": r2(foot_y - 8.6), "w": BOX_W, "style": "folio", "when": side,
        "set": dict({"align": "right", "margin_right": "0.04em"} if side == "recto" else {"align": "left"}, **({"color": colour} if colour else {})),
        "content": [{"field": "page"}]}
    run = lambda side: {
        "type": "text", "x": box_x(side), "y": r2(foot_y - 6.6), "w": BOX_W, "style": "runfoot", "when": side,
        "set": {"align": "left"} if side == "recto" else {"align": "right", "margin_right": "0.2em"}, "text": TITLE}
    return {
        "colors": {"ink": "#231f1c", "cream": "#f7eed8", "grey": "#6a635c", "hair": "#cfc8bd", "foot": "#8b847c",
                   "orig": "#3d3833", "paper": "#ffffff"},
        "fonts": {
            # the book's composite "Book Serif": Latin from Noto Serif, other scripts from their Noto faces
            "serif": ["Noto Serif", "Noto Naskh Arabic", "Noto Serif Hebrew", "Noto Serif Armenian", "Noto Serif Georgian",
                      "Noto Serif Devanagari", "Noto Serif Bengali", "Noto Serif Tamil", "Noto Serif Thai", "Noto Serif Ethiopic",
                      "Noto Serif SC"],
            "display": "Young Serif",
            "label": "Archivo Narrow",
        },
        "base": {"font": "serif", "size": 10, "line_height": 1.42, "color": "ink", "hyphenate": False, "lang": "en"},
        "styles": styles,
        "spans": spans,
        "masters": {
            "page": {"width": "6in", "height": "9in", "bleed": "0.125in", "fill": "paper",
                     "margins": {"top": "0.4in", "bottom": "0.4in", "inside": "0.56in", "outside": "0.44in"},
                     "foreground": [folio("recto"), folio("verso"), run("recto"), run("verso")]},
            # openers: a folio at 86 % opacity, no running foot
            "opener": {"inherits": "page", "foreground": [folio("recto", "#231f1cdb"), folio("verso", "#231f1cdb")]},
        },
    }


# ---------------------------------------------------------------- page kinds
def running_head(side: str, label: str, head: str) -> dict:
    st = {"align": "right", "margin_right": "0.17em"} if side == "recto" else {"align": "left"}
    return {"type": "text", "id": f"head-{side}-{label.replace(' ', '').replace('.', '')}", "x": box_x(side), "y": r2(0.4 * IN), "w": BOX_W,
            "style": "head", "set": st, "content": head_runs(label, head)}


def chip(x: float, y: float, colour: str, d: float = 0.115 * IN) -> dict:
    return {"type": "ellipse", "x": r2(x), "y": r2(y), "w": r2(d), "h": r2(d), "fill": colour}


def literal_style(text: str) -> str:
    n = len(plain(text))
    return "literal-s1" if n <= 19 else "literal-s2" if n <= 34 else "literal-s3" if n <= 60 else "literal-s4" if n <= 96 else "literal-s5"


WORD = {  # .strip .w b per script: size, line-height
    "zh": (15.5, 1.25), "ja": (15.5, 1.25), "ko": (15.5, 1.25), "yue": (15.5, 1.25),
    "ar": (16.5, 1.5), "fa": (16.5, 1.5), "ur": (16.5, 1.5), "he": (14.4, 1.32), "yi": (14.4, 1.32),
    "th": (14, 1.55), "hi": (13.6, 1.5), "bn": (13.6, 1.5), "ne": (13.6, 1.5), "ta": (12.8, 1.5), "am": (12.8, 1.5),
}


def strip_long(entry: dict) -> bool:
    lang, words = entry.get("lang", ""), entry["words"]
    return sum(len(w[0]) for w in words) > (20 if lang in CJK else 46) or len(words) > 9


def strip(entry: dict) -> dict:
    """Word-for-word: columns of a word over its centred gloss; RTL runs right to left, packed left.
    A two-row table: auto columns as wide as the wider of word and gloss, the gap as each cell's right inset."""
    lang = entry.get("lang", "")
    long = strip_long(entry)
    order = list(reversed(entry["words"])) if lang in RTL else entry["words"]
    word_style = "word-long" if long else f"word-{lang}" if lang in WORD else "word"
    gloss_style = "gloss-long" if long else "gloss"
    return {"type": "table", "columns": ["auto"] * len(order), "stroke": "none",
            "rows": [[{"text": apos(o), "style": word_style, "align": "center"} for o, _ in order],
                     [{"text": apos(g), "style": gloss_style, "align": "center"} for _, g in order]],
            "inset": {"top": 0, "bottom": 0, "left": 0, "right": r2((0.1 if long else 0.13) * IN)},
            "space_before": "0.2in", "space_after": 0}


def plate_page(ch: dict, entry: dict, number: int) -> dict:
    side = "recto" if number % 2 else "verso"
    x = box_x(side)
    ground = ch["ground"]
    els = [running_head(side, f"No. {ch['no']}", ch["head"])]
    plate_y = 0.66 * IN
    els.append({"type": "rect", "x": x, "y": r2(plate_y), "w": BOX_W, "h": 240, "fill": ground})
    els.append({"type": "image", "id": "plate", "src": f"book/visuals/art/print/{entry['art']}.jpg", "x": x, "y": r2(plate_y),
                "w": BOX_W, "h": 240, "fit": "stretch"})
    # the entry column: 4.3 in from the trim top to 0.74 in above the trim bottom
    top, bottom = 4.3 * IN, TRIM_H - 0.74 * IN
    chip_d = 0.115 * IN
    els.append(chip(x, top + (9.2 - chip_d) / 2, ground))
    lang_runs = [entry["language"]]
    if entry.get("where"):
        lang_runs.append({"text": " " + entry["where"], "weight": 500, "color": "grey", "tracking": "0.16em"})
    els.append({"type": "text", "id": "lang", "x": r2(x + chip_d + 0.09 * IN), "y": r2(top), "w": r2(BOX_W - chip_d - 0.09 * IN),
                "style": "lang", "content": lang_runs})
    blocks = [{"type": "p", "id": "literal", "style": literal_style(entry["literal"]), "content": runs(entry["literal"])}]
    if entry.get("words"):
        blocks.append(strip(entry))
    elif entry.get("original"):
        o = {"type": "p", "style": "orig", "text": apos(entry["original"]), "set": {"lang": entry.get("lang", "en")}}
        blocks.append(o)
    if entry.get("roman"):
        blocks.append({"type": "p", "style": "roman", "text": nobreak_end(entry["roman"]),
                       **({"set": {"space_before": r2((0.09 - 0.012) * IN)}} if entry.get("words") else {})})
    lit_top = top + 9.2 + 0.13 * IN
    els.append({"type": "text", "id": "entry", "x": x, "y": r2(lit_top), "w": BOX_W, "h": r2(bottom - lit_top - 60), "blocks": blocks})
    # the note sits at the bottom of the column (margin-top: auto)
    note_top = lit_top + 120
    els.append({"type": "text", "id": "note", "x": x, "y": r2(note_top), "w": BOX_W, "h": r2(bottom - note_top), "valign": "bottom",
                "style": "note", "content": runs(entry["note"])})
    return {"type": "page", "id": f"p{number}", "master": "page", "layers": True, "page_from": number, "elements": els}


def opener_page(ch: dict, number: int) -> dict:
    side = "recto"
    x = box_x(side)
    ground = ch["ground"]
    on = "cream" if luminance(ground) < 0.2 else "ink"
    title = ch["title"]
    size = 60 if len(title) <= 13 else 50 if len(title) <= 22 else 42
    langs: list[str] = []
    for p in ch["pages"]:
        if p["language"] not in langs:
            langs.append(p["language"])
    also: list[str] = []
    for a in ch.get("also", []):
        if a["language"] not in langs and a["language"] not in also:
            also.append(a["language"])
    cast = "\u00a0\u00b7 ".join(name.replace(" ", "\u00a0") for name in langs)
    if also:
        cast += f"\u00a0\u00b7 and\u00a0{len(also)}\u00a0more"
    numeral_size = 400
    # .numeral { right: -0.12in; bottom: -1.08in; line-height: 1 } -> the baseline sits 0.84 em below the line top
    # (Young Serif ascent 1.046, descent 0.366). The glyph run is 1.046 em wide with Chrome's trailing tracking.
    num_w = numeral_width(str(ch["no"]))
    line_bottom = TRIM_H + 1.08 * IN
    num_x = 6 * IN + 0.12 * IN - num_w * numeral_size
    baseline = line_bottom - numeral_size + 0.84 * numeral_size
    els = [
        {"type": "lettering", "id": "numeral", "text": str(ch["no"]), "font": "display", "size": numeral_size, "tracking": "-0.04em",
         "features": ["lnum"], "color": mix_black(ground, 0.91), "x": r2(num_x), "y": r2(baseline - 1.046 * numeral_size), "decorative": True},
        {"type": "text", "id": "opener-num", "x": x, "y": r2(1.55 * IN), "w": BOX_W, "style": "opener-num", "set": {"color": on},
         "text": f"No. {ch['no']}"},
        # an <h1> in a one-weight face: Chrome fakes the bold with a stroke of size/32 in the text colour
        {"type": "text", "id": "opener-title", "x": x, "y": r2(1.9 * IN), "w": BOX_W, "style": "opener-title",
         "set": {"size": size, "color": on, "outline": {"width": r2(size / 32), "color": on, "join": "round"}}, "content": runs(title)},
        {"type": "rect", "x": x, "y": r2(4.34 * IN), "w": r2(0.5 * IN), "h": 1.5, "fill": on},
        {"type": "text", "id": "lede", "x": x, "y": r2(4.62 * IN), "w": r2(4.25 * IN), "set": {"color": on},
         "blocks": [{"type": "p", "style": "lede", "content": runs(p)} for p in ch["lede"]]},
        {"type": "text", "id": "cast", "x": x, "y": r2(TRIM_H - 0.78 * IN - 80), "w": r2(4.6 * IN), "h": 80, "valign": "bottom",
         "set": {"color": "#231f1ce6" if on == "ink" else "#f7eed8e6"},
         "blocks": [{"type": "p", "style": "cast-label", "text": "In this chapter"}, {"type": "p", "style": "cast", "text": cast}]},
    ]
    return {"type": "page", "id": f"p{number}", "master": "opener", "layers": True, "page_from": number, "fill": ground, "elements": els}


def numeral_width(text: str) -> float:
    """The numeral's advance in em, read from the book's own outline cache (design/numerals.json viewBox)."""
    svg = json.loads((BOOK / "design/numerals.json").read_text())[text]
    vb = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', svg)
    return float(vb.group(1)) / float(vb.group(2))


def title_block(side: str, colour: str, kicker: str, title: str) -> list:
    x = box_x(side)
    top = 0.72 * IN
    chip_d = 0.115 * IN
    return [
        chip(x, top + (9.2 - chip_d) / 2, colour),
        {"type": "text", "id": "kicker", "x": r2(x + chip_d + 0.09 * IN), "y": r2(top), "w": 300, "style": "kicker", "content": runs(kicker)},
        {"type": "text", "id": "title", "x": x, "y": r2(top + 9.2 + 0.12 * IN), "w": BOX_W, "style": "also-h2", "content": runs(title)},
    ]


def also_page(ch: dict, number: int) -> dict:
    side = "recto" if number % 2 else "verso"
    x = box_x(side)
    els = [running_head(side, ch.get("head_label") or f"No. {ch['no']}", ch["head"])]
    els += title_block(side, ch["ground"], ch.get("also_kicker", "Also said"), ch.get("also_title", "The rest of the world"))
    rows = []
    for a in ch["also"]:
        # the row's top hairline and padding: a rule above each cell's first paragraph (per-side table strokes: GAPS.md)
        label = [{"type": "p", "style": "row-label", "text": a["language"],
                  "set": {"rule_above": {"stroke": HAIR, "gap": r2(0.375 + 0.095 * IN + 0.045 * IN)}}}]
        if a.get("where"):
            label.append({"type": "p", "style": "row-where", "text": a["where"]})
        body = [{"type": "p", "style": "saying", "content": runs(a["literal"]),
                 "set": {"rule_above": {"stroke": HAIR, "gap": r2(0.375 + 0.095 * IN)}}}]
        if a.get("original"):
            lang = a.get("lang", "")
            orig = {"text": apos(a["original"]), "lang": lang}
            if lang in CJK:
                orig.update(font=[CJK[lang], "serif"], size=10)
            elif lang in {"ar", "fa", "ur"}:
                orig.update(size=11.6)
            elif lang in {"he", "yi", "th", "hi", "bn"}:
                orig.update(size=10)
            content = [orig]
            if a.get("roman"):
                content += [" ", {"text": nobreak_end(a["roman"]), "span": "roman"}]
            body.append({"type": "p", "style": "row-orig", "content": content})
        if a.get("note"):
            body.append({"type": "p", "style": "row-note", "content": runs(a["note"], {"span": "flip"})})
        rows.append([{"blocks": label}, {"blocks": body}])
    list_top = 1.78 * IN
    bottom = TRIM_H - 0.74 * IN
    table = {"type": "table", "columns": [r2(1.02 * IN + 0.14 * IN), "1fr"], "rows": rows, "stroke": "none",
             # rule centre to rule centre: half the hairline, the padding, the content, the padding, half the next hairline
             "inset": {"top": 0, "bottom": r2(0.1 * IN + 0.375), "left": 0, "right": 0}, "space_before": 0, "space_after": 0}
    els.append({"type": "text", "id": "list", "x": x, "y": r2(list_top + 0.375), "w": BOX_W, "blocks": [table, closing_rule(0, 0)]})
    if ch.get("tail"):
        els.append({"type": "text", "id": "tail", "x": x, "y": r2(bottom - 60), "w": r2(4.4 * IN), "h": 60, "valign": "bottom",
                    "style": "tail", "content": runs(ch["tail"], {"italic": False})})
    return {"type": "page", "id": f"p{number}", "master": "page", "layers": True, "page_from": number, "elements": els}


def closing_rule(before: float, at: float = 0.375) -> dict:
    """The hairline under the last row: a line in a canvas of no height."""
    return {"type": "canvas", "h": 0, "space_before": r2(before), "space_after": 0,
            "elements": [{"type": "line", "x1": 0, "y1": at, "x2": BOX_W, "y2": at, "stroke": HAIR}]}


def sup_raise(size: float) -> float:
    """vertical-align: top + 0.04 in for an 8 pt Archivo count beside a heading at line-height 1."""
    heading_baseline = (size - (1.046 + 0.366) * size) / 2 + 1.046 * size
    sup_baseline = 0.04 * IN + (8 - (1.035 + 0.312) * 8) / 2 + 1.035 * 8
    return r2(heading_baseline - sup_baseline)


def chart_page(block: dict, part: dict, number: int) -> dict:
    side = "recto" if number % 2 else "verso"
    x = box_x(side)
    ink = "#231f1c"
    els = [running_head(side, block["kicker"], block["head"])]
    els += title_block(side, ink, part["kicker"], part["title"])
    groups = []
    for i, g in enumerate(part["groups"]):
        n = min(len(g["who"]), 10)
        size = {10: 62, 4: 40, 3: 33, 2: 27, 1: 21}.get(n, 21)
        what = runs(g["what"])
        if len(g["who"]) > 1:
            what += [{"text": " ", "font": "label", "size": 8, "word_spacing": "2pt"},
                     {"text": str(len(g["who"])), "font": "label", "weight": 700, "size": 8, "tracking": "0.06em", "color": "grey",
                      "baseline": sup_raise(size)}]
        says = [{"text": "says ", "span": "says-word", "word_spacing": r2(0.04 * IN)}, " \u00a0\u00b7\u00a0 ".join(g["who"])]
        # padding 0.12 in under a 0.75 pt top rule; the previous group's 0.13 in bottom padding is the space before
        groups.append({"type": "p", "style": f"what-n{n}", "content": what,
                       "set": {"rule_above": {"stroke": HAIR, "gap": r2(0.375 + 0.12 * IN)}, **({"space_before": r2(0.13 * IN + 0.375)} if i else {})}})
        groups.append({"type": "p", "style": "says", "content": says})
        if g.get("note"):
            groups.append({"type": "p", "style": "chart-note", "content": runs(g["note"], {"span": "flip"})})
    groups.append(closing_rule(0.13 * IN))
    els.append({"type": "text", "id": "chart", "x": x, "y": r2(1.78 * IN + 0.375), "w": BOX_W, "blocks": groups})
    return {"type": "page", "id": f"p{number}", "master": "page", "layers": True, "page_from": number, "elements": els}


def index_rows(keys: list[str]) -> list[dict]:
    out = []
    for key in sorted((k for k in keys if k in THINGS), key=str.casefold):
        pages = ", ".join(map(str, sorted(set(THINGS[key]))))
        # The engine tracks the key's last letter too, as CSS letter-spacing does.
        out.append({"type": "p", "style": "index-entry",
                    "content": [{"text": key, "span": "index-key"}, " ", {"text": pages, "span": "pp"}]})
    return out


def index_page(block: dict, number: int) -> dict:
    """Kicker, title, then per section a ruled head and the keys in four balanced columns, all one flow."""
    side = "recto" if number % 2 else "verso"
    x = box_x(side)
    blocks = [{"type": "p", "style": "index-kicker", "content": runs(block["kicker"])},
              {"type": "p", "style": "index-h2", "content": runs(block["title"])}]
    for i, sec in enumerate(block["sections"]):
        # after a section the columns end with their last entry's 0.12 in gap, which CSS drops at a column's end
        blocks.append({"type": "p", "style": "index-sec", "content": runs(sec["title"]), **({"set": {"space_before": r2(0.22 * IN - 0.12 * IN + 0.375)}} if i else {})})
        blocks.append({"type": "columns", "count": 4, "gutter": "0.16in", "balance": True, "blocks": index_rows(sec["keys"])})
    els = [{"type": "text", "id": "index", "x": x, "y": r2(0.9 * IN), "w": BOX_W, "blocks": blocks}]
    return {"type": "page", "id": f"p{number}", "master": "page", "layers": True, "start": "verso" if side == "verso" else "recto",
            "page_from": number, "elements": els}


def prefix_ids(page: dict) -> dict:
    """Ids are unique across the document: element ids get the page's id in front."""
    def walk(node):
        if isinstance(node, dict):
            if "id" in node and node is not page:
                node["id"] = f"{page['id']}-{node['id']}"
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    walk(page)
    return page


def main() -> None:
    chapters = {c["id"]: c for c in DATA["chapters"]}
    inter = DATA["interlude"]
    care = chapters["care"]
    entry = next(e for e in care["pages"] if e["art"] == "care-ar")
    things_block = next(b for b in DATA["back"] if b["id"] == "index-things")
    built = {
        PAGES["care-ar"]: plate_page(care, entry, PAGES["care-ar"]),
        PAGES["greek-chart-1"]: chart_page(inter, inter["chart"][0], PAGES["greek-chart-1"]),
        PAGES["devil-opener"]: opener_page(chapters["devil"], PAGES["devil-opener"]),
        PAGES["rain-also"]: also_page(chapters["rain"], PAGES["rain-also"]),
        PAGES["index-things"]: index_page(things_block, PAGES["index-things"]),
    }
    content = [prefix_ids(built[p["original"]]) for p in SAMPLE["pages"]]
    doc = {
        "matenadar": "1",
        "meta": {"title": TITLE, "author": CONFIG["author"], "lang": "en-US"},
        "fonts": ["book/design/fonts"],
        "design": design(),
        "content": content,
    }
    (HERE / "doc.json").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("doc.json:", ", ".join(f"p{p['original']}" for p in SAMPLE["pages"]))


if __name__ == "__main__":
    main()
