#!/usr/bin/env python3
"""Render the landing page's examples with the real engine.

Builds every document under `examples/` (and the rebuilt book pages under
`../samples/`) with the `matenadar` binary, then writes what the site shows:

    public/examples/<id>/p-N.webp       pages at display size (and -s.webp, smaller)
    public/examples/<id>/<id>.pdf       the PDF, for download
    public/examples/<id>/<id>.json      the document itself
    src/data/examples.json              sizes, timings and every node's box
    public/og.jpg                       the social card, examples/og/doc.json

The site builds from those files, so it never needs the engine. Run it from
`website/` after `cargo build --release`, when an example changes:

    python3 scripts/render-examples.py [--bin ../target/release/matenadar] [only-this-id …]

Needs ImageMagick (`magick`) and `cwebp`. The example fonts are fetched by
`examples/fonts/fetch.sh`.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SITE = Path(__file__).resolve().parent.parent
REPO = SITE.parent
OUT = SITE / "public" / "examples"
DATA = SITE / "src" / "data" / "examples.json"

# id: document, the long edge in pixels for the large image, and the pages
# the site shows (1-based; None for all).
EXAMPLES = {
    "poster": ("examples/poster/doc.json", 1600, None),
    "invoice": ("examples/invoice/doc.json", 1600, None),
    "deck": ("examples/deck/doc.json", 1600, None),
    "cover": ("examples/cover/doc.json", 2200, None),
    "magazine": ("examples/magazine/doc.json", 1400, [2, 3]),
    "menu": ("examples/menu/doc.json", 1600, None),
    "calendar": ("examples/calendar/doc.json", 1600, None),
    "field-guide": ("../samples/a-field-guide-to-software-bugs/doc.json", 1400, [6, 7]),
    "draft": ("examples/draft/doc.json", 1200, None),
}
# Documents whose preflight report the site shows (`check --preset kdp`).
CHECK = {"draft"}
# Pages of a published book: shown, not offered for download.
NO_DOWNLOAD = {"field-guide"}
# The books' samples are set in the faces the books use, installed on the
# machine (Adobe Caslon Pro and others); the examples bring their own.
SYSTEM_FONTS = {"field-guide"}

# Rebuilt pages of the published books: sample folder, title, pages to show.
BOOKS = [
    ("a-field-guide-to-software-bugs", "A Field Guide to Software Bugs", [1, 3, 5, 6, 7]),
    ("the-twenty-fourth-clue", "The Twenty-Fourth Clue", None),
    ("idioms-of-the-world", "Idioms of the World, Drawn Literally", None),
    ("the-machine-in-the-back-of-the-book", "The Machine in the Back of the Book", None),
    ("i-just-had-it", "I Just Had It", None),
    ("reverse-coloring", "Reverse Coloring", None),
    ("third-shift-handbook", "Third Shift Handbook", None),
    ("christmas-word-search", "Christmas Word Search", None),
]


def run(cmd, **kw):
    r = subprocess.run(cmd, capture_output=True, text=True, **kw)
    if r.returncode != 0:
        sys.exit(f"failed: {' '.join(map(str, cmd))}\n{r.stdout}\n{r.stderr}")
    return r.stdout


def blank(png: Path) -> bool:
    """A page with (almost) nothing on it."""
    sd = float(run(["magick", str(png), "-colorspace", "gray", "-format", "%[fx:standard_deviation]", "info:"]))
    return sd < 0.01


def webp(src: Path, dst: Path, width: int, quality: int = 84):
    run(["cwebp", "-quiet", "-q", str(quality), "-m", "6", "-resize", str(width), "0", str(src), "-o", str(dst)])


def size_px(png: Path):
    w, h = run(["magick", "identify", "-format", "%w %h", str(png)]).split()
    return int(w), int(h)


def build(binary, doc: Path, tmp: Path, dpi: float, pdf: Path | None = None, flags=("--no-system-fonts",)):
    pdf = pdf or tmp / "out.pdf"
    out = run([binary, "build", str(doc), "-o", str(pdf), "--png", str(tmp / "png"), "--dpi", f"{dpi:.2f}", *flags, "--json"])
    return json.loads(out)


def element_ids(doc) -> set[str]:
    """The ids of the placed elements, the boxes worth pointing at."""
    ids = set()

    def walk(els):
        for e in els or []:
            if isinstance(e, dict) and e.get("id"):
                ids.add(e["id"])

    for item in doc.get("content", []):
        walk(item.get("elements"))
        for side in ("left", "right"):
            if isinstance(item.get(side), dict):
                walk(item[side].get("elements"))
    return ids


def render_example(binary, key, rel, long_px, show):
    doc_path = (SITE / rel).resolve()
    doc = json.loads(doc_path.read_text())
    dest = OUT / key
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    flags = () if key in SYSTEM_FONTS else ("--no-system-fonts",)
    layout = json.loads(run([binary, "layout", str(doc_path), *flags, "--json"]))
    pages = [p for p in layout if show is None or p["page"] in show]
    longest = max(max(p["width"], p["height"]) for p in pages)
    dpi = long_px / (longest / 72)
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        report = build(binary, doc_path, tmp, dpi, dest / f"{key}.pdf", flags)
        ids = element_ids(doc)
        out_pages = []
        for p in pages:
            png = tmp / "png" / f"p-{p['page']:03d}.png"
            n = len(out_pages) + 1
            big, small = dest / f"p-{n}.webp", dest / f"p-{n}-s.webp"
            pw, ph = size_px(png)
            webp(png, big, pw)
            webp(png, small, max(360, round(pw * 0.42)))
            nodes = [
                {"id": nd["id"], "kind": nd["kind"], "r": [round(nd["rect"][k], 2) for k in ("x0", "y0", "x1", "y1")]}
                for nd in p["nodes"]
                if nd["id"] in ids
            ]
            out_pages.append({
                "src": f"/examples/{key}/p-{n}.webp",
                "small": f"/examples/{key}/p-{n}-s.webp",
                "w": round(p["width"], 2), "h": round(p["height"], 2),
                "px": [pw, ph],
                "label": p["label"],
                "nodes": nodes,
            })
    downloads = key not in NO_DOWNLOAD
    if downloads:
        shutil.copy(doc_path, dest / f"{key}.json")
    else:
        (dest / f"{key}.pdf").unlink()
    fonts = sorted({f["family"] for f in json.loads(run([binary, "fonts", "--used", str(doc_path), *flags, "--json"]))})
    # A hash per page; the WebAssembly build gives the same ones for the same
    # document and fonts.
    fingerprints = json.loads(run([binary, "fingerprint", str(doc_path), *flags]))["fingerprints"]
    checks = None
    if key in CHECK:
        r = subprocess.run([binary, "check", str(doc_path), "--preset", "kdp", *flags, "--json"], capture_output=True, text=True)
        checks = json.loads(r.stdout)
    return {
        "id": key,
        "checks": checks,
        "title": doc.get("meta", {}).get("title", key),
        "pages": out_pages,
        "page_count": report["pages"],
        "layout_ms": round(report["layout_ms"], 1),
        "issues": len(report.get("issues", [])) + len(report.get("problems", [])),
        "fingerprints": fingerprints,
        "pdf": {"href": f"/examples/{key}/{key}.pdf", "bytes": (dest / f"{key}.pdf").stat().st_size} if downloads else None,
        "json": {"href": f"/examples/{key}/{key}.json" if downloads else None, "bytes": doc_path.stat().st_size, "path": str(doc_path.relative_to(REPO))},
        "fonts": sorted({re.sub(r" \d+pt$", "", f) for f in fonts}),
    }


def render_books(binary):
    dest = OUT / "books"
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    books = []
    for slug, title, want in BOOKS:
        doc = REPO / "samples" / slug / "doc.json"
        with tempfile.TemporaryDirectory() as t:
            tmp = Path(t)
            report = build(binary, doc, tmp, 110, flags=())
            pngs = sorted((tmp / "png").glob("p-*.png"))
            pages = []
            for i, png in enumerate(pngs, 1):
                if (want and i not in want) or blank(png):
                    continue
                name = f"{slug}-{i}.webp"
                w, h = size_px(png)
                webp(png, dest / name, 560, 82)
                pages.append({"src": f"/examples/books/{name}", "px": [560, round(h * 560 / w)]})
        books.append({"slug": slug, "title": title, "pages": pages, "layout_ms": round(report["layout_ms"], 1)})
    return books


def render_og(binary):
    """The social card: examples/og/doc.json, with three examples as its art.

    The art is cut from the renders above (the engine reads JPEG and PNG,
    not WebP), built at twice the size and brought down to 1200 × 630."""
    doc = SITE / "examples" / "og" / "doc.json"
    art = doc.parent / "art"
    art.mkdir(exist_ok=True)
    for name in ("poster", "invoice", "deck"):
        run(["magick", str(OUT / name / "p-1.webp"), "-resize", "900x900>", "-quality", "90", str(art / f"{name}.jpg")])
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        build(binary, doc, tmp, 144)
        png = next((tmp / "png").glob("*.png"))
        run(["magick", str(png), "-filter", "Lanczos", "-resize", "1200x630!", "-strip", "-quality", "90",
             "-sampling-factor", "4:4:4", str(SITE / "public" / "og.jpg")])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default=str(REPO / "target" / "release" / "matenadar"))
    ap.add_argument("only", nargs="*")
    a = ap.parse_args()
    data = json.loads(DATA.read_text()) if DATA.exists() else {"examples": {}, "books": []}
    for key, (rel, long_px, show) in EXAMPLES.items():
        if a.only and key not in a.only:
            continue
        print(f"{key} …", flush=True)
        data["examples"][key] = render_example(a.bin, key, rel, long_px, show)
    if not a.only or "books" in a.only:
        print("books …", flush=True)
        data["books"] = render_books(a.bin)
    if not a.only or "og" in a.only:
        print("og …", flush=True)
        render_og(a.bin)
    data["version"] = run([a.bin, "--version"]).split()[-1]
    DATA.parent.mkdir(parents=True, exist_ok=True)
    DATA.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    print(f"wrote {DATA.relative_to(SITE)}")


if __name__ == "__main__":
    main()
