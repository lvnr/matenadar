#!/usr/bin/env python3
"""Black ink: two pages of a black-ink interior showing one of The Attention Repair Manual's colour cover
designs (visuals/accepted/cover-04b-r1.png), as it might appear on an "also in print" page.

    python3 samples/black-ink/make.py      # writes doc.json beside this file

Page 1 has the picture as delivered, and its heading in a blue: `matenadar check` reports both
(color-in-black-ink). Page 2 has the same picture through the grey filter (Rec. 709 luma and a contrast
curve) and its heading in the ink: nothing to report.
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
ART = "book/visuals/accepted/cover-04b-r1.png"


def page(pid: str, heading: str, colour: str, note: str, filt: dict | None) -> dict:
    picture = {"type": "image", "id": f"{pid}-cover", "src": ART, "x": "1.6in", "y": "1.75in", "w": "2.8in", "h": "4.2in", "fit": "contain",
               "alt": "The cover of The Attention Repair Manual: black type on yellow, with a blue panel."}
    if filt is not None:
        picture["filter"] = filt
    return {"type": "page", "id": pid, "master": "page", "layers": True, "elements": [
        {"type": "text", "id": f"{pid}-heading", "x": "0.85in", "y": "0.9in", "w": "4.5in", "style": "heading", "text": heading,
         "set": {"color": colour}},
        picture,
        {"type": "text", "id": f"{pid}-note", "x": "0.85in", "y": "6.3in", "w": "4.5in", "style": "note", "text": note},
    ]}


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    doc = {
        "matenadar": "1",
        "meta": {"title": "The Attention Repair Manual", "author": "Levon Arakelyan", "lang": "en-US"},
        "print": {"ink": "black"},
        "design": {
            "colors": {"ink": "#1a1a1a"},
            "base": {"font": "Libertinus Serif", "size": 10.5, "line_height": "14.5pt", "color": "ink"},
            "styles": {"heading": {"size": 16, "weight": "semibold", "align": "center"},
                       "note": {"size": 9.5, "italic": True, "align": "center"},
                       "folio": {"size": 8.5, "align": "center"}},
            "masters": {"page": {"width": "6in", "height": "9in", "bleed": "0.125in",
                                 "margins": {"top": "0.75in", "bottom": "0.85in", "inside": "0.85in", "outside": "0.65in"},
                                 "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": "0.85in", "y": "8.45in", "w": "4.5in",
                                                 "content": [{"field": "page"}]}]}},
        },
        "content": [
            page("delivered", "Also in print", "#1f4fa0", "As delivered: a colour picture and a blue heading on a black-ink page.", None),
            page("filtered", "Also in print", "ink", "Through the grey filter: luma with a contrast curve, the heading in the ink.",
                 {"gray": True, "contrast": 3, "black": 0.02, "white": 0.98}),
        ],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
