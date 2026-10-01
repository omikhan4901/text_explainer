"""Builds the bundled dictionary (SQLite) from WordNet 3.1.

Sources (installed by `npm ci` as dev dependencies):
- wordnet-db: WordNet 3.1 index and data files (Princeton WordNet licence).
- wink-lexicon: WordNet's irregular forms ("mice" → "mouse", "went" → "go"), MIT.

Output: src-tauri/resources/dictionary.sqlite (not committed), with the WordNet licence
next to it. Run: python scripts/build-dictionary.py
"""

import json
import pathlib
import sqlite3
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DICT = ROOT / "node_modules" / "wordnet-db" / "dict"
OUT_DIR = ROOT / "src-tauri" / "resources"
OUT = OUT_DIR / "dictionary.sqlite"
POS = {"noun": "n", "verb": "v", "adj": "a", "adv": "r"}
SENSES_PER_POS = 3
SYNONYMS = 4


def parse_data(path: pathlib.Path) -> dict[int, tuple[str, list[str], list[str]]]:
    """offset → (definition, examples, words in the synset)."""
    out = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if line.startswith("  "):
                continue  # licence header
            head, _, gloss = line.partition(" | ")
            fields = head.split()
            offset = int(fields[0])
            count = int(fields[3], 16)
            words = [fields[4 + 2 * i].replace("_", " ") for i in range(count)]
            words = [w.split("(")[0] for w in words]  # adjective markers like "(a)"
            parts = [p.strip() for p in gloss.strip().split(";")]
            definition = "; ".join(p for p in parts if not p.startswith('"'))
            examples = [p.strip('"') for p in parts if p.startswith('"')]
            out[offset] = (definition, examples, words)
    return out


def exceptions() -> list[tuple[str, str, str]]:
    script = (
        "const out = [];"
        "for (const [pos, file] of [['n','wn-noun-exceptions'],['v','wn-verb-exceptions'],['a','wn-adjective-exceptions']]) {"
        "  const e = require('wink-lexicon/src/' + file + '.js');"
        "  for (const form of Object.keys(e)) out.push([form, e[form], pos]);"
        "}"
        "process.stdout.write(JSON.stringify(out));"
    )
    raw = subprocess.run(["node", "-e", script], cwd=ROOT, capture_output=True, text=True, check=True).stdout
    return [tuple(x) for x in json.loads(raw)]


def main() -> int:
    if not DICT.exists():
        print("WordNet not found; run `npm ci` first.", file=sys.stderr)
        return 1
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    tmp = OUT.with_suffix(".tmp")
    tmp.unlink(missing_ok=True)
    db = sqlite3.connect(tmp)
    db.executescript(
        """
        PRAGMA page_size = 4096;
        CREATE TABLE senses (lemma TEXT NOT NULL, pos TEXT NOT NULL, rank INTEGER NOT NULL,
                             definition TEXT NOT NULL, examples TEXT NOT NULL, synonyms TEXT NOT NULL);
        CREATE TABLE forms (form TEXT NOT NULL, lemma TEXT NOT NULL, pos TEXT NOT NULL);
        CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        """
    )
    rows = 0
    for name, pos in POS.items():
        data = parse_data(DICT / f"data.{name}")
        with open(DICT / f"index.{name}", encoding="utf-8", errors="replace") as f:
            for line in f:
                if line.startswith("  "):
                    continue
                fields = line.split()
                lemma = fields[0]
                synset_count = int(fields[2])
                offsets = [int(o) for o in fields[-synset_count:]]
                display = lemma.replace("_", " ")
                for rank, off in enumerate(offsets[:SENSES_PER_POS]):
                    definition, examples, words = data[off]
                    synonyms = [w for w in words if w.lower() != display.lower()][:SYNONYMS]
                    db.execute(
                        "INSERT INTO senses VALUES (?,?,?,?,?,?)",
                        (display.lower(), pos, rank, definition, json.dumps(examples[:1]), json.dumps(synonyms)),
                    )
                    rows += 1
    forms = exceptions()
    db.executemany("INSERT INTO forms VALUES (?,?,?)", [(f.lower(), l.lower(), p) for f, l, p in forms])
    db.execute("INSERT INTO meta VALUES ('source', 'WordNet 3.1 (Princeton University); irregular forms via wink-lexicon')")
    db.execute("CREATE INDEX senses_lemma ON senses (lemma)")
    db.execute("CREATE INDEX forms_form ON forms (form)")
    db.commit()
    db.execute("VACUUM")
    db.close()
    tmp.replace(OUT)
    licence = (ROOT / "node_modules" / "wordnet-db" / "LICENSE").read_text(encoding="utf-8")
    (OUT_DIR / "WORDNET-LICENSE.txt").write_text(licence, encoding="utf-8")
    print(f"{OUT}: {rows} senses, {len(forms)} irregular forms, {OUT.stat().st_size / 1e6:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
