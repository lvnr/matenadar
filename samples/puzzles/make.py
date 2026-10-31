#!/usr/bin/env python3
"""Five Christmas puzzles and their solutions, drawn by the engine from their data.

    python3 samples/puzzles/make.py      # writes doc.json beside this file

The page and type follow Christmas Word Search (8 x 10 in, no bleed, Chaparral Pro): its folio, its margins. The
puzzles are made here, deterministically: a maze from a seeded depth-first carve, a Sudoku from a seeded fill with
cells taken away while the answer stays unique, a hand-made crossword whose words read across and down, a logic
grid with one answer, and a dot-to-dot tree with a star. Every puzzle is the engine's own element (maze,
letter-grid, logic-grid, dots), so a solution page is the same data with `solved` or without `blank`.
"""
from __future__ import annotations

import json
import math
import random
from pathlib import Path

HERE = Path(__file__).resolve().parent
W, H = 576.0, 720.0
M = {"top": 54.0, "bottom": 54.0, "inside": 75.0, "outside": 66.6}


def maze(cols: int, rows: int, seed: int) -> tuple[list[str], list[list[int]]]:
    """Walls per cell (1 top, 2 right, 4 bottom, 8 left) and the way from the top-left to the bottom-right."""
    rng = random.Random(seed)
    walls = [[15] * cols for _ in range(rows)]
    seen = {(0, 0)}
    stack = [(0, 0)]
    steps = ((-1, 0, 1, 4), (0, 1, 2, 8), (1, 0, 4, 1), (0, -1, 8, 2))
    while stack:
        r, c = stack[-1]
        nexts = [(r + dr, c + dc, b, o) for dr, dc, b, o in steps if 0 <= r + dr < rows and 0 <= c + dc < cols and (r + dr, c + dc) not in seen]
        if not nexts:
            stack.pop()
            continue
        nr, nc, b, o = rng.choice(nexts)
        walls[r][c] &= ~b
        walls[nr][nc] &= ~o
        seen.add((nr, nc))
        stack.append((nr, nc))
    walls[0][0] &= ~8
    walls[rows - 1][cols - 1] &= ~2
    prev = {(0, 0): None}
    queue = [(0, 0)]
    for r, c in queue:
        for dr, dc, b, _ in steps:
            if not walls[r][c] & b and (r + dr, c + dc) not in prev and 0 <= r + dr < rows and 0 <= c + dc < cols:
                prev[(r + dr, c + dc)] = (r, c)
                queue.append((r + dr, c + dc))
    way, at = [], (rows - 1, cols - 1)
    while at:
        way.append(list(at))
        at = prev[at]
    return ["".join(f"{v:x}" for v in row) for row in walls], way[::-1]


def sudoku(seed: int, keep: int) -> tuple[str, str]:
    rng = random.Random(seed)

    def solve(g: list[int], count: int = 2) -> int:
        try:
            i = g.index(0)
        except ValueError:
            return 1
        r, c = divmod(i, 9)
        used = {g[r * 9 + k] for k in range(9)} | {g[k * 9 + c] for k in range(9)}
        br, bc = r // 3 * 3, c // 3 * 3
        used |= {g[(br + a) * 9 + bc + b] for a in range(3) for b in range(3)}
        found = 0
        for v in range(1, 10):
            if v not in used:
                g[i] = v
                found += solve(g, count - found)
                g[i] = 0
                if found >= count:
                    break
        return found

    def fill(g: list[int]) -> bool:
        if 0 not in g:
            return True
        i = g.index(0)
        r, c = divmod(i, 9)
        vals = list(range(1, 10))
        rng.shuffle(vals)
        for v in vals:
            br, bc = r // 3 * 3, c // 3 * 3
            if all(g[r * 9 + k] != v and g[k * 9 + c] != v for k in range(9)) and all(g[(br + a) * 9 + bc + b] != v for a in range(3) for b in range(3)):
                g[i] = v
                if fill(g):
                    return True
                g[i] = 0
        return False

    full = [0] * 81
    fill(full)
    puzzle = full[:]
    order = list(range(81))
    rng.shuffle(order)
    for i in order:
        if sum(1 for v in puzzle if v) <= keep:
            break
        v, puzzle[i] = puzzle[i], 0
        if solve(puzzle[:]) != 1:
            puzzle[i] = v
    return "".join(map(str, full)), "".join(str(v) if v else "." for v in puzzle)


# A crossword whose rows 1, 3 and 5 read across and columns 1, 3 and 5 read down.
CROSSWORD = ["STARS", "L#N#W", "EAGLE", "D#E#E", "SPLIT"]
NUMBERS = {(0, 0): "1", (0, 2): "2", (0, 4): "3", (2, 0): "4", (4, 0): "5"}
CLUES = [
    ("Across", [("1", "They shine on the tree and over the stable"), ("4", "Bird on a carved ornament"), ("5", "Break a cracker in two")]),
    ("Down", [("1", "What the reindeer pull"), ("2", "She sits at the very top of the tree"), ("3", "Like a candy cane")]),
]

LOGIC = [
    {"name": "Child", "items": ["Ana", "Ben", "Cleo"]},
    {"name": "Gift", "items": ["Scarf", "Sled", "Book"]},
    {"name": "Paper", "items": ["Red", "Green", "Gold"]},
]
# The answer, as (child, gift, paper): Ana, the sled, red; Ben, the book, gold; Cleo, the scarf, green.
ANSWER = [(0, 1, 0), (1, 2, 2), (2, 0, 1)]
LOGIC_STORY = "Three children each found one gift under the tree, each in its own paper. Who found which gift, in which paper?"
LOGIC_CLUES = ["The sled was wrapped in red.", "Ana's paper was red.", "The scarf was not in gold.", "The book was not Cleo's."]


def logic_marks() -> list[dict]:
    """Every cell of the solved grid: a dot where two things go together, a cross elsewhere."""
    marks = []
    for a, b in [(0, 1), (0, 2), (2, 1)]:
        for i in range(3):
            for j in range(3):
                hit = any(row[a] == i and row[b] == j for row in ANSWER)
                marks.append({"a": [a, i], "b": [b, j], "mark": "o" if hit else "x"})
    return marks


def tree() -> tuple[list[list[float]], list[int]]:
    """A tree's outline, closed, then a star on its top, closed: two shapes."""
    outline = [(0, -100), (34, -58), (18, -58), (56, -14), (34, -14), (78, 36), (14, 36), (14, 70), (-14, 70), (-14, 36),
               (-78, 36), (-34, -14), (-56, -14), (-18, -58), (-34, -58)]
    pts = [[150 + x, 160 + y] for x, y in outline] + [[150, 60]]
    star = []
    for i in range(10):
        r = 26 if i % 2 == 0 else 11
        t = -math.pi / 2 + i * math.pi / 5
        star.append([150 + r * math.cos(t), 22 + r * math.sin(t)])
    return pts + [[round(x, 2), round(y, 2)] for x, y in star + [star[0]]], [len(pts)]


def text(id_: str, x: float, y: float, w: float, style: str, words: str, **more) -> dict:
    return {"type": "text", "id": id_, "x": x, "y": y, "w": w, "style": style, "text": words, **more}


def heading(n: int, name: str, how: str) -> list[dict]:
    x, w = M["inside"], W - M["inside"] - M["outside"]
    return [text(f"p{n}-kicker", x, 58, w, "kicker", f"Puzzle {n}"), text(f"p{n}-title", x, 76, w, "title", name),
            text(f"p{n}-how", x, 118, w, "how", how)]


def page(id_: str, elements: list[dict]) -> dict:
    return {"type": "page", "id": id_, "master": "page", "layers": True, "elements": elements}


def main() -> None:
    walls, way = maze(16, 20, seed=24)
    solution, given = sudoku(seed=12, keep=30)
    dots, breaks = tree()
    x0, w = M["inside"], W - M["inside"] - M["outside"]
    cw = 24.0
    maze_el = {"type": "maze", "id": "maze", "x": x0 + (w - 16 * cw) / 2, "y": 170, "cell": cw, "rows": walls,
               "start": [0, 0], "end": [19, 15], "solution": way, "walls": {"width": 1.6, "color": "ink"}, "marks": "ink"}

    def crossword(id_: str, blank: bool, cell: float, x: float, y: float) -> dict:
        rows = []
        for r, line in enumerate(CROSSWORD):
            row = []
            for c, ch in enumerate(line):
                if ch == "#":
                    row.append({"block": True})
                elif (r, c) in NUMBERS:
                    row.append({"text": ch, "number": NUMBERS[(r, c)]})
                else:
                    row.append(ch)
            rows.append(row)
        return {"type": "letter-grid", "id": id_, "x": x, "y": y, "cell": cell, "rows": rows, "blank": blank, "style": "grid",
                "lines": {"width": 0.8, "color": "ink"}, "frame": {"width": 1.6, "color": "ink"}}

    def sudoku_el(id_: str, blank: bool, cell: float, x: float, y: float) -> dict:
        rows = [[{"text": solution[r * 9 + c], "given": True} if given[r * 9 + c] != "." else
                 ({"text": solution[r * 9 + c], "color": "soft"} if not blank else solution[r * 9 + c]) for c in range(9)] for r in range(9)]
        return {"type": "letter-grid", "id": id_, "x": x, "y": y, "cell": cell, "rows": rows, "blank": blank, "regions": [3, 3],
                "style": "grid", "lines": {"width": 0.6, "color": "ink"}, "region_lines": {"width": 2, "color": "ink"}}

    def logic(id_: str, solved: bool, y: float) -> dict:
        return {"type": "logic-grid", "id": id_, "x": x0 + (w - 300) / 2, "y": y, "cell": 34, "style": "grid-label",
                "categories": LOGIC, "marks": logic_marks() if solved else [],
                "lines": {"width": 0.6, "color": "ink"}, "frame": {"width": 1.4, "color": "ink"}}

    def clues(prefix: str, x: float, y: float, w: float, head: str, items: list[tuple[str, str]]) -> list[dict]:
        out = [text(f"{prefix}-head", x, y, w, "clue-head", head)]
        for i, (n, clue) in enumerate(items):
            out.append({"type": "text", "id": f"{prefix}-{i + 1}", "x": x, "y": y + 22 + i * 19, "w": w, "style": "clue",
                        "content": [{"text": n + "  ", "weight": "bold"}, clue]})
        return out

    clue_blocks = []
    for k, (head, items) in enumerate(CLUES):
        clue_blocks += clues(f"cw-{head.lower()}", x0 + k * (w / 2 + 10), 470, w / 2 - 10, head, items)
    content = [
        page("puzzle-1", heading(1, "The long way to the stable", "Start at the dot, end at the ring.") + [maze_el]),
        page("puzzle-2", heading(2, "A small crossword", "Rows 1, 3 and 5 read across; columns 1, 3 and 5 read down.")
             + [crossword("crossword-grid", True, 50, x0 + (w - 250) / 2, 180)] + clue_blocks),
        page("puzzle-3", heading(3, "Nine by nine", "Every row, column and box holds 1 to 9 once.") + [sudoku_el("sudoku-grid", True, 40, x0 + (w - 360) / 2, 180)]),
        page("puzzle-4", heading(4, "Under the tree", LOGIC_STORY) + [logic("logic-grid", False, 196)]
             + clues("logic-clues", x0, 470, w, "Clues", [(f"{i + 1}.", c) for i, c in enumerate(LOGIC_CLUES)])),
        page("puzzle-5", heading(5, "Join the dots", "From 1, in order; the star last.")
             + [{"type": "dots", "id": "dots-puzzle", "x": x0 + (w - 380) / 2, "y": 170, "w": 380, "h": 450, "points": dots, "breaks": breaks, "viewbox": [40, -12, 220, 282], "style": "dot-number", "size": 8}]),
        page("solutions-1", [text("s1-title", x0, 58, w, "title", "Solutions"),
                             text("s1-maze", x0, 110, w, "kicker", "Puzzle 1"),
                             {**maze_el, "id": "maze-solved", "cell": 12, "x": x0, "y": 130, "solved": True, "walls": {"width": 1, "color": "ink"}},
                             text("s1-cw", x0 + 230, 110, w, "kicker", "Puzzle 2"),
                             crossword("crossword-solved", False, 34, x0 + 230, 130),
                             text("s1-sd", x0, 400, w, "kicker", "Puzzle 3"),
                             sudoku_el("sudoku-solved", False, 26, x0, 420)]),
        page("solutions-2", [text("s2-title", x0, 58, w, "title", "Solutions"),
                             text("s2-logic", x0, 110, w, "kicker", "Puzzle 4"),
                             logic("logic-solved", True, 136),
                             text("s2-dots", x0, 420, w, "kicker", "Puzzle 5"),
                             {"type": "dots", "id": "dots-solved", "x": x0 + 60, "y": 440, "w": 200, "h": 236, "points": dots, "breaks": breaks, "viewbox": [40, -12, 220, 282], "solved": True, "style": "dot-number", "size": 6}]),
    ]
    doc = {
        "matenadar": "1",
        "meta": {"title": "Christmas Puzzles", "lang": "en-US"},
        "design": {
            "colors": {"ink": "#000000", "soft": "#6b6b6b"},
            "fonts": {"serif": ["Chaparral Pro", "Libertinus Serif"]},
            "base": {"font": "serif", "size": 11, "line_height": 1.35, "color": "ink"},
            "styles": {
                "kicker": {"size": 10, "case": "upper", "tracking": "0.14em", "color": "soft"},
                "title": {"size": 28, "weight": 600, "line_height": 1.15},
                "how": {"size": 12, "italic": True, "line_height": 1.4},
                "grid": {"size": 20, "align": "center"},
                "grid-label": {"size": 11},
                "clue-head": {"size": 12, "weight": 600, "case": "upper", "tracking": "0.1em"},
                "clue": {"size": 11.5, "line_height": 1.35, "hanging": "1.2em"},
                "dot-number": {"size": 8},
                "folio": {"size": 10, "align": "center", "color": "soft"},
            },
            "masters": {"page": {
                "width": f"{W}pt", "height": f"{H}pt",
                "margins": {k: f"{v}pt" for k, v in M.items()},
                "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": 66.6, "y": 684, "w": 442.8, "content": [{"field": "page"}]}],
            }},
        },
        "content": content,
    }
    (HERE / "doc.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
