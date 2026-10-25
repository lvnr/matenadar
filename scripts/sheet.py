#!/usr/bin/env python3
"""One page showing every sample: each original page beside its rebuild.

    python3 scripts/sheet.py                     # samples/index.html, linking build/ images
    python3 scripts/sheet.py --artifact DIR      # DIR/proofs.html and DIR/img/*.jpg, to publish

Reads each samples/<slug>/sample.json, build/report.json, the compare sheets
review.py wrote, and the fidelity table in README.md: a table whose header
has a "score" or "fidelity" column, the book's page in its first column
(or one headed "original" or "book"), and optionally a "differs" column.
"""
import html, json, re, statistics, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SAMPLES = ROOT / "samples"


def scores(readme: Path) -> dict:
    """{book page: (score, what still differs)} from the README's fidelity table."""
    out = {}
    if not readme.exists():
        return out
    lines = readme.read_text().splitlines()
    cells = lambda line: [c.strip() for c in line.strip().strip("|").split("|")]
    i = 0
    while i < len(lines) - 1:
        if lines[i].startswith("|") and re.match(r"\|\s*:?-", lines[i + 1]):
            head = [h.lower() for h in cells(lines[i])]
            sc = next((k for k, h in enumerate(head) if "score" in h or "fidelity" in h), None)
            if sc is not None:
                oc = next((k for k, h in enumerate(head) if h in ("original", "book", "book page")), 0)
                nc = next((k for k, h in enumerate(head) if "differ" in h), None)
                i += 2
                while i < len(lines) and lines[i].startswith("|"):
                    row = cells(lines[i])
                    page = re.search(r"\d+", row[oc]) if oc < len(row) else None
                    score = re.search(r"\d+(?:\.\d+)?", row[sc]) if sc < len(row) else None
                    if page and score:
                        note = row[nc] if nc is not None and nc < len(row) else ""
                        out[int(page.group())] = (score.group(), note)
                    i += 1
                continue
        i += 1
    return out


def inline_md(text: str) -> str:
    """Backticks as code, the rest escaped."""
    parts = re.split(r"(`[^`]+`)", text)
    return "".join(f"<code>{html.escape(p[1:-1])}</code>" if p.startswith("`") else html.escape(p) for p in parts)


def collect():
    books = []
    for d in sorted(p for p in SAMPLES.iterdir() if (p / "sample.json").exists()):
        spec = json.loads((d / "sample.json").read_text())
        report = json.loads((d / "build" / "report.json").read_text()) if (d / "build" / "report.json").exists() else {}
        sc = scores(d / "README.md")
        pages = []
        for pg in spec["pages"]:
            o = pg["original"]
            img = d / "build" / f"compare-{o:03d}.png"
            if img.exists():
                score, note = sc.get(o, ("", ""))
                pages.append({"original": o, "rebuilt": pg.get("rebuilt"), "what": pg.get("what", ""), "score": score, "note": note, "img": img})
        books.append({"slug": d.name, "title": spec.get("title") or d.name.replace("-", " ").title(), "report": report, "pages": pages})
    return books


STYLE = """
:root {
  /* A proof sheet: paper and ink, the printer's blue for marks. */
  --paper: #f6f7f8; --sheet: #ffffff; --ink: #16191c; --muted: #5d646c; --rule: #d8dce0;
  --mark: #2648c4; --mark-ink: #ffffff; --soft: #e9edf7;
  --display: "Newsreader", "Iowan Old Style", Georgia, serif;
  --sans: "IBM Plex Sans", "Helvetica Neue", Arial, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) {
  --paper: #121417; --sheet: #1b1e22; --ink: #e8eaed; --muted: #a2a9b1; --rule: #30353b;
  --mark: #9db0ff; --mark-ink: #10131a; --soft: #1f2533; color-scheme: dark } }
:root[data-theme="dark"] {
  --paper: #121417; --sheet: #1b1e22; --ink: #e8eaed; --muted: #a2a9b1; --rule: #30353b;
  --mark: #9db0ff; --mark-ink: #10131a; --soft: #1f2533; color-scheme: dark }
body { background: var(--paper); color: var(--ink); font: 15px/1.55 var(--sans); }
main { max-width: 1240px; margin: 0 auto; padding-block: 28px 72px;
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); }
h1 { font: 500 clamp(30px, 5vw, 44px)/1.05 var(--display); margin: 0 0 10px; text-wrap: balance; letter-spacing: -0.01em; }
h2 { font: 500 26px/1.15 var(--display); margin: 0; text-wrap: balance; }
.lede { max-width: 68ch; color: var(--muted); margin: 0 0 22px; }
.label { font: 600 11px/1.3 var(--sans); letter-spacing: 0.09em; text-transform: uppercase; color: var(--muted); }
.wrap { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; min-width: 560px; font-variant-numeric: tabular-nums; }
th, td { text-align: left; padding: 7px 10px 7px 0; border-bottom: 1px solid var(--rule); vertical-align: baseline; }
th { font: 600 11px/1.3 var(--sans); letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
td.num, th.num { text-align: right; font-family: var(--mono); font-size: 13px; }
a { color: var(--mark); text-underline-offset: 2px; }
a:focus-visible { outline: 2px solid var(--mark); outline-offset: 2px; }
section { margin-top: 48px; border-top: 2px solid var(--ink); padding-top: 14px; display: grid; gap: 18px; }
.meta { color: var(--muted); font: 13px/1.4 var(--mono); }
figure { margin: 0; display: grid; gap: 8px; }
figcaption { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 4px 12px; align-items: baseline; }
.score { font: 600 13px/1 var(--mono); background: var(--mark); color: var(--mark-ink); border-radius: 3px; padding: 5px 7px; min-width: 3.4em; text-align: center; }
.score.low { background: var(--soft); color: var(--ink); }
.what { font-weight: 600; min-width: 0; }
.where { font: 12px/1.3 var(--mono); color: var(--muted); }
.note { grid-column: 2; color: var(--muted); min-width: 0; }
code { font: 0.9em var(--mono); }
figure img { display: block; width: 100%; height: auto; border: 1px solid var(--rule); background: #fff; }
.how { display: flex; flex-wrap: wrap; gap: 6px 18px; color: var(--muted); font-size: 13px; margin: 0 0 6px; }
"""


def page_html(books, src_of, total_pages):
    rows = []
    for b in books:
        nums = [float(p["score"]) for p in b["pages"] if p["score"]]
        rng = f"{min(nums):g}–{max(nums):g}" if nums else "–"
        med = f"{statistics.median(nums):g}" if nums else "–"
        r = b["report"]
        rows.append(
            f"<tr><td><a href=\"#{b['slug']}\">{html.escape(b['title'])}</a></td>"
            f"<td class=num>{len(b['pages'])}</td><td class=num>{med}</td><td class=num>{rng}</td>"
            f"<td class=num>{r.get('pages', '–')}</td><td class=num>{r.get('layout_ms', 0):.0f}</td>"
            f"<td class=num>{sum(1 for i in r.get('issues', []) if i.get('severity') == 'error')}</td></tr>"
        )
    sections = []
    for b in books:
        figs = []
        for p in b["pages"]:
            score = p["score"]
            chip = f"<span class=\"score{' low' if score and float(score) < 9 else ''}\">{html.escape(score)}</span>" if score else "<span></span>"
            where = f"book p. {p['original']} · rebuilt p. {p['rebuilt']}" if p["rebuilt"] else f"book p. {p['original']}"
            note = f"<div class=note>{inline_md(p['note'])}</div>" if p["note"] else ""
            figs.append(
                f"<figure><figcaption>{chip}<div class=what>{inline_md(p['what'])} <span class=where>{where}</span></div>{note}</figcaption>"
                f"<img loading=lazy decoding=async src=\"{src_of(b, p)}\" alt=\"{html.escape(b['title'])}, book page {p['original']} beside Matenadar's rebuild\"></figure>"
            )
        r = b["report"]
        sections.append(
            f"<section id=\"{b['slug']}\"><div><div class=label>{len(b['pages'])} pages</div><h2>{html.escape(b['title'])}</h2>"
            f"<div class=meta>{r.get('pages', '?')} pages laid out in {r.get('layout_ms', 0):.0f} ms · check: "
            f"{sum(1 for i in r.get('issues', []) if i.get('severity') == 'error')} errors</div></div>{''.join(figs)}</section>"
        )
    return f"""<div class=label>Matenadar · sample review</div>
<h1>Matenadar proofs</h1>
<p class=lede>{total_pages} pages from eight published books, rebuilt in Matenadar from each book's own text, data and art.
Every sheet sets the book's PDF page (left) beside Matenadar's page (right), both rendered at 150 dpi.
Scores are the rebuilding agents' own, out of 10.</p>
<p class=how><span>Left: the published interior</span><span>Right: Matenadar</span><span>Blue mark: 9 or better</span></p>
<div class=wrap><table><thead><tr><th>Book</th><th class=num>Sheets</th><th class=num>Median</th><th class=num>Range</th><th class=num>Pages</th><th class=num>Layout ms</th><th class=num>Errors</th></tr></thead>
<tbody>{''.join(rows)}</tbody></table></div>
{''.join(sections)}"""


FONTS = '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600&family=IBM+Plex+Sans:wght@400;600&family=Newsreader:opsz,wght@6..72,500&display=swap">'


def main():
    books = collect()
    total = sum(len(b["pages"]) for b in books)
    if "--artifact" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--artifact") + 1])
        (out / "img").mkdir(parents=True, exist_ok=True)

        def src(b, p):
            name = f"img/{b['slug']}-{p['original']:03d}.jpg"
            subprocess.run(["magick", str(p["img"]), "-resize", "1600x>", "-quality", "74", "-strip", str(out / name)], check=True)
            return name

        body = page_html(books, src, total)
        page = f"""<title>Matenadar proofs</title>
<meta name="claude:viewport-fit-sides" content="cover">
{FONTS}
<style>{STYLE}</style>
<main>{body}</main>
"""
        (out / "proofs.html").write_text(page)
        size = sum(f.stat().st_size for f in out.rglob("*") if f.is_file())
        print(out / "proofs.html", f"{size / 1e6:.1f} MB with images", total, "pages")
        return
    body = page_html(books, lambda b, p: p["img"].relative_to(SAMPLES).as_posix(), total)
    out = SAMPLES / "index.html"
    out.write_text(f"""<!doctype html><html lang=en><head><meta charset=utf-8>
<meta name=viewport content="width=device-width, initial-scale=1">
<title>Matenadar proofs</title>{FONTS}<style>{STYLE}</style></head><body><main>{body}</main></body></html>
""")
    print(out, total, "pages")


if __name__ == "__main__":
    main()
