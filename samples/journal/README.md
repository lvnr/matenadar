# The Leave Note, as a dated journal

The Attention Repair Manual's Leave Note (its four prompts and the line under its heading, from
`manuscript/06-field-guide.md`) as a week of journal pages, with a month to tick off, a dot-grid page and a
graph-paper page. 6 x 9 in, the book's trim. The book set Warnock Pro and Halyard; this computer has neither, so
Minion Pro and Myriad Pro stand in.

```
python3 samples/journal/make.py
bin/matenadar build samples/journal/doc.json -o samples/journal/build/journal.pdf --png samples/journal/build/png
```

| page | shows |
|---|---|
| 1 | `calendar`: November 2026 from its first weekday (a Sunday), Monday first, the months either side in grey |
| 2–8 | one page in the document, `repeat`ed seven times from 2 November: each time its date as words (a field with a `format`) and its day number (`n`); `ruling` lines under each prompt |
| 9 | `ruling` dots, 5 mm apart, centred |
| 10 | `ruling` graph paper, 0.1 in squares, every fifth line heavier |

Looked at: all ten pages at 90 dpi. The dated pages keep one x for both sides of the spread (elements are placed
from the page's left edge), so versos sit 13 pt further from their outer edge than rectos.
