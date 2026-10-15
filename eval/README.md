# Evaluation corpus

`corpus.jsonl`: passages written for this project in the style of real documents
(leases, discharge letters, abstracts, API docs, tax and visa rules, finance, news), plus
a few in Spanish, Bangla, Hindi and French for the language guard. One JSON object per
line: `id`, `domain`, `lang`, `text`.

`translate.jsonl`: English items for AI Translate, written for this project: sentences
with money, percentages, doses, dates in two notations, times, names and an acronym, plus
three single words with the sentence they appear in (`context`). One JSON object per line:
`id`, `domain`, `text`, and `context` for words. `te-eval --task translate` runs each into
Bangla, Spanish, Hindi, Arabic and Chinese (`--targets`).

Run by `crates/eval` (see `.github/workflows/eval.yml`); results are in `docs/models.md`.
