"""Downloads the pinned llama.cpp Windows CPU build, checks its SHA-256, and puts
llama-server.exe and its DLLs in src-tauri/resources/llama/ for bundling.

The pin comes from the probe workflow (.github/workflows/probe.yml). To update llama.cpp,
run the probe, then change VERSION, ASSET and SHA256 here and test with the eval."""

import hashlib
import io
import pathlib
import sys
import urllib.request
import zipfile

VERSION = "b11323"
ASSET = f"llama-{VERSION}-bin-win-cpu-x64.zip"
SHA256 = "bc984e5e0f0337f89c2364cbc24274c31a4ba83f9e8ceb497abb85b8d5046a95"
URL = f"https://github.com/ggml-org/llama.cpp/releases/download/{VERSION}/{ASSET}"

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEST = ROOT / "src-tauri" / "resources" / "llama"


def main() -> int:
    stamp = DEST / "VERSION"
    if stamp.exists() and stamp.read_text().strip() == VERSION and (DEST / "llama-server.exe").exists():
        print(f"llama.cpp {VERSION} already in {DEST}")
        return 0
    print(f"Downloading {URL}")
    req = urllib.request.Request(URL, headers={"User-Agent": "text-explainer-build"})
    with urllib.request.urlopen(req, timeout=300) as r:
        data = r.read()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        print(f"SHA-256 mismatch: got {got}, expected {SHA256}", file=sys.stderr)
        return 1
    DEST.mkdir(parents=True, exist_ok=True)
    for old in DEST.iterdir():
        if old.name != "README.md":
            old.unlink()
    kept = []
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for info in z.infolist():
            name = pathlib.PurePosixPath(info.filename).name
            if info.is_dir() or not name:
                continue
            if name == "llama-server.exe" or name.lower().endswith(".dll") or name.upper().startswith("LICENSE"):
                (DEST / name).write_bytes(z.read(info))
                kept.append(name)
    if "llama-server.exe" not in kept:
        print("llama-server.exe not found in the archive", file=sys.stderr)
        return 1
    stamp.write_text(VERSION + "\n")
    print(f"Kept {len(kept)} files: {', '.join(sorted(kept))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
