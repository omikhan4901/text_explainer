"""Narration: speaks each line of narration.json with Kokoro (local, Apache-2.0), trims the
silence around it, then times every word with faster-whisper so the picture can land on
the spoken words.

Usage (from the repo root):
  docs/marketing/video/.venv/Scripts/python.exe -I docs/marketing/video/scripts/voice.py

Writes audio/vo/<scene>.wav and work/vo.json ({scene: {duration, text, heard, words}}).
"""

import json
import re
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from faster_whisper import WhisperModel
from kokoro_onnx import Kokoro

VIDEO = Path("docs/marketing/video")
MODELS = VIDEO / "models"
OUT = VIDEO / "audio" / "vo"
REPORT = VIDEO / "work" / "vo.json"


def trim(audio, rate, floor_db=-45.0, pad=0.06):
    """Cuts leading and trailing silence, keeping a short natural pad."""
    level = 20 * np.log10(np.abs(audio) + 1e-9)
    loud = np.where(level > floor_db)[0]
    if not len(loud):
        return audio
    start = max(0, loud[0] - int(pad * rate))
    end = min(len(audio), loud[-1] + int(pad * rate * 2))
    return audio[start:end]


def main():
    spec = json.loads((VIDEO / "narration.json").read_text(encoding="utf-8"))
    kokoro = Kokoro(str(MODELS / "kokoro-v1.0.onnx"), str(MODELS / "voices-v1.0.bin"))
    OUT.mkdir(parents=True, exist_ok=True)
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    gap = spec.get("sentence_gap", 0.34)
    for line in spec["lines"]:
        # Each sentence on its own, joined by a real pause: the model alone runs them together.
        parts = []
        for sentence in re.split(r"(?<=[.?!])\s+", line["text"].strip()):
            audio, rate = kokoro.create(sentence, voice=spec["voice"], speed=spec["speed"], lang="en-us")
            parts.append(trim(np.asarray(audio, dtype=np.float32), rate))
            parts.append(np.zeros(int(gap * rate), dtype=np.float32))
        sf.write(OUT / f"{line['scene']}.wav", np.concatenate(parts[:-1]), rate, subtype="PCM_16")

    # Word timings, and a check that each line says what it should.
    whisper = WhisperModel("small.en", device="cpu", compute_type="int8")
    report = {}
    for line in spec["lines"]:
        path = OUT / f"{line['scene']}.wav"
        info = sf.info(path)
        audio, rate = sf.read(path, dtype="float32")
        # Whisper wants 16 kHz mono; passing an array avoids its file decoder.
        x = np.linspace(0, len(audio) - 1, int(len(audio) * 16000 / rate))
        audio16 = np.interp(x, np.arange(len(audio)), audio).astype(np.float32)
        segments, _ = whisper.transcribe(audio16, word_timestamps=True, language="en", beam_size=5)
        words = [
            {"w": w.word.strip(), "start": round(w.start, 3), "end": round(w.end, 3)}
            for seg in segments
            for w in seg.words
        ]
        heard = " ".join(w["w"] for w in words)
        report[line["scene"]] = {"duration": round(info.duration, 3), "text": line["text"], "heard": heard, "words": words}
        print(f"{line['scene']:7} {info.duration:5.2f}s  {heard}")
    REPORT.write_text(json.dumps(report, indent=1, ensure_ascii=False), encoding="utf-8")


if __name__ == "__main__":
    sys.exit(main())
