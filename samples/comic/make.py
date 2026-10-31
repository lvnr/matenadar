#!/usr/bin/env python3
"""Friday morning from I Just Had It, as a comic page.

    python3 samples/comic/make.py      # writes doc.json beside this file

Five panels in three rows, each the book's own scene SVG (build/scenes/<id>.svg) cropped round the thing that was
lost (its position is in build/scenes/<id>.json). The words are the book's (manuscript/scenes.json): the time and
place in caption boxes, a line of each notice in a balloon (speech, thought, whisper or shout). The speaker is never
drawn, so tails point off the panel's foot, as comics do for a voice out of frame. Type: the book's own fonts
(design/fonts): SIText for the balloons, Courier Prime for the captions.
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
SCENES = {s["id"]: s for s in json.loads((BOOK / "manuscript/scenes.json").read_text())["scenes"]}
W, H, BLEED = 504.0, 720.0, 9.0
MARGIN = 36.0


def target(scene: str) -> tuple[float, float]:
    """Where the lost thing is, as fractions of the scene."""
    info = json.loads((BOOK / f"build/scenes/{scene}.json").read_text())
    mw, mh = info["media_in"]
    t = next(f for f in info["find"] if f["role"] == "target")
    return t["x"] / mw, t["y"] / mh


def crop(scene: str, w: float, aspect: float) -> dict:
    """A window `w` of the scene's width with the panel's aspect (w / h), round the lost thing."""
    info = json.loads((BOOK / f"build/scenes/{scene}.json").read_text())
    mw, mh = info["media_in"]
    h = w * mw / mh / aspect
    tx, ty = target(scene)
    x = min(max(tx - w / 2, 0.0), 1.0 - w)
    y = min(max(ty - h / 2, 0.0), 1.0 - h)
    return {"x": round(x, 4), "y": round(y, 4), "w": round(w, 4), "h": round(min(h, 1.0), 4)}


def line(scene: str, label: str) -> str:
    return next(b for a, b in SCENES[scene]["lines"] if a == label)


def caption(id_: str, words: str) -> dict:
    return {"type": "balloon", "id": id_, "shape": "caption", "x": 8, "y": 8, "w": 150, "text": words, "style": "caption",
            "fill": "paper", "stroke": {"width": 0.8, "color": "ink"}, "inset": "4pt"}


def balloon(id_: str, shape: str, x: float, y: float, words: str, tail: tuple[float, float], w: float = 130) -> dict:
    """A balloon centred at (x, y) whose tail runs `tail` (dx, dy) toward the speaker below the frame."""
    return {"type": "balloon", "id": id_, "shape": shape, "x": x, "y": y, "w": w, "text": words, "tail": [x + tail[0], y + tail[1]],
            "anchor": "center", "stroke": {"width": 1, "color": "ink"}}


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    pw, ph = W - 2 * MARGIN, H - 2 * MARGIN
    gutter = 10.0
    row_h = (ph - 2 * gutter) / 3

    def panel(id_: str, scene: str, w_share: float, width: float, *elements: dict) -> dict:
        return {"id": id_, "w": f"{w_share}fr", "src": f"book/build/scenes/{scene}.svg", "crop": crop(scene, 0.42 if width < 300 else 0.62, width / row_h),
                "alt": f"{SCENES[scene]['where']}: somewhere in it, {SCENES[scene]['head'].lower().rstrip('?')}.", "elements": list(elements)}

    w1 = (pw - gutter) * 1.25 / 2.25
    w2 = pw - gutter - w1
    half = (pw - gutter) / 2
    s = SCENES
    rows = [
        {"h": "1fr", "panels": [
            panel("phone", "phone", 1.25, w1, caption("phone-when", f"{s['phone']['when']} {s['phone']['where']}."),
                  balloon("phone-says", "speech", w1 * 0.6, 92, line("phone", "Last seen"), (26, 62))),
            panel("glasses", "glasses", 1, w2, caption("glasses-when", s["glasses"]["when"].split(", ")[1]),
                  balloon("glasses-thinks", "thought", w2 * 0.55, 88, line("glasses", "Already checked"), (-30, 70), 100)),
        ]},
        {"h": "1fr", "panels": [
            panel("keys", "keys", 1, pw, caption("keys-when", f"{s['keys']['when'].split(', ')[1]} {s['keys']['where']}."),
                  balloon("keys-says", "speech", pw * 0.7, 66, line("keys", "Last seen"), (34, 64), 150),
                  balloon("keys-shouts", "shout", pw * 0.3, row_h - 64, s["keys"]["head"].upper() + "!", (-30, 50), 110)),
        ]},
        {"h": "1fr", "panels": [
            panel("shoe", "shoe", 1, half, caption("shoe-when", s["shoe"]["when"].split(", ")[1]),
                  balloon("shoe-says", "speech", half * 0.55, 84, line("shoe", "Have"), (-24, 70), 120)),
            panel("wallet", "wallet", 1, half, caption("wallet-when", f"{s['wallet']['when'].split(', ')[1]} At the counter."),
                  balloon("wallet-whispers", "whisper", half * 0.5, 84, line("wallet", "Witnesses"), (30, 70), 120)),
        ]},
    ]
    doc = {
        "matenadar": "1",
        "meta": {"title": "I Just Had It: Friday morning", "lang": "en-US"},
        "fonts": ["book/design/fonts"],
        "design": {
            "colors": {"ink": "#151a2b", "accent": "#e5553f", "paper": "#faf5e8"},
            "base": {"font": "SIText", "size": 10, "line_height": 1.25, "color": "ink"},
            "styles": {
                "balloon": {"font": "SIText", "size": 10.5, "line_height": 1.22, "align": "center"},
                "caption": {"font": "Courier Prime", "weight": 700, "size": 7.6, "line_height": 1.3, "tracking": "0.06em", "case": "upper"},
            },
            "masters": {"sheet": {"width": f"{W}pt", "height": f"{H}pt", "bleed": f"{BLEED}pt",
                                  "margins": {"top": f"{MARGIN}pt", "bottom": f"{MARGIN}pt", "inside": f"{MARGIN}pt", "outside": f"{MARGIN}pt"}}},
        },
        "content": [{"type": "page", "id": "friday-morning", "master": "sheet", "fill": "paper", "elements": [
            {"type": "panels", "id": "friday-panels", "x": MARGIN, "y": MARGIN, "w": pw, "h": ph, "gutter": gutter, "rows": rows,
             "border": {"width": 1.6, "color": "ink"}}]}],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
