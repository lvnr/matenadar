#!/usr/bin/env python3
"""A seek-and-find cube from I Just Had It: a sheet to cut, fold and glue.

    python3 samples/paper-craft/make.py      # writes doc.json beside this file

Each face is one of the book's scenes (build/scenes/<id>.svg), cut square round the thing that was lost (its
position is in build/scenes/<id>.json); the list of what to find is the book's own headings
(manuscript/scenes.json). The net is the usual cross of six squares. Its fourteen outer edges close up in seven
pairs, and one edge of each pair carries the glue tab; the five edges between faces fold. The sheet is one craft
element, so the faces, tabs and lines move together and stay vector.
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = json.loads((HERE / "sample.json").read_text())
BOOK = Path(SAMPLE["book"])
SCENES = {s["id"]: s for s in json.loads((BOOK / "manuscript/scenes.json").read_text())["scenes"]}
W, H = 504.0, 720.0
U = 104.0
FACES = ["glasses", "phone", "keys", "wallet", "charger", "bunny"]
# Each face's corner in the net, in squares: the cross.
AT = [(1, 0), (0, 1), (1, 1), (2, 1), (3, 1), (1, 2)]


def square_crop(scene: str, w: float) -> dict:
    info = json.loads((BOOK / f"build/scenes/{scene}.json").read_text())
    mw, mh = info["media_in"]
    t = next(f for f in info["find"] if f["role"] == "target")
    h = w * mw / mh
    x = min(max(t["x"] / mw - w / 2, 0.0), 1.0 - w)
    y = min(max(t["y"] / mh - h / 2, 0.0), 1.0 - h)
    return {"x": round(x, 4), "y": round(y, 4), "w": w, "h": round(h, 4)}


def main() -> None:
    link = HERE / "book"
    if not link.exists():
        link.symlink_to(BOOK)
    faces = []
    for scene, (cx, cy) in zip(FACES, AT):
        x, y = cx * U, cy * U
        faces.append({"points": [[x, y], [x + U, y], [x + U, y + U], [x, y + U]], "src": f"book/build/scenes/{scene}.svg",
                      "crop": square_crop(scene, 0.5)})

    def edge(a: tuple[float, float], b: tuple[float, float], face: int, tab: bool) -> dict:
        """An outer edge of `face`; a tab, when it has one, points away from the face."""
        e = {"from": [a[0] * U, a[1] * U], "to": [b[0] * U, b[1] * U]}
        if tab:
            fx, fy = AT[face][0] + 0.5, AT[face][1] + 0.5
            ux, uy = b[0] - a[0], b[1] - a[1]
            mx, my = (a[0] + b[0]) / 2 - fx, (a[1] + b[1]) / 2 - fy
            e["tab"] = {"depth": 22, "angle": 50, "side": "left" if uy * mx - ux * my > 0 else "right"}
        return e

    edges = [
        edge((1, 0), (2, 0), 0, False), edge((1, 0), (1, 1), 0, True), edge((2, 0), (2, 1), 0, True),
        edge((0, 1), (1, 1), 1, False), edge((0, 1), (0, 2), 1, False), edge((0, 2), (1, 2), 1, False),
        edge((2, 1), (3, 1), 3, False), edge((2, 2), (3, 2), 3, False),
        edge((3, 1), (4, 1), 4, True), edge((4, 1), (4, 2), 4, True), edge((3, 2), (4, 2), 4, False),
        edge((1, 2), (1, 3), 5, True), edge((1, 3), (2, 3), 5, True), edge((2, 2), (2, 3), 5, True),
    ]
    for a, b in [((1, 1), (2, 1)), ((1, 1), (1, 2)), ((2, 1), (2, 2)), ((3, 1), (3, 2)), ((1, 2), (2, 2))]:
        edges.append({"from": [a[0] * U, a[1] * U], "to": [b[0] * U, b[1] * U], "kind": "valley"})
    pad = 30.0
    net_w, net_h = 4 * U + 2 * pad, 3 * U + 2 * pad
    find = ", ".join(SCENES[s]["head"].rstrip("?").replace("My ", "my ") for s in FACES)
    left = (W - 4 * U) / 2
    doc = {
        "matenadar": "1",
        "meta": {"title": "I Just Had It: the seek-and-find cube", "lang": "en-US"},
        "fonts": ["book/design/fonts"],
        "design": {
            "colors": {"ink": "#151a2b", "accent": "#e5553f", "glue": "#e8e2d4"},
            "base": {"font": "SIText", "size": 11, "line_height": 1.35, "color": "ink"},
            "styles": {
                "kicker": {"font": "Courier Prime", "weight": 700, "size": 8, "case": "upper", "tracking": "0.09em", "color": "accent"},
                "title": {"font": "SILead", "size": 40, "line_height": 1.0, "tracking": "-0.015em"},
                "how": {"font": "SIText", "size": 11.5, "line_height": 1.4},
                "find": {"font": "SITextItalic", "size": 11.5, "line_height": 1.4},
                "craft": {"font": "Courier Prime", "weight": 700, "size": 7, "case": "upper", "tracking": "0.08em"},
            },
            "masters": {"sheet": {"width": f"{W}pt", "height": f"{H}pt",
                                  "margins": {"top": "0.6in", "bottom": "0.6in", "inside": "0.6in", "outside": "0.6in"}}},
        },
        "content": [{"type": "page", "id": "cube", "master": "sheet", "elements": [
            {"type": "text", "id": "cube-kicker", "x": left, "y": 48, "w": 4 * U, "style": "kicker", "text": "I Just Had It · a sheet to make"},
            {"type": "text", "id": "cube-title", "x": left, "y": 64, "w": 4 * U, "style": "title", "text": "The seek-and-find cube"},
            {"type": "text", "id": "cube-how", "x": left, "y": 118, "w": 4 * U, "style": "how",
             "text": "Cut along the solid lines. Fold every dashed line away from you, then glue each shaded tab under the face it meets."},
            {"type": "text", "id": "cube-find", "x": left, "y": 160, "w": 4 * U, "style": "find", "text": f"On its six faces, find {find}."},
            {"type": "craft", "id": "cube-net", "x": left - pad, "y": 230, "w": net_w, "h": net_h, "viewbox": [-pad, -pad, net_w, net_h],
             "faces": faces, "edges": edges, "glue": "glue", "scissors": True, "style": "craft",
             "cut": {"width": 1, "color": "ink"}, "valley": {"width": 0.8, "color": "ink"}},
        ]}],
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
