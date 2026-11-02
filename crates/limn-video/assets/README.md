# Sample clip

`pattern.mp4` is 2 s of FFmpeg's synthesized `testsrc2` pattern, 320 × 180 at
30 frames a second, H.264 Constrained Baseline; `pattern.png` is its first
frame, the poster. The tests and the gallery's Video page play them. They
were made with:

```sh
ffmpeg -f lavfi -i "testsrc2=size=320x180:rate=30:duration=2" -an \
  -vf format=yuv420p -c:v libx264 -profile:v baseline -level:v 3.1 \
  -preset slow -crf 30 -g 60 -bitexact -map_metadata -1 -fflags +bitexact \
  -flags:v +bitexact -movflags +faststart pattern.mp4
ffmpeg -i pattern.mp4 -frames:v 1 -bitexact pattern.png
```
