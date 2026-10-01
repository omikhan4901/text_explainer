# Text Explainer

Select text you're reading anywhere, press one key, and a clearer version appears next to
your cursor. Fully offline, on an ordinary 8 GB laptop with no GPU.

> **Status:** in development (Windows first). See [`docs/PROGRESS.md`](docs/PROGRESS.md).

- **Readable rewrites, not definitions.** Long sentences, jargon and acronyms become plain
  language at the reading level you choose, keeping every number, date and name.
- **Never in your way.** The card appears next to what you're reading without taking focus
  from the app you're in. Esc closes it.
- **Runs on your machine.** A small open model through llama.cpp. Nothing you read leaves
  your computer.
- **Your model, your rules.** Pick a model that fits your RAM, bring your own GGUF, and tune
  every setting.

## Built with

Tauri 2 (Rust) · React + TypeScript · Tailwind CSS · llama.cpp · SQLite (WordNet)

## Development

See [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md) for the plan and
[`CLAUDE.md`](CLAUDE.md) for conventions.

```bash
npm ci
scripts/check.sh          # every check CI runs on Linux
npm run tauri dev         # run the app (Windows)
```

## License

MIT
