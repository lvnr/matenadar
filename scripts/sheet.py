#!/usr/bin/env python3
"""One page showing every sample: each original page beside its rebuild.

    python3 scripts/sheet.py                 # samples/index.html, linking build/ images
    python3 scripts/sheet.py --embed out.html  # one self-contained file (JPEG data URIs)

Reads each samples/<slug>/sample.json, build/report.json, the compare sheets
review.py wrote, and the scores table in README.md (rows "| p<N> | score |").
"""
import base64, html, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SAMPLES = ROOT / "samples"


def scores(readme: Path):
    out = {}
    if not readme.exists():
        return out
    for line in readme.read_text().splitlines():
        m = re.match(r"\|\s*p\.?\s*(\d+)\b[^|]*\|\s*(\d+(?:\.\d+)?)\s*(?:/\s*10)?\s*\|(.*)", line, re.I)
        if m:
            out[int(m.group(1))] = (m.group(2), m.group(3).strip(" |"))
    return out


def jpeg_uri(png: Path, width=1100):
    data = subprocess.run(["magick", str(png), "-resize", f"{width}x>", "-quality", "70", "jpg:-"], capture_output=True, check=True).stdout
    return "data:image/jpeg;base64," + base64.b64encode(data).decode()


def main():
    embed = "--embed" in sys.argv
    out = Path(sys.argv[sys.argv.index("--embed") + 1]) if embed else SAMPLES / "index.html"
    parts = []
    total_pages = 0
    for d in sorted(p for p in SAMPLES.iterdir() if (p / "sample.json").exists()):
        spec = json.loads((d / "sample.json").read_text())
        report = json.loads((d / "build" / "report.json").read_text()) if (d / "build" / "report.json").exists() else {}
        sc = scores(d / "README.md")
        title = spec.get("title") or d.name.replace("-", " ").title()
        meta = f"{report.get('pages', '?')} pages · layout {report.get('layout_ms', 0):.0f} ms"
        parts.append(f"<section><h2>{html.escape(title)}</h2><p class=meta>{html.escape(meta)}</p>")
        for pg in spec["pages"]:
            o = pg["original"]
            img = d / "build" / f"compare-{o:03d}.png"
            if not img.exists():
                continue
            total_pages += 1
            src = jpeg_uri(img) if embed else img.relative_to(SAMPLES).as_posix()
            score, note = sc.get(o, ("", ""))
            cap = f"p.{o} — {pg.get('what', '')}"
            badge = f"<span class=score>{html.escape(score)}/10</span>" if score else ""
            parts.append(
                f"<figure><figcaption>{badge}<b>{html.escape(cap)}</b> {html.escape(note)}</figcaption>"
                f"<img loading=lazy src=\"{src}\" alt=\"{html.escape(cap)}\"></figure>"
            )
        parts.append("</section>")
    page = f"""<!doctype html><html lang=en><head><meta charset=utf-8>
<meta name=viewport content="width=device-width, initial-scale=1">
<title>Matenadar samples</title>
<style>
:root {{ --bg: #f6f3ee; --fg: #1f1c19; --muted: #6b645c; --line: #ddd5ca; --accent: #8a2b0f; }}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --bg: #171513; --fg: #ece6dd; --muted: #a39a8f; --line: #3a342e; --accent: #e08a62; }} }}
:root[data-theme="dark"] {{ --bg: #171513; --fg: #ece6dd; --muted: #a39a8f; --line: #3a342e; --accent: #e08a62; }}
body {{ margin: 0; background: var(--bg); color: var(--fg); font: 15px/1.5 ui-serif, Georgia, serif; }}
main {{ max-width: 1160px; margin: 0 auto; padding: 24px 16px 64px; }}
h1 {{ font-size: 28px; margin: 0 0 4px; }} h2 {{ font-size: 21px; margin: 40px 0 0; border-top: 1px solid var(--line); padding-top: 18px; }}
.meta, .lede {{ color: var(--muted); margin: 2px 0 12px; }}
figure {{ margin: 18px 0; }} figcaption {{ margin-bottom: 6px; color: var(--muted); }} figcaption b {{ color: var(--fg); font-weight: 600; }}
.score {{ display: inline-block; min-width: 3.2em; text-align: center; background: var(--accent); color: var(--bg); border-radius: 3px; padding: 0 6px; margin-right: 8px; font: 600 13px/1.6 ui-sans-serif, system-ui; }}
img {{ width: 100%; height: auto; border: 1px solid var(--line); background: #fff; }}
</style></head><body><main>
<h1>Matenadar samples</h1>
<p class=lede>{total_pages} pages from eight published books, rebuilt from their own text, data and art. Each sheet shows the original (left) beside Matenadar's page (right).</p>
{''.join(parts)}
</main></body></html>"""
    out.write_text(page)
    print(out, f"{out.stat().st_size / 1e6:.1f} MB", total_pages, "pages")


if __name__ == "__main__":
    main()
