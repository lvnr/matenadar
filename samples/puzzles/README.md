# Christmas puzzles, drawn from their data

Five puzzles and two pages of solutions, every one an engine element drawn from data: a `maze`, a crossword and a
Sudoku (`letter-grid` with `blank`, `given` and `regions`), a `logic-grid` and a `dots` dot-to-dot. A solution page
is the same data with `solved: true`, or without `blank`. The page follows Christmas Word Search (8 x 10 in, no
bleed, Chaparral Pro, its folio on every page through `layers: true`).

```
python3 samples/puzzles/make.py
bin/matenadar build samples/puzzles/doc.json -o samples/puzzles/build/puzzles.pdf --png samples/puzzles/build/png
```

`make.py` makes the puzzles, deterministically: the maze is a seeded depth-first carve (16 x 20) with its way
through found by search; the Sudoku is a seeded fill with cells taken away while the answer stays unique (30
givens); the crossword is hand-made, rows 1, 3, 5 across and columns 1, 3, 5 down; the logic grid has one answer
from its four clues; the dot-to-dot is a tree and a star, two closed shapes (the last point of each returns to its
first and isn't numbered again).

| page | shows |
|---|---|
| 1 | the maze: walls merged into long strokes, the start a dot, the end a ring |
| 2 | the crossword: numbers, blocks, letters left out; clues as text frames |
| 3 | the Sudoku: givens only, 3 x 3 regions in heavier lines |
| 4 | the logic grid in its staircase, with its story and clues |
| 5 | the dot-to-dot: numbers set outside each turn, and outside the shapes at their inner corners |
| 6 | solutions: the maze's way through in red, the crossword and the Sudoku filled (answers in grey, givens in black) |
| 7 | solutions: the logic grid's dots and crosses, the dot-to-dot drawn |

Looked at: all seven pages at 90 and 110 dpi. Build: 7 pages, layout about 20 ms.
