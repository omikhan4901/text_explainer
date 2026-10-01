# Evaluation corpus

`corpus.jsonl`: passages written for this project in the style of real documents
(leases, discharge letters, abstracts, API docs, tax and visa rules, finance, news), plus
a few in Spanish, Bangla, Hindi and French for the language guard. One JSON object per
line: `id`, `domain`, `lang`, `text`.

Run by `crates/eval` (see `.github/workflows/eval.yml`); results are in `docs/models.md`.
