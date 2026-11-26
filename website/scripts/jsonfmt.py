"""Compact, readable JSON: a value stays on one line when it fits.

    python3 scripts/jsonfmt.py file.json [more.json …]   (rewrites in place)
"""

import json
import sys

WIDTH = 68


def _one(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def dumps(v, indent=0, lead=0, width=WIDTH):
    """`v` set at `indent`, with `lead` characters already on its first line."""
    one = _one(v)
    if lead + len(one) <= width or not isinstance(v, (dict, list)) or not v:
        return one
    pad = " " * (indent + 2)
    if isinstance(v, dict):
        # Short members share a line, as they would written by hand; a
        # member that needs lines of its own gets them.
        rows, cur = [], []
        for k, x in v.items():
            key = f"{json.dumps(k, ensure_ascii=False)}: "
            s = key + dumps(x, indent + 2, indent + 2 + len(key), width)
            if "\n" in s:
                if cur:
                    rows.append(", ".join(cur))
                    cur = []
                rows.append(s)
                continue
            if cur and len(pad) + len(", ".join(cur + [s])) + 1 > width:
                rows.append(", ".join(cur))
                cur = []
            cur.append(s)
        if cur:
            rows.append(", ".join(cur))
        return "{\n" + ",\n".join(pad + r for r in rows) + "\n" + " " * indent + "}"
    if all(not isinstance(x, (dict, list)) for x in v):
        # Scalars wrap like words in a paragraph.
        lines, cur = [], []
        for x in v:
            s = _one(x)
            if cur and len(pad) + len(", ".join(cur + [s])) + 1 > width:
                lines.append(", ".join(cur))
                cur = []
            cur.append(s)
        lines.append(", ".join(cur))
        return "[\n" + ",\n".join(pad + line for line in lines) + "\n" + " " * indent + "]"
    return "[\n" + ",\n".join(pad + dumps(x, indent + 2, indent + 2, width) for x in v) + "\n" + " " * indent + "]"


if __name__ == "__main__":
    for path in sys.argv[1:]:
        with open(path) as f:
            data = json.load(f)
        text = dumps(data) + "\n"
        assert json.loads(text) == data, path
        with open(path, "w") as f:
            f.write(text)
