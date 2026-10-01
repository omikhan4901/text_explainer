"""Downloads the pinned llama.cpp build and checks its SHA-256.

Default: the Windows CPU build; llama-server.exe and its DLLs go to
src-tauri/resources/llama/ for bundling with the app.

--linux: the Linux build into target/llama-linux/ (for the model evaluation workflow,
which runs on Linux runners).

The pins come from the probe workflow (.github/workflows/probe.yml). To update llama.cpp,
run the probe, change VERSION and the hashes here, and re-run the evaluation."""

import hashlib
import io
import pathlib
import sys
import tarfile
import urllib.request
import zipfile

VERSION = "b11323"
WINDOWS_ASSET = f"llama-{VERSION}-bin-win-cpu-x64.zip"
WINDOWS_SHA256 = "bc984e5e0f0337f89c2364cbc24274c31a4ba83f9e8ceb497abb85b8d5046a95"
LINUX_ASSET = f"llama-{VERSION}-bin-ubuntu-x64.tar.gz"
LINUX_SHA256 = "6b8801b592f19d838a0c5e6bbf6282f8f0788d8c99561b98cc4cd3ef558346f5"

ROOT = pathlib.Path(__file__).resolve().parent.parent
WINDOWS_DEST = ROOT / "src-tauri" / "resources" / "llama"
LINUX_DEST = ROOT / "target" / "llama-linux"


def fetch(asset: str, sha: str) -> bytes:
    url = f"https://github.com/ggml-org/llama.cpp/releases/download/{VERSION}/{asset}"
    print(f"Downloading {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "text-explainer-build"})
    with urllib.request.urlopen(req, timeout=300) as r:
        data = r.read()
    got = hashlib.sha256(data).hexdigest()
    if got != sha:
        raise SystemExit(f"SHA-256 mismatch for {asset}: got {got}, expected {sha}")
    return data


def windows() -> int:
    stamp = WINDOWS_DEST / "VERSION"
    if stamp.exists() and stamp.read_text().strip() == VERSION and (WINDOWS_DEST / "llama-server.exe").exists():
        print(f"llama.cpp {VERSION} already in {WINDOWS_DEST}")
        return 0
    data = fetch(WINDOWS_ASSET, WINDOWS_SHA256)
    WINDOWS_DEST.mkdir(parents=True, exist_ok=True)
    for old in WINDOWS_DEST.iterdir():
        if old.name != "README.md":
            old.unlink()
    kept = []
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for info in z.infolist():
            name = pathlib.PurePosixPath(info.filename).name
            if info.is_dir() or not name:
                continue
            if name == "llama-server.exe" or name.lower().endswith(".dll") or name.upper().startswith("LICENSE"):
                (WINDOWS_DEST / name).write_bytes(z.read(info))
                kept.append(name)
    if "llama-server.exe" not in kept:
        raise SystemExit("llama-server.exe not found in the archive")
    stamp.write_text(VERSION + "\n")
    print(f"Kept {len(kept)} files: {', '.join(sorted(kept))}")
    return 0


def linux() -> int:
    import shutil
    import tempfile

    data = fetch(LINUX_ASSET, LINUX_SHA256)
    with tempfile.TemporaryDirectory() as tmp:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as t:
            t.extractall(tmp, filter="data")
        servers = list(pathlib.Path(tmp).rglob("llama-server"))
        if not servers:
            raise SystemExit("llama-server not found in the archive")
        # The libraries sit next to the server (some as symlinks); keep them together.
        if LINUX_DEST.exists():
            shutil.rmtree(LINUX_DEST)
        shutil.copytree(servers[0].parent, LINUX_DEST, symlinks=True)
    print(f"llama.cpp {VERSION} Linux build in {LINUX_DEST}: {', '.join(sorted(p.name for p in LINUX_DEST.iterdir()))}")
    return 0


if __name__ == "__main__":
    sys.exit(linux() if "--linux" in sys.argv else windows())
