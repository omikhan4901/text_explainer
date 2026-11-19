"""Mixes the soundtrack: narration on top, music ducked under it, and the sound effects the
composition logged (work/sfx.json, from cues.mjs), all on the timing from timing.py.
Writes composition/assets/audio/soundtrack.wav.

Usage (from the repo root): python docs/marketing/video/scripts/mix.py

Sources in docs/marketing/video/audio/: the music (Sascha Ende, CC BY 4.0), CC0 effects
(Kenney; keyboard by unicae_games) and the narration (vo/, made by voice.py with Kokoro).
Whooshes, swishes and the riser are synthesized here.
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

VIDEO = Path("docs/marketing/video")
SRC = VIDEO / "audio"
OUT = VIDEO / "composition" / "assets" / "audio" / "soundtrack.wav"
STEMS = VIDEO / "out" / "editable" / "audio"
MUSIC = SRC / "happy-beats-business-moves-vol-12-by-ende-dot-app.mp3"
TIMING = json.loads((VIDEO / "work" / "timing.json").read_text(encoding="utf-8"))
CUES = json.loads((VIDEO / "work" / "sfx.json").read_text(encoding="utf-8"))

SYNTH = {
    "whoosh": ("anoisesrc=color=pink:duration=0.6:amplitude=0.7:seed=7", "highpass=f=280,lowpass=f=3800,afade=t=in:d=0.38:curve=exp,afade=t=out:st=0.38:d=0.22"),
    "whoosh_long": ("anoisesrc=color=pink:duration=0.7:amplitude=0.8:seed=11", "highpass=f=200,lowpass=f=5200,afade=t=in:d=0.5:curve=exp,afade=t=out:st=0.5:d=0.2"),
    "swish": ("anoisesrc=color=white:duration=0.38:amplitude=0.4:seed=3", "highpass=f=1800,lowpass=f=8000,afade=t=in:d=0.16,afade=t=out:st=0.16:d=0.22"),
    "riser": ("anoisesrc=color=pink:duration=0.9:amplitude=0.6:seed=5", "highpass=f=700,lowpass=f=7000,afade=t=in:d=0.88:curve=exp"),
}
FILES = {
    "impact_heavy": "impactSoft_heavy_000.ogg",
    "impact_heavy2": "impactSoft_heavy_001.ogg",
    "impact_med": "impactSoft_medium_001.ogg",
    "impact_med2": "impactSoft_medium_000.ogg",
    "tick": "click_002.ogg",
    "click": "mouseclick1.ogg",
    "pop": "drop_002.ogg",
    "pop2": "drop_001.ogg",
    "bong": "bong_001.ogg",
    "switch": "switch_002.ogg",
    "rollover": "rollover2.ogg",
    "bell": "impactBell_heavy_000.ogg",
}


def sound_file(name, synth):
    if name in synth:
        return synth[name]
    if name in FILES:
        return SRC / FILES[name]
    if name.startswith("key"):
        return SRC / f"keypress-{name[3:]}.wav"
    if name.startswith("type"):
        return SRC / f"keypress-{(int(name[4:]) * 7) % 32 + 1:03d}.wav"
    sys.exit(f"unknown sound: {name}")


def run(cmd):
    result = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if result.returncode:
        sys.exit(result.stderr[-2000:])


def main():
    if not shutil.which("ffmpeg"):
        sys.exit("ffmpeg is needed")
    total = TIMING["total"]
    offset = TIMING["musicOffset"]
    with tempfile.TemporaryDirectory() as tmp:
        synth = {}
        for name, (source, chain) in SYNTH.items():
            path = Path(tmp) / f"{name}.wav"
            run(["ffmpeg", "-y", "-f", "lavfi", "-i", source, "-af", chain, "-ar", "48000", "-ac", "2", str(path)])
            synth[name] = path

        inputs = ["-i", str(MUSIC)]
        parts = []
        # Music: comes in under the cold open, lands its drop on "Plainly.", and stops on the
        # downbeat that ends its phrase (TIMING["musicEnd"]), easing out over the ring-out.
        ms = round(offset * 1000)
        stop = TIMING["musicEnd"] - offset
        parts.append(
            f"[0:a]atrim=0:{stop + 1.8:.3f},aresample=48000,aformat=channel_layouts=stereo,"
            f"afade=t=in:d=2.5,afade=t=out:st={stop + 0.12:.3f}:d=1.6:curve=qsin,adelay={ms}|{ms},volume=-2dB[music]"
        )

        # Narration lines on one bus: low cut, a little presence, de-essed, gently compressed.
        vo = []
        for i, (line, start) in enumerate(TIMING["lines"].items(), start=1):
            inputs += ["-i", str(SRC / "vo" / f"{line}.wav")]
            d = round(start * 1000)
            parts.append(f"[{i}:a]aresample=48000,aformat=channel_layouts=stereo,adelay={d}|{d}[v{i}]")
            vo.append(f"[v{i}]")
        n = len(vo)
        parts.append(
            f"{''.join(vo)}amix=inputs={n}:normalize=0,highpass=f=80,"
            "equalizer=f=220:t=q:w=1.2:g=-1.5,equalizer=f=3800:t=q:w=1.4:g=2,deesser=i=0.35,"
            "acompressor=threshold=-21dB:ratio=3:attack=6:release=140:makeup=3dB,volume=2dB,"
            # Padded to full length: the ducking stops when its key does, which cut the music.
            f"apad=whole_dur={total},asplit=3[vo][key][vostem]"
        )

        # Effects, in one small shared room, without their harsh top.
        sfx = []
        for k, (t, name, gain) in enumerate(CUES):
            idx = n + 1 + k
            inputs += ["-i", str(sound_file(name, synth))]
            d = round(t * 1000)
            parts.append(f"[{idx}:a]aresample=48000,aformat=channel_layouts=stereo,adelay={d}|{d},volume={gain - 2}dB[c{k}]")
            sfx.append(f"[c{k}]")
        parts.append(f"{''.join(sfx)}amix=inputs={len(sfx)}:normalize=0,lowpass=f=9500,aecho=0.85:0.6:32|64:0.16|0.09,asplit=2[sfx][sfxstem]")

        # The music ducks whenever the narrator speaks.
        parts.append("[music][key]sidechaincompress=threshold=0.035:ratio=5:attack=25:release=480:makeup=1,asplit=2[duck][musicstem]")
        parts.append(
            f"[duck][vo][sfx]amix=inputs=3:normalize=0,apad,atrim=0:{total},"
            "alimiter=limit=0.9,loudnorm=I=-14:TP=-1.5:LRA=11,aresample=48000[out]"
        )

        OUT.parent.mkdir(parents=True, exist_ok=True)
        script = Path(tmp) / "graph.txt"
        script.write_text(";".join(parts), encoding="utf-8")
        # Stems for editing, before the master limiter: they add up to the mix.
        STEMS.mkdir(parents=True, exist_ok=True)
        pad = f"apad,atrim=0:{total}"
        parts.append(f"[vostem]{pad}[s1];[musicstem]{pad}[s2];[sfxstem]{pad}[s3]")
        script.write_text(";".join(parts), encoding="utf-8")
        run([
            "ffmpeg", "-y", *inputs, "-/filter_complex", str(script),
            "-map", "[out]", "-ar", "48000", "-ac", "2", str(OUT),
            "-map", "[s1]", "-ar", "48000", str(STEMS / "narration.wav"),
            "-map", "[s2]", "-ar", "48000", str(STEMS / "music-ducked.wav"),
            "-map", "[s3]", "-ar", "48000", str(STEMS / "effects.wav"),
        ])
    print(f"wrote {OUT} and stems in {STEMS}")


if __name__ == "__main__":
    main()
