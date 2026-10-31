#!/usr/bin/env python3
"""A week of Leave Notes from The Attention Repair Manual, as journal pages.

    python3 samples/journal/make.py      # writes doc.json beside this file

The four prompts and the line under the Leave Note's heading are the book's own (manuscript/06-field-guide.md,
"Bench tools"). The page is the book's 6 x 9 in; the book set Warnock Pro and Halyard, which this computer doesn't
have, so Minion Pro and Myriad Pro (Adobe's, near cousins) stand in, falling back to Libertinus Serif.

One page in the document makes the seven dated pages: `repeat` lays it out once a day from Monday 2 November 2026,
each time with its date and its number, and the heading's field shows the date as words. A calendar page opens the
week, and a dot-grid and a graph-paper page close it.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
W, H = 432.0, 648.0
M = {"top": 54.0, "bottom": 58.0, "inside": 63.0, "outside": 50.0}


def leave_note() -> tuple[str, list[str]]:
    """The Leave Note's instruction line and its four prompts, from the field guide."""
    text = (BOOK / "manuscript/06-field-guide.md").read_text()
    part = text.split("### Leave Note", 1)[1].split("###", 1)[0]
    lines = [l.strip() for l in part.splitlines()]
    intro = next(l for l in lines if l and not l.startswith(">"))
    prompts = [re.sub(r"\*\*|>", "", l).strip() for l in lines if l.startswith("> **")]
    if len(prompts) != 4:
        raise SystemExit(f"expected the Leave Note's four prompts, found {prompts}")
    return intro, prompts


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    intro, prompts = leave_note()
    x, w = M["outside"], W - M["inside"] - M["outside"]

    def col(recto: bool) -> float:
        return M["inside"] if recto else M["outside"]

    # The dated page: date and day at the top, then each prompt over ruled lines.
    day = [
        {"type": "text", "id": "day-kicker", "x": M["inside"], "y": M["top"], "w": w, "style": "kicker",
         "content": ["Leave Note · day ", {"field": "section.n"}]},
        {"type": "text", "id": "day-date", "x": M["inside"], "y": M["top"] + 16, "w": w, "style": "date",
         "content": [{"field": "section.date", "format": "[weekday], [day padding:none] [month repr:long]"}]},
        {"type": "text", "id": "day-intro", "x": M["inside"], "y": M["top"] + 48, "w": w, "style": "intro", "text": intro},
    ]
    y = M["top"] + 86
    block = (H - M["bottom"] - y) / 4
    for i, p in enumerate(prompts):
        day.append({"type": "text", "id": f"day-prompt-{i + 1}", "x": M["inside"], "y": y, "w": w, "style": "prompt", "text": p})
        day.append({"type": "ruling", "id": f"day-lines-{i + 1}", "x": M["inside"], "y": y + 14, "w": w, "h": block - 22, "pitch": 22,
                    "stroke": {"width": 0.5, "color": "rule"}})
        y += block
    content = [
        {"type": "page", "id": "month", "master": "page", "layers": True, "elements": [
            {"type": "text", "id": "month-kicker", "x": col(True), "y": M["top"], "w": w, "style": "kicker", "text": "Four weeks"},
            {"type": "text", "id": "month-title", "x": col(True), "y": M["top"] + 16, "w": w, "style": "date", "text": "November 2026"},
            {"type": "text", "id": "month-how", "x": col(True), "y": M["top"] + 48, "w": w, "style": "intro",
             "text": "Tick each day you wrote a note. Leave the boxes of days without one empty: they count too."},
            {"type": "calendar", "id": "month-grid", "x": col(True), "y": M["top"] + 96, "w": w, "h": 300, "year": 2026, "month": 11,
             "style": "day-number", "header_style": "weekday", "outside": True, "lines": {"width": 0.6, "color": "rule"}},
        ]},
        {"type": "page", "id": "day", "master": "page", "layers": True, "start": "verso",
         "repeat": {"count": 7, "dates": {"from": "2026-11-02"}}, "elements": day},
        {"type": "page", "id": "notes", "master": "page", "layers": True, "elements": [
            {"type": "text", "id": "notes-title", "x": col(False), "y": M["top"], "w": w, "style": "kicker", "text": "Notes"},
            {"type": "ruling", "id": "notes-dots", "x": col(False), "y": M["top"] + 22, "w": w, "h": H - M["top"] - M["bottom"] - 30, "pattern": "dots",
             "pitch": "5mm", "stroke": {"width": 1.1, "color": "#9a9a9a"}},
        ]},
        {"type": "page", "id": "guesses", "master": "page", "layers": True, "elements": [
            {"type": "text", "id": "guesses-title", "x": col(True), "y": M["top"], "w": w, "style": "kicker", "text": "Two guesses"},
            {"type": "ruling", "id": "guesses-grid", "x": col(True), "y": M["top"] + 22, "w": w, "h": H - M["top"] - M["bottom"] - 30, "pattern": "graph",
             "pitch": "0.1in", "major": 5, "stroke": {"width": 0.35, "color": "#b4b4b4"}, "major_stroke": {"width": 0.7, "color": "#8c8c8c"}},
        ]},
    ]
    doc = {
        "matenadar": "1",
        "meta": {"title": "The Leave Note", "author": "The Attention Repair Manual", "lang": "en-US"},
        "design": {
            "colors": {"ink": "#1d1d1b", "soft": "#6e6a64", "rule": "#a8a49c"},
            "fonts": {"serif": ["Minion Pro", "Libertinus Serif"], "sans": ["Myriad Pro", "Libertinus Serif"]},
            "base": {"font": "serif", "size": 10.5, "line_height": 1.35, "color": "ink"},
            "styles": {
                "kicker": {"font": "sans", "size": 8.5, "case": "upper", "tracking": "0.14em", "color": "soft"},
                "date": {"font": "serif", "size": 22, "line_height": 1.15},
                "intro": {"font": "serif", "size": 10.5, "italic": True, "color": "soft"},
                "prompt": {"font": "sans", "size": 9.5, "weight": 600},
                "weekday": {"font": "sans", "size": 8, "case": "upper", "tracking": "0.08em", "color": "soft"},
                "day-number": {"font": "sans", "size": 9},
                "folio": {"font": "sans", "size": 8, "align": "center", "color": "soft"},
            },
            "masters": {"page": {
                "width": f"{W}pt", "height": f"{H}pt",
                "margins": {k: f"{v}pt" for k, v in M.items()},
                "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": M["outside"], "y": H - 36, "w": W - 2 * M["outside"],
                                "content": [{"field": "page"}]}],
            }},
        },
        "content": content,
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
