#!/usr/bin/env python3
"""Rebuild a sample and set its pages beside the original book's.

    python3 scripts/review.py samples/<slug>            # build + compare sheets
    python3 scripts/review.py samples/<slug> --dpi 120

Reads samples/<slug>/sample.json:
    {"book": "/abs/path/to/book/project",  "pdf": "release/x-interior.pdf",
     "bleed": true, "pages": [{"original": 9, "rebuilt": 1, "what": "chapter opener"}]}
Writes samples/<slug>/build/: rebuilt/, original/, compare-<original>.png, report.json
"""
import json, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# A stable copy of the engine, replaced atomically when the engine changes.
BIN = ROOT / "bin" / "matenadar"


def main():
    sample = Path(sys.argv[1]).resolve()
    dpi = int(sys.argv[sys.argv.index("--dpi") + 1]) if "--dpi" in sys.argv else 100
    spec = json.loads((sample / "sample.json").read_text())
    book = Path(spec["book"])
    build = sample / "build"
    (build / "rebuilt").mkdir(parents=True, exist_ok=True)
    (build / "original").mkdir(parents=True, exist_ok=True)
    link = sample / "book"
    if not link.exists():
        link.symlink_to(book)
    cmd = [str(BIN), "build", str(sample / "doc.json"), "--root", str(sample), "-o", str(build / "rebuilt.pdf"),
           "--png", str(build / "rebuilt"), "--dpi", str(dpi), "--check", "--json"]
    if spec.get("bleed"):
        cmd.append("--bleed")
    t = time.time()
    out = subprocess.run(cmd, capture_output=True, text=True)
    if out.returncode not in (0, 1):
        print(out.stdout, out.stderr)
        sys.exit(2)
    result = json.loads(out.stdout)
    result["wall_ms"] = round((time.time() - t) * 1000)
    sheets = []
    for pg in spec["pages"]:
        o, r = pg["original"], pg["rebuilt"]
        orig = build / "original" / f"p-{o:03d}"
        subprocess.run(["pdftocairo", "-r", str(dpi), "-png", "-singlefile", "-f", str(o), "-l", str(o), str(book / spec["pdf"]), str(orig)], check=True)
        reb = build / "rebuilt" / f"p-{r:03d}.png"
        sheet = build / f"compare-{o:03d}.png"
        subprocess.run(["magick", str(orig) + ".png", "-bordercolor", "#888", "-border", "1", str(reb), "-bordercolor", "#888", "-border", "1",
                        "-background", "#ddd", "-gravity", "north", "+append", "-gravity", "northwest", "-pointsize", "14",
                        "-fill", "#c00", "-annotate", "+8+4", f"original p{o}", "-gravity", "northeast", "-annotate", "+8+4", f"matenadar p{r}",
                        str(sheet)], check=True)
        sheets.append(str(sheet))
    result["sheets"] = sheets
    (build / "report.json").write_text(json.dumps(result, indent=2))
    issues = [i for i in result.get("issues", []) if i["severity"] == "error"]
    print(f"{result['pages']} pages, layout {result['layout_ms']:.0f} ms, {len(issues)} errors")
    for p in result.get("problems", []):
        print("  problem:", p)
    for i in result.get("issues", [])[:20]:
        print(f"  {i['severity']} {i['code']} p{i.get('page')}: {i['message']}")
    for s in sheets:
        print(" ", s)


if __name__ == "__main__":
    main()
