"""Writes an editable Premiere Pro timeline (FCP7 XML, which Premiere imports) for the launch
video, from the layer renders and the audio stems in out/editable/:

  V3  reference: the finished video (switched off, to compare against)
  V2  headlines.mov: the headlines and lines under them, transparent
  V1  picture.mp4: everything else, cut at every scene and named after it
  A1  narration, A2 music (ducked under the narration), A3 effects
  markers at every scene start

Usage (from the repo root): python docs/marketing/video/scripts/premiere.py
Then in Premiere Pro: File > Import > out/editable/plainly-launch.xml.
"""

import json
from pathlib import Path
from xml.sax.saxutils import escape

VIDEO = Path("docs/marketing/video").resolve()
ED = VIDEO / "out" / "editable"
TIMING = json.loads((VIDEO / "work" / "timing.json").read_text(encoding="utf-8"))
FPS = 60
TOTAL = round(TIMING["total"] * FPS)
NAMES = {
    "s1": "Cold open", "s2": "Re-read", "brand": "Plainly", "desk": "Select and card", "morph": "Morph",
    "levels": "Levels", "apps": "Every app", "tr": "Translate", "priv": "Private", "words": "Words",
    "check": "Check", "outro": "Outro",
}
RATE = f"<rate><timebase>{FPS}</timebase><ntsc>FALSE</ntsc></rate>"


def url(path):
    return "file://localhost/" + path.as_posix().replace(" ", "%20")


def file_el(fid, path, video, audio, first):
    """A <file> in full the first time it is used, then by reference."""
    if not first:
        return f'<file id="{fid}"/>'
    media = ""
    if video:
        media += "<video><samplecharacteristics><width>1920</width><height>1080</height><pixelaspectratio>square</pixelaspectratio></samplecharacteristics></video>"
    if audio:
        media += "<audio><samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics><channelcount>2</channelcount></audio>"
    return f'<file id="{fid}"><name>{escape(path.name)}</name><pathurl>{url(path)}</pathurl>{RATE}<duration>{TOTAL}</duration><media>{media}</media></file>'


def clip(cid, name, fid, path, start, end, video=True, audio=False, first=True, enabled=True, kind="video"):
    track = f"<sourcetrack><mediatype>{kind}</mediatype><trackindex>1</trackindex></sourcetrack>" if kind == "audio" else ""
    return (
        f'<clipitem id="{cid}"><name>{escape(name)}</name><enabled>{"TRUE" if enabled else "FALSE"}</enabled>'
        f"<duration>{TOTAL}</duration>{RATE}<start>{start}</start><end>{end}</end><in>{start}</in><out>{end}</out>"
        f"{file_el(fid, path, video, audio, first)}{track}</clipitem>"
    )


def main():
    picture = ED / "video" / "picture.mp4"
    headlines = ED / "video" / "headlines.mov"
    master = VIDEO / "out" / "plainly-launch.mp4"
    for p in (picture, headlines, master, *(ED / "audio" / n for n in ("narration.wav", "music-ducked.wav", "effects.wav"))):
        if not p.exists():
            raise SystemExit(f"missing {p}")

    scenes = sorted(TIMING["scenes"].items(), key=lambda kv: kv[1]["start"])
    cuts = [round(s["start"] * FPS) for _, s in scenes] + [TOTAL]
    v1 = "".join(
        clip(f"pic-{i}", f"{i + 1:02d} {NAMES.get(sid, sid)}", "f-picture", picture, cuts[i], cuts[i + 1], first=(i == 0))
        for i, (sid, _) in enumerate(scenes)
    )
    v2 = clip("headlines", "Headlines (transparent)", "f-headlines", headlines, 0, TOTAL)
    v3 = clip("reference", "Reference: finished video", "f-master", master, 0, TOTAL, audio=True, enabled=False)
    audio = "".join(
        f"<track>{clip(f'a-{n}', label, f'f-{n}', ED / 'audio' / n, 0, TOTAL, video=False, audio=True, kind='audio')}</track>"
        for n, label in (("narration.wav", "Narration"), ("music-ducked.wav", "Music (ducked)"), ("effects.wav", "Effects"))
    )
    markers = "".join(
        f"<marker><name>{escape(NAMES.get(sid, sid))}</name><comment></comment><in>{round(s['start'] * FPS)}</in><out>-1</out></marker>"
        for sid, s in scenes
    )
    fmt = "<format><samplecharacteristics>" + RATE + "<width>1920</width><height>1080</height><pixelaspectratio>square</pixelaspectratio><fielddominance>none</fielddominance></samplecharacteristics></format>"
    xml = (
        '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE xmeml>\n<xmeml version="4"><sequence id="plainly-launch">'
        f"<name>Plainly launch</name><duration>{TOTAL}</duration>{RATE}"
        f"<media><video>{fmt}<track>{v1}</track><track>{v2}</track><track>{v3}</track></video>"
        f"<audio><numOutputChannels>2</numOutputChannels>{audio}</audio></media>{markers}</sequence></xmeml>\n"
    )
    out = ED / "plainly-launch.xml"
    out.write_text(xml, encoding="utf-8")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
