"""Lists the real GGUF files (size, SHA-256) for candidate models and the latest llama.cpp
Windows build, so the model catalog and the bundled engine are pinned to facts.
Run by .github/workflows/probe.yml (this build container can't reach Hugging Face)."""

import json
import os
import urllib.request

SEARCHES = ["Qwen3.5-4B", "Qwen3.5-2B", "Qwen3.5-9B", "gemma-4-E4B-it", "gemma-4-E2B-it", "LFM2.5-1.2B-Instruct"]
PREFERRED_OWNERS = ["unsloth", "ggml-org", "Qwen", "google", "bartowski", "lmstudio-community", "LiquidAI"]
QUANTS = ["Q4_K_M", "Q4_0", "Q5_K_M", "Q8_0", "UD-Q4_K_XL", "IQ4_XS"]


def get(url, token=None):
    req = urllib.request.Request(url, headers={"User-Agent": "text-explainer-probe"})
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def main():
    for term in ([] if os.environ.get("ONLY_ENGINE") else SEARCHES):
        print(f"\n### search: {term}")
        try:
            repos = get(f"https://huggingface.co/api/models?search={term}&filter=gguf&limit=40")
        except Exception as e:  # noqa: BLE001
            print("  search failed:", e)
            continue
        ids = [m["id"] for m in repos]
        ids.sort(key=lambda i: (PREFERRED_OWNERS.index(i.split("/")[0]) if i.split("/")[0] in PREFERRED_OWNERS else 99, i))
        for repo in ids[:6]:
            print(f"  repo {repo}")
            try:
                files = get(f"https://huggingface.co/api/models/{repo}/tree/main")
            except Exception as e:  # noqa: BLE001
                print("    tree failed:", e)
                continue
            for f in files:
                path = f.get("path", "")
                if path.endswith(".gguf") and any(q in path for q in QUANTS):
                    lfs = f.get("lfs") or {}
                    print(f"    {path}  size={f.get('size')}  sha256={lfs.get('oid')}")
                elif path.startswith("mmproj") or path in ("LICENSE", "README.md"):
                    pass
    print("\n### llama.cpp recent releases")
    rels = get("https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=4", os.environ.get("GITHUB_TOKEN"))
    for rel in rels:
        print("tag", rel["tag_name"], "published", rel["published_at"], "assets", len(rel["assets"]))
        for a in rel["assets"]:
            print(f"  {a['name']}  size={a['size']}  digest={a.get('digest')}")


if __name__ == "__main__":
    main()
