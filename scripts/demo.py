#!/usr/bin/env python3
"""Lists the samples for the page editor demo (demo/editor/samples.json):
each sample's document and its font files, as paths the demo can fetch
from a server at the repository's root.

    scripts/wasm.sh && python3 scripts/demo.py && python3 -m http.server 5199
    open http://localhost:5199/demo/editor/
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONT_TYPES = {".ttf", ".otf", ".ttc", ".otc"}


def main() -> None:
    samples = []
    for doc_path in sorted(ROOT.glob("samples/*/doc.json")):
        sample = doc_path.parent
        doc = json.loads(doc_path.read_text())
        fonts = []
        for folder in doc.get("fonts", []):
            base = sample / folder
            if base.is_dir():
                fonts += sorted(str(p.relative_to(ROOT)) for p in base.rglob("*") if p.suffix.lower() in FONT_TYPES)
        meta = json.loads((sample / "sample.json").read_text()) if (sample / "sample.json").exists() else {}
        samples.append({
            "slug": sample.name,
            "title": meta.get("title") or doc.get("meta", {}).get("title") or sample.name,
            "root": str(sample.relative_to(ROOT)) + "/",
            "doc": str(doc_path.relative_to(ROOT)),
            "fonts": fonts,
        })
    out = ROOT / "demo/editor/samples.json"
    out.write_text(json.dumps(samples, indent=2) + "\n")
    print(f"{out.relative_to(ROOT)}: {len(samples)} samples")


if __name__ == "__main__":
    main()
