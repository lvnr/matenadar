#!/usr/bin/env python3
"""A 32-page picture book from the picture-book template.

    python3 samples/picture-book/make.py      # writes spec.json, art/dig-site.svg and doc.json beside this file

The story pages are Whynarium: How Dinosaurs Worked's own lettered, full-bleed pages (8.5 x 11 in), from the book's
page manifest (release/how-dinosaurs-worked-interior-page-manifest.json), each on a page of the same side as in
the book. Pages 4-5 are a spread drawn here as vector art, with words typeset over it on request. The title,
copyright and back matter are typeset from the book's own words (release/copyright-page.md and the description,
release/kdp-description.html). Cooper Hewitt (SIL OFL, the series' lettering face) is copied to build/fonts when
the system has it; otherwise the template falls back to Libertinus Serif.
"""
from __future__ import annotations

import html
import json
import re
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
MANIFEST = json.loads((BOOK / "release/how-dinosaurs-worked-interior-page-manifest.json").read_text())
ALT = json.loads((BOOK / "visuals/alt-text.json").read_text())

# The book's pages that keep their place: page n of the sample is page n of the book.
KEEP = [3] + list(range(6, 31))


def page_art(n: int) -> str:
    entry = MANIFEST["pages"][n - 1]
    return "book/" + entry["asset_path"]


def page_alt(n: int) -> str:
    return ALT.get(MANIFEST["pages"][n - 1]["asset_path"], "")


def copyright_words() -> str:
    """The copyright page's first paragraphs (release/copyright-page.md)."""
    paras = [p.strip() for p in (BOOK / "release/copyright-page.md").read_text().split("\n\n") if p.strip() and not p.startswith("#")]
    keep = [p for p in paras if p.startswith(("Text and illustrations", "No part", "First edition"))]
    return "\n".join(keep)


def grown_ups() -> str:
    paras = [p.strip() for p in (BOOK / "release/copyright-page.md").read_text().split("\n\n")]
    return "\n\n".join(p for p in paras if p.startswith(("A note for grown-ups", "The science is simplified")))


def description() -> list[str]:
    text = (BOOK / "release/kdp-description.html").read_text()
    lines = [html.unescape(l).strip() for l in re.sub(r"<[^>]+>", "\n", text).splitlines()]
    return [l for l in lines if l]


def dig_site() -> str:
    """A badlands dig site at sunrise, 17 x 11.25 in (both pages and their outer bleeds), in hundredths of an
    inch. The fold is at x = 850; nothing that matters sits within 60 of it."""
    import math

    w, h = 1700, 1125

    def wavy(y0: float, amp: float, phase: float) -> str:
        pts = [f"{x} {y0 + amp * math.sin(x / 130 + phase) + amp * 0.5 * math.sin(x / 47 + phase * 2):.1f}" for x in range(0, w + 1, 25)]
        return "L " + " L ".join(pts)

    strata = ["#b8644a", "#e3b27c", "#c97e55", "#f0d6a6", "#a35f4f", "#e9c08b", "#d28f63", "#f3e0b6", "#9a5848", "#ddb07d"]
    bands, y = [], 455
    for i, c in enumerate(strata * 2):
        top = y
        y += 30 + (i * 13) % 26
        bands.append(f'<path d="M 0 {h} L 0 {top} {wavy(top, 4 + i % 3, i * 0.7)} L {w} {h} Z" fill="{c}"/>')
        if y > 900:
            break
    mesa = "M 300 905 L 330 820 L 372 760 L 400 640 L 432 600 L 452 520 L 486 470 L 1215 466 L 1250 520 L 1268 600 L 1305 650 L 1330 760 L 1380 820 L 1430 905 Z"
    far = ("M 0 690 L 70 640 L 80 590 L 230 586 L 245 640 L 310 690 Z M 1330 680 L 1385 625 L 1392 560 L 1560 556 "
           "L 1572 610 L 1640 650 L 1700 640 L 1700 700 L 1330 700 Z")
    clouds = "".join(
        f'<g fill="#fffaf0" opacity="0.75"><ellipse cx="{x}" cy="{y0}" rx="{r * 1.8:.0f}" ry="{r * 0.55:.0f}"/>'
        f'<ellipse cx="{x - r * 0.6:.0f}" cy="{y0 - r * 0.25:.0f}" rx="{r:.0f}" ry="{r * 0.5:.0f}"/>'
        f'<ellipse cx="{x + r * 0.7:.0f}" cy="{y0 - r * 0.3:.0f}" rx="{r * 0.9:.0f}" ry="{r * 0.45:.0f}"/></g>'
        for x, y0, r in [(250, 300, 70), (560, 210, 50), (1120, 250, 60), (1560, 330, 45)]
    )
    bone = lambda x, y0, k, a: (  # noqa: E731
        f'<g transform="translate({x} {y0}) rotate({a}) scale({k})" fill="#fbf3e2" stroke="#7d5240" stroke-width="3">'
        '<rect x="-46" y="-7" width="92" height="14" rx="7"/><circle cx="-48" cy="-9" r="10"/><circle cx="-48" cy="9" r="10"/>'
        '<circle cx="48" cy="-9" r="10"/><circle cx="48" cy="9" r="10"/></g>'
    )
    ammonite = (
        '<g transform="translate({x} {y})" fill="#eedfc0" stroke="#7d5240" stroke-width="4" stroke-linecap="round">'
        '<circle r="34"/><path fill="none" d="M 0 0 m -27 0 a 27 27 0 1 0 54 0 a 20 20 0 1 0 -40 0 a 13 13 0 1 0 26 0 a 6 6 0 1 0 -12 0"/></g>'
    )
    grid = ['<g transform="translate(150 960)"><rect x="0" y="0" width="330" height="120" fill="#e2c38f" stroke="#8a6248" stroke-width="3"/>']
    for gx in range(55, 330, 55):
        grid.append(f'<line x1="{gx}" y1="0" x2="{gx}" y2="120" stroke="#fff6e4" stroke-width="2.5"/>')
    grid.append('<line x1="0" y1="60" x2="330" y2="60" stroke="#fff6e4" stroke-width="2.5"/>')
    grid.append(bone(95, 30, 0.7, -8) + bone(230, 88, 0.8, 14) + ammonite.format(x=280, y=30).replace("r=\"34\"", "r=\"22\""))
    grid.append("</g>")
    shrubs = "".join(
        f'<g fill="#7d8c55"><ellipse cx="{x}" cy="{y0}" rx="{r}" ry="{r * 0.55:.0f}"/><ellipse cx="{x + r * 0.6:.0f}" cy="{y0 - r * 0.25:.0f}" rx="{r * 0.6:.0f}" ry="{r * 0.4:.0f}" fill="#93a265"/></g>'
        for x, y0, r in [(70, 935, 40), (600, 960, 30), (1130, 950, 34), (1640, 940, 46), (1460, 1060, 30), (700, 1090, 26)]
    )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w / 100}in" height="{h / 100}in">
<defs>
  <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#8fbcd4"/><stop offset="0.6" stop-color="#f4d9b3"/><stop offset="1" stop-color="#f8e6c9"/></linearGradient>
  <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#fff3d2"/><stop offset="0.3" stop-color="#ffd88c" stop-opacity="0.85"/><stop offset="1" stop-color="#ffd88c" stop-opacity="0"/></radialGradient>
  <linearGradient id="sand" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e6c58f"/><stop offset="1" stop-color="#f1dcb2"/></linearGradient>
  <clipPath id="mesa"><path d="{mesa}"/></clipPath>
</defs>
<rect width="{w}" height="{h}" fill="url(#sky)"/>
<circle cx="1470" cy="470" r="230" fill="url(#glow)"/>
<circle cx="1470" cy="470" r="78" fill="#ffcc6e"/>
{clouds}
<path d="{far}" fill="#c4a19f" opacity="0.85"/>
<path d="M 0 700 C 300 680 500 720 850 700 C 1200 680 1450 720 1700 700 L 1700 905 L 0 905 Z" fill="#d7b49a"/>
<g clip-path="url(#mesa)">{''.join(bands)}</g>
<path d="{mesa}" fill="none" stroke="#6e4335" stroke-width="4" stroke-linejoin="round"/>
<path d="M 300 905 L 430 860 L 560 885 L 700 850 L 980 880 L 1160 845 L 1320 870 L 1430 905 Z" fill="#cf9d72"/>
<g transform="translate(600 466)"><path d="M -80 0 L 0 -70 L 80 0 Z" fill="#f6f3ea" stroke="#5f3f33" stroke-width="3"/><path d="M 0 -70 L -18 0 L 18 0 Z" fill="#d8ccb4"/>
<line x1="0" y1="-70" x2="0" y2="-118" stroke="#5f3f33" stroke-width="3"/><path d="M 0 -118 L 38 -106 L 0 -94 Z" fill="#e2603f"/></g>
<rect x="0" y="895" width="{w}" height="{h - 895}" fill="url(#sand)"/>
<path d="M 0 905 C 260 885 520 925 850 905 C 1180 885 1440 920 1700 900 L 1700 940 C 1400 955 1150 925 850 945 C 560 965 260 935 0 950 Z" fill="#d9b67f"/>
{''.join(grid)}{shrubs}
{bone(1020, 1030, 1.0, -10)}{bone(1540, 990, 0.9, 20)}{ammonite.format(x=1280, y=1060)}
</svg>
"""


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    fonts = HERE / "build/fonts"
    fonts.mkdir(parents=True, exist_ok=True)
    for face in ["Book", "BookItalic", "Medium", "Bold", "Heavy"]:
        src = Path.home() / f"Library/Fonts/CooperHewitt-{face}.otf"
        if src.exists():
            shutil.copyfile(src, fonts / src.name)
    (HERE / "art/dig-site.svg").write_text(dig_site())

    desc = description()
    story = [{"src": page_art(3), "id": "p03-welcome", "alt": page_alt(3)}]
    story.append({
        "id": "p04-dig-site", "spread": True, "src": "art/dig-site.svg", "gutter": "0.125in",
        "alt": "A striped hill at sunrise, a white tent on its top and old bones in the sand below.",
        "left": {"text": "Long ago, this hill was the muddy bank of a river."},
        "right": {"words": [{
            "type": "text", "id": "p04-dig-site-right-words", "x": "36pt", "y": "684pt", "w": "540pt", "anchor": "bottom-left",
            "style": "story", "safe": True, "text": "Animals came to drink. Some never left, and their bones are still inside the stone.",
            "scrim": {"opacity": 0.82, "feather": "10pt", "inset": "10pt", "radius": "10pt"}}]},
    })
    for n in KEEP[1:]:
        story.append({"src": page_art(n), "id": f"p{n:02d}", "alt": page_alt(n)})
    spec = {
        "trim": "8.5x11",
        "pages": 32,
        "ends": "separate",
        "title": "How Dinosaurs Worked",
        "subtitle": "Answers to Dinosaur Why Questions for Curious Kids",
        "author": "Nell Bramley",
        "publisher": "Whynarium",
        "copyright": copyright_words(),
        "fonts": {"display": "Cooper Hewitt", "text": "Cooper Hewitt"},
        "font_dirs": ["build/fonts"],
        # Cooper Hewitt's files number their weights 701-714 (Thin to Heavy
        # Italic), so the faces are asked for by those numbers.
        "design": {
            "base": {"weight": 705},
            "styles": {"title": {"weight": 711, "size": 46}, "subtitle": {"weight": 707}, "heading": {"weight": 711},
                       "publisher": {"weight": 707}, "story": {"weight": 707}},
        },
        "story": story,
        "back": [
            {"id": "about", "heading": "About this book", "text": "\n\n".join(desc[:3])},
            {"id": "grown-ups", "heading": "For grown-ups", "text": grown_ups()},
        ],
    }
    (HERE / "spec.json").write_text(json.dumps(spec, indent=2, ensure_ascii=False) + "\n")
    engine = ROOT / "bin/matenadar"
    subprocess.run([str(engine), "template", "picture-book", str(HERE / "spec.json"), "-o", str(HERE / "doc.json")], check=True)


if __name__ == "__main__":
    main()
