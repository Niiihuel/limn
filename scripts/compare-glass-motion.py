#!/usr/bin/env python3
"""Place deterministic limn frames beside the supplied 30 fps reference video."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("video", type=Path)
parser.add_argument("frames", type=Path, help="glass_qa --motion output directory")
parser.add_argument("output", type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)


def ffmpeg(*arguments):
    subprocess.run(["ffmpeg", "-y", "-v", "error", *map(str, arguments)], check=True)


# Each grid reads left to right, then top to bottom, at 1/30 second intervals.
# Crops retain aspect ratio. Padding, rather than stretching, fits the cells.
cases = [
    ("menu", 22.8, 15, 2, "crop=400:700:180:180", "crop=360:350:0:90", 200, 350, 5, 3),
    ("tabs", 35.25, 21, 7, "crop=588:220:0:1054", "crop=380:100:80:90", 294, 110, 7, 3),
    ("tabs-return", 44.8, 21, 0, "crop=588:220:0:1054", "crop=380:100:80:90", 294, 110, 7, 3),
    ("collapse", 36.6, 30, 8, "crop=588:280:0:994", "crop=470:180:80:90", 294, 140, 6, 5),
    ("sheet", 13.4, 21, 11, "null", "null", 147, 318, 7, 3),
]
for name, start, count, lead, reference_crop, own_crop, width, height, columns, rows in cases:
    def filters(crop, label):
        return (f"{crop},scale={width}:{height}:force_original_aspect_ratio=decrease,"
                f"pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,"
                f"tile={columns}x{rows},pad=iw:ih+28:0:28,"
                f"drawtext=text='{label} - 30 fps':x=8:y=6:fontsize=16:fontcolor=white")
    reference = args.output / f"{name}-reference.png"
    own = args.output / f"{name}-limn.png"
    ffmpeg("-ss", start, "-i", args.video, "-frames:v", 1,
           "-vf", f"trim=end_frame={count}," + filters(reference_crop, "Video reference"), reference)
    ffmpeg("-framerate", 30, "-i", args.frames / name / "%03d.png", "-frames:v", 1,
           "-vf", f"tpad=start={lead}:start_mode=clone,trim=end_frame={count}," + filters(own_crop, "limn"), own)
    ffmpeg("-i", reference, "-i", own, "-filter_complex", "hstack=inputs=2",
           "-frames:v", 1, args.output / f"{name}-comparison.png")

# The supplied video contains no switch interaction, so this is deliberately
# labeled as implementation evidence, not a comparison to an invented reference.
for name, count, columns, rows in [("switch", 21, 7, 3), ("switch-drag", 24, 8, 3),
                                  ("menu-close", 21, 7, 3), ("sheet-close", 21, 7, 3)]:
    crop = "crop=100:70:85:90" if name.startswith("switch") else "null"
    width, height = (200, 140) if name.startswith("switch") else (147, 318)
    vf = (f"trim=end_frame={count},{crop},scale={width}:{height}:force_original_aspect_ratio=decrease,"
          f"pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,tile={columns}x{rows},"
          "pad=iw:ih+28:0:28,drawtext=text='limn implementation - no matching video interval':"
          "x=8:y=6:fontsize=16:fontcolor=white")
    ffmpeg("-framerate", 30, "-i", args.frames / name / "%03d.png", "-vf", vf,
           "-frames:v", 1, args.output / f"{name}-sequence.png")
