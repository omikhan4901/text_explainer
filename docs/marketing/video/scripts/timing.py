"""Lays the video out around the narration: each scene lasts as long as its line needs,
plus room to read what is on screen, and from the beat drop on every scene starts on a
beat of the music. Writes work/timing.json for build.mjs and mix.py.

Usage (from the repo root): python docs/marketing/video/scripts/timing.py
"""

import json
import math
from pathlib import Path

VIDEO = Path("docs/marketing/video")
VO = json.loads((VIDEO / "work" / "vo.json").read_text(encoding="utf-8"))
CUES = json.loads((VIDEO / "audio" / "music-cues.json").read_text(encoding="utf-8"))
MUSIC_DROP = 8.7423  # the track's first strong beat, where the bass comes in
RING_OUT = 2.4  # seconds held after the final hit

# (scene, narration lines placed in it as (line, seconds after the scene starts), how to
# end it: seconds after the last line ends, and the shortest the scene may be).
PLAN = [
    ("s1", [("s1", 1.5)], None, 5.6),
    ("s2", [("s2", 0.35)], 1.4, 0),
    ("brand", [], None, None),  # the drop: "Plainly fixes that." (timed below)
    ("desk", [], 1.9, 0),
    ("morph", [("morph", 0.55)], 1.9, 0),
    ("levels", [("levels", 0.45)], 1.6, 6.2),
    ("apps", [("apps", 0.35)], 1.5, 0),
    ("tr", [("tr", 0.45)], 1.7, 0),
    ("priv", [("priv", 0.5)], 1.5, 0),
    ("words", [("words", 0.9)], 1.9, 5.8),
    ("check", [("check", 0.45)], 2.0, 0),
    ("outro", [("outro", 1.1)], 3.2, 0),
]


def main():
    scenes = {}
    lines = {}
    t = 0.0
    beats = None
    music_offset = None

    def snap(x):
        """The first beat at or after x (from the drop on)."""
        if beats is None:
            return x
        return min((b for b in beats if b >= x - 0.08), default=x)

    for scene, placed, tail, minimum in PLAN:
        start = snap(t)
        if scene == "brand":
            # The drop: the music's first strong beat lands here.
            music_offset = start - MUSIC_DROP
            beats = [b["time"] + music_offset for b in CUES["beats"]]
            # "Plainly fixes that." is spoken over the brand slam; the rest over the desk.
            lines["desk"] = start + 0.2
            words = VO["desk"]["words"]
            that = next(w for w in words if w["w"].lower().startswith("that"))
            dur = snap(start + 0.2 + that["end"] + 0.35) - start
            scenes[scene] = {"start": round(start, 3), "dur": round(dur, 3)}
            t = start + dur
            continue
        if scene == "desk":
            desk_end = lines["desk"] + VO["desk"]["duration"]
            lines["card"] = desk_end + 0.5
            end = lines["card"] + VO["card"]["duration"] + tail
            dur = end - start
        else:
            for line, offset in placed:
                lines[line] = start + offset
            last = max((lines[l] + VO[l]["duration"] for l, _ in placed), default=start)
            dur = (last + tail - start) if tail is not None else 0
            dur = max(dur, minimum or 0)
        scenes[scene] = {"start": round(start, 3), "dur": round(dur, 3)}
        t = start + dur

    # End on the music's own phrase: the first 8-bar boundary (32 beats from the drop) after
    # the outro has said its line, then let the last hit ring out.
    drop = min(range(len(beats)), key=lambda i: abs(beats[i] - scenes["brand"]["start"]))
    music_end = next(beats[i] for i in range(drop, len(beats), 32) if beats[i] >= t - 0.4)
    total = round(music_end + RING_OUT, 3)
    scenes["outro"]["dur"] = round(total - scenes["outro"]["start"], 3)

    words = {}
    for line, start in lines.items():
        words[line] = [{"w": w["w"], "t": round(start + w["start"], 3), "end": round(start + w["end"], 3)} for w in VO[line]["words"]]
    out = {
        "total": total,
        "musicEnd": round(music_end, 3),
        "musicOffset": round(music_offset, 3),
        "scenes": scenes,
        "lines": {k: round(v, 3) for k, v in lines.items()},
        "words": words,
        "beats": [round(b, 3) for b in beats if 0 <= b <= total],
    }
    (VIDEO / "work" / "timing.json").write_text(json.dumps(out, indent=1, ensure_ascii=False), encoding="utf-8")
    for k, v in scenes.items():
        print(f"{k:7} {v['start']:6.2f}  {v['dur']:5.2f}")
    print("total", total, "music offset", out["musicOffset"], "music end", out["musicEnd"])


if __name__ == "__main__":
    main()
