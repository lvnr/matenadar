#!/usr/bin/env python3
"""One Pen: Winter Lights from the coloring template: four pictures, each on a right-hand page with a blank back.

    python3 samples/coloring/make.py      # writes spec.json and doc.json beside this file

The pictures are the book's own activity pages (coloring/pNN-*/pNN-*.activity.svg): whole 8.5 x 11 in pages with
their own white edge and frame, so they go in as `full` pages at the trim. Title, subtitle and author are the
book's (book.json), the copyright line its front matter's (manuscript/00-frontmatter.md), the note on each back
its instructions (pNN-*.coloring.json). The book was set in Adobe Caslon Pro, which this computer has.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
PAGES = ["p01-the-lamplighter", "p02-oranges-under-the-awning", "p03-the-skating-pond", "p04-tram-number-seven"]


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    book = json.loads((BOOK / "book.json").read_text())
    front = (BOOK / "manuscript/00-frontmatter.md").read_text()
    copyright = next(l.strip() for l in front.splitlines() if l.startswith("Copyright"))
    first = json.loads((BOOK / f"coloring/{PAGES[0]}/{PAGES[0]}.coloring.json").read_text())
    spec = {
        "trim": "8.5x11",
        "title": book["title"],
        "subtitle": book["subtitle"].split(":")[0],
        "author": book["author"],
        "copyright": copyright,
        "back_note": first["instructions"],
        "test_page": "Try your pen here",
        "pages": [{"id": p, "src": f"book/coloring/{p}/{p}.activity.svg", "full": True,
                   "alt": json.loads((BOOK / f"coloring/{p}/{p}.coloring.json").read_text())["title"]} for p in PAGES],
        "fonts": {"display": "Adobe Caslon Pro", "text": "Adobe Caslon Pro"},
        "design": {"styles": {"title": {"weight": 600, "size": 46}, "subtitle": {"italic": True}}},
    }
    (HERE / "spec.json").write_text(json.dumps(spec, indent=1, ensure_ascii=False) + "\n")
    engine = ROOT / "bin/matenadar"
    if not engine.exists():
        engine = ROOT / "target/debug/matenadar"
    subprocess.run([str(engine), "template", "coloring", str(HERE / "spec.json"), "-o", str(HERE / "doc.json")], check=True)


if __name__ == "__main__":
    main()
