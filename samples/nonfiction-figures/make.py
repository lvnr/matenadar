#!/usr/bin/env python3
"""Two chapters of The Attention Repair Manual in black ink, with numbered figures, a list of figures, a
cross-reference and a full-bleed plate.

    python3 samples/nonfiction-figures/make.py      # writes doc.json beside this file

Reads the book's manuscript (manuscript/00-frontmatter.md, 01-opening.md, 02-week-one.md) and its accepted
drawings (visuals/accepted). The drawings have their titles lettered in ("Fig. 1. The page you were
protecting"); each figure is cropped below that band, measured from the picture, and the engine sets the
number and title as a caption instead, so numbers stay right when figures move. The chapter plate (Week 1)
goes to the bleed on a page of its own. The book has no cross-references; the one added here, "(Fig. 2,
page N)", shows how they read. Libertinus Serif (the engine's own) sets the text and Source Sans Pro (SIL OFL,
copied to build/fonts when the system has it) the headings and captions; the book itself used Warnock Pro
and Halyard.
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
IN = 72.0


# Where each drawing's lettered title band ends, as a share of its height (measured by eye against a ruler;
# the drawings begin just under these lines).
TITLE_BANDS = {"rf-fig01-sofa-r1.png": 0.072, "rf-fig02-reach-r1.png": 0.085, "rf-fig05-ladder-r1.png": 0.066, "rf-fig06-handle-r1.png": 0.11}


def inline(text: str, notes: dict[str, str]) -> list:
    """Markdown emphasis and footnote references as runs."""
    out: list = []
    for part in re.split(r"(\*\*[^*]+\*\*|\*[^*]+\*|\[\^[^\]]+\])", text):
        if not part:
            continue
        if part.startswith("**"):
            out.append({"text": part[2:-2], "strong": True})
        elif part.startswith("[^"):
            key = part[2:-1]
            if key in notes:
                out.append({"note": inline(notes[key], {})})
        elif part.startswith("*"):
            out.append({"text": part[1:-1], "em": True})
        else:
            out.append(part.replace("\\_", "_"))
    return out


def figure(line: str, n: list[int], plate: bool) -> dict:
    m = re.match(r"!\[\]\(([^)]+)\)\{([^}]*)\}", line)
    src, attrs = m.group(1), m.group(2)
    alt = re.search(r'alt="([^"]*)"', attrs).group(1)
    path = BOOK / src
    fig = {"type": "figure", "src": "book/" + src, "alt": alt}
    if plate:
        n[1] += 1
        fig.update({"id": f"plate-{n[1]}", "plate": "bleed"})
        return fig
    n[0] += 1
    title = re.search(r"titled Fig\. \d+, ([^.]+)\.", alt)
    width = re.search(r"width=(\d+)%", attrs)
    fig.update({"id": f"fig-{n[0]}", "w": f"{int(width.group(1)) * 0.9 if width else 88}%", "align": "center", "caption": (title.group(1) + ".") if title else ""})
    band = TITLE_BANDS.get(path.name)
    if band:
        fig["crop"] = {"y": band, "h": round(1 - band, 4)}
    return fig


def chapter(name: str, number: str, n: list[int]) -> dict:
    text = (BOOK / "manuscript" / name).read_text()
    notes = dict(re.findall(r"^\[\^([^\]]+)\]: (.+)$", text, flags=re.M))
    body = re.sub(r"^\[\^[^\]]+\]: .+$", "", text, flags=re.M)
    flow: list = []
    title = ""
    quoted = False
    for chunk in [c.strip() for c in body.split("\n\n") if c.strip()]:
        if chunk.startswith("> "):
            # A card: quoted lines, one box until something else comes.
            if not quoted:
                flow.append({"type": "box", "style": "card", "fill": "#efefef", "inset": "10pt", "space_before": "8pt", "space_after": "8pt", "blocks": []})
            flow[-1]["blocks"] += [{"type": "p", "style": "card", "content": inline(l[2:], notes)} for l in chunk.splitlines() if l[2:].strip()]
            quoted = True
            continue
        quoted = False
        if chunk.startswith("# ") and title:
            flow.append({"type": "h", "level": 2, "text": chunk[2:], "outline": False})
        elif chunk.startswith("# "):
            title = chunk[2:].split(": ", 1)[-1]
            flow.append({"type": "p", "style": "kicker", "text": f"Chapter {number}"})
            flow.append({"type": "h", "level": 1, "id": f"ch{number}", "text": title})
        elif chunk.startswith("## "):
            flow.append({"type": "h", "level": 2, "text": chunk[3:], "outline": False})
        elif chunk.startswith("!["):
            flow.append(figure(chunk, n, plate="height=72%" in chunk))
        elif re.match(r"^(- |\d+\. )", chunk):
            items = [inline(re.sub(r"^(- |\d+\. )", "", l), notes) for l in chunk.splitlines()]
            flow.append({"type": "list", "ordered": chunk[0].isdigit(), "items": [[{"type": "p", "content": i}] for i in items]})
        else:
            flow.append({"type": "p", "content": inline(" ".join(chunk.splitlines()), notes)})
    return {"type": "section", "id": f"chapter-{number}", "master": "body", "start": "recto", "title": title,
            "vars": {"chapter": number}, "flow": flow}


def cross_reference(sec: dict) -> None:
    """The one reference added to the book: where the six suspects are introduced, a pointer to Fig. 2."""
    for b in sec["flow"]:
        if b.get("type") == "p" and b.get("content") and isinstance(b["content"][0], str) and b["content"][0].startswith("The sorting tool has six suspects"):
            words = b["content"][0].rstrip(".")
            b["content"] = [words + " (", {"ref": "fig-2", "show": "label"}, ", page ", {"ref": "fig-2"}, ")."] + b["content"][1:]
            return
    raise SystemExit("the paragraph introducing the six suspects is gone")


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    fonts = HERE / "build/fonts"
    fonts.mkdir(parents=True, exist_ok=True)
    for face in ["Regular", "It", "Semibold", "Bold"]:
        src = Path.home() / f"Library/Fonts/SourceSansPro-{face}.otf"
        if src.exists():
            shutil.copyfile(src, fonts / src.name)
    n = [0, 0]
    ch1 = chapter("01-opening.md", "1", n)
    ch2 = chapter("02-week-one.md", "2", n)
    cross_reference(ch1)
    front = (BOOK / "manuscript/00-frontmatter.md").read_text()
    title = "The Attention Repair Manual"
    m = re.search(r"^title:\s*\"?([^\"\n]+)", front, flags=re.M)
    if m:
        title = m.group(1).strip()
    sans = ["Source Sans Pro", "Libertinus Serif"]
    doc = {
        "matenadar": "1",
        "meta": {"title": title, "author": "Levon Arakelyan", "lang": "en-US"},
        "fonts": ["build/fonts"],
        "print": {"ink": "black", "filter": {"gray": True, "contrast": 2}},
        "design": {
            "colors": {"ink": "#1a1a1a", "rule": "#9a9a9a"},
            "fonts": {"serif": ["Libertinus Serif"], "sans": sans},
            "base": {"font": "serif", "size": 10.5, "line_height": "14.5pt", "color": "ink", "lang": "en-US"},
            "styles": {
                "body": {"align": "justify", "indent": "1.2em", "hyphenate": True},
                "kicker": {"font": "sans", "size": 9.5, "weight": "semibold", "case": "upper", "tracking": "0.14em", "space_before": "1.1in", "space_after": "6pt"},
                "h1": {"font": "sans", "size": 22, "weight": "semibold", "line_height": 1.15, "space_after": "28pt", "keep_with_next": True},
                "h2": {"font": "sans", "size": 11.5, "weight": "semibold", "line_height": 1.3, "space_before": "16pt", "space_after": "5pt", "keep_with_next": True},
                "card": {"font": "serif", "size": 10, "line_height": "13.5pt", "space_after": "4pt"},
                "caption": {"font": "sans", "size": 8.5, "line_height": "11pt", "align": "center", "space_before": "5pt"},
                "toc-1": {"size": 10.5, "line_height": "17pt"},
                "lof": {"size": 10, "line_height": "16pt"},
                "folio": {"font": "sans", "size": 8.5, "align": "center"},
                "contents-title": {"font": "sans", "size": 16, "weight": "semibold", "space_before": "0.9in", "space_after": "18pt"},
                "title": {"font": "sans", "size": 28, "weight": "semibold", "line_height": 1.15, "align": "center"},
                "byline": {"font": "serif", "size": 13, "italic": True, "align": "center"}
            },
            "spans": {"figlabel": {"font": "sans", "weight": "semibold"}},
            "figures": {"label": "Fig.", "per": "book", "span": "figlabel"},
            "masters": {"body": {
                "width": "6in", "height": "9in", "bleed": "0.125in",
                "margins": {"top": "0.75in", "bottom": "0.85in", "inside": "0.85in", "outside": "0.65in"},
                "style": "body",
                "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": "0.85in", "y": "8.45in", "w": "4.5in",
                                "content": [{"field": "page"}], "when": "not-first not-blank"}]
            }}
        },
        "content": [
            {"type": "page", "id": "title", "master": "body", "elements": [
                {"type": "text", "id": "title-words", "x": "0.85in", "y": "2.6in", "w": "4.5in", "style": "title", "text": title},
                {"type": "text", "id": "byline", "x": "0.85in", "y": "4.1in", "w": "4.5in", "style": "byline", "text": "Levon Arakelyan"}
            ]},
            {"type": "section", "id": "contents", "master": "body", "start": "recto", "numbering": "i", "page_from": 1, "flow": [
                {"type": "h", "level": 1, "style": "contents-title", "text": "Contents", "outline": False},
                {"type": "toc", "leader": "."},
                {"type": "h", "level": 1, "style": "contents-title", "text": "Figures", "outline": False},
                {"type": "lof", "leader": "."}
            ]},
            {**ch1, "numbering": "1", "page_from": 1},
            ch2,
        ],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
