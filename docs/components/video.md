# Video

A short muted clip that plays in place and loops, such as a feature shown in
a "what's new" dialog, with a round play and pause button over its
bottom-right corner. limn draws it and reports the button; the host decodes
the clip and presents its frames. For a `limn-egui` host the `limn-video`
crate does that part.

## Import

```rust
use limn::{Component, Video};
// The host's side, in a limn-egui window:
use limn_video::{Clip, VideoPlayer};
```

## Usage

The video is controlled: the host passes whether it plays on every rebuild
and flips it when the button reports `Video::toggle_id(id)`. A
`VideoPlayer` keeps that state, so the host answers the button and presents
frames with two calls.

```rust
// Once, where the clip is loaded. Under reduced motion it waits on its poster.
let clip = Clip::from_mp4(&bytes)?;
let mut player = VideoPlayer::new("whats-new:clip", clip, !reduced_motion);

// The view.
Video::new("whats-new:clip", "The new sidebar", player.is_playing(), palette)
    .poster(poster)
    .width(Modal::body_width(modal_width))
    .render()

// Each frame, after the layout.
for action in tree.handle_event(event) {
    player.handle(&action);
}
let frame = tree.layout(window);
player.update(ctx, &frame);
```

`update` presents the frame that is due on the surface `whats-new:clip` and
asks egui for a repaint when the next one falls due. Dropping the player
stops its decoding thread and frees the surface's texture.

## Poster

```rust
Video::new(id, label, playing, palette).poster(poster)
```

The poster is a still picture under the clip: it shows until the first
frame is presented, while the clip loads, and for as long as it never plays,
as under reduced motion. Make it from the clip's first frame (see
[Clips](#clips)) so nothing jumps when the clip starts. It is not named to
the platform; the video's `label` is.

## Size and corners

```rust
video.width(480.0).radius(12.0)
```

`width(px)` lays the video out that wide and 9/16 of that high. Without it
the video fills the box its parent gives it. The clip and the poster fill
the box and are cropped around the center, as `SurfaceFit::Cover` does, so
a clip of another shape loses its edges rather than leaving bars. `radius(px)` rounds the clip, the poster
and the box, such as to the inner radius of the card it sits in;
`tokens().radius.card` unless set.

## Labels

```rust
video.control_labels("Reproducir video", "Pausar video")
```

The button is named "Play video" while the clip is paused and "Pause video"
while it plays, unless set.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Reaches the play and pause button, and shows it. |
| Enter, Space | Reports `{id}:toggle`. |

## Motion

While the clip plays, the button shows only while the pointer is over the
video or the button has the focus, fading in over `enter_ms` and out over
`exit_ms`; paused, it stays, so a paused clip does not pass for a picture.
Under reduced motion the fade is a switch, and a host should start the clip
paused: `VideoPlayer::new(id, clip, !reduced_motion)` never decodes a clip
that is not played.

## Actions

| Action | When |
| --- | --- |
| `Clicked` with `{id}:toggle` | The button is clicked, or Enter or Space is pressed on it. |

## Playback

`VideoPlayer` decodes on a thread of its own, three frames ahead, and loops
without a gap: it decodes the next lap while the last frames of this one
show. Its clock runs only while the clip plays and its surface is in view,
as `Frame::surface_shown(id)` says: not scrolled out of view, not faded out,
still in the tree, in a window that is not minimized. Paused or hidden, it
neither decodes nor asks for repaints, and it resumes from the frame it
stopped on. The decoding thread starts the first time the clip plays in
view. When the decoder falls behind, the clock waits for it rather than
skipping frames.

`player.tick(now, shown)` is the same step without egui: it returns the
frame due, with its index in the clip, and when to look again.

## Clips

`limn-video` plays MP4 files with one H.264 track in Constrained Baseline
profile, which has no B-frames: the profile OpenH264 decodes. Audio tracks
are ignored. Make release clips at 1280 × 720 at most, at 30 frames a second,
up to 2 Mbit/s (about 7.5 MB for 30 s), and the poster from the clip's first
frame:

```sh
ffmpeg -i input.mov -an \
  -vf "fps=30,scale=1280:720:flags=lanczos,format=yuv420p" \
  -c:v libx264 -profile:v baseline -level:v 3.1 -preset slow -crf 23 \
  -maxrate 2M -bufsize 4M -g 60 -movflags +faststart clip.mp4
ffmpeg -i clip.mp4 -frames:v 1 poster.png
```

`Clip::from_mp4` refuses a file that is not an MP4, has no H.264 track, does
not start on a key frame, or has frames larger than a surface takes (8192
pixels a side, 16 megapixels).

## Limits

- `limn-video` compiles OpenH264 from the source it vendors with the C++
  compiler `cc` finds: MSVC on Windows, Clang on macOS, GCC or Clang on
  Linux. It does not build for the browser, where the video shows its
  poster.
- Without `nasm` on the build machine OpenH264 is built without its x86
  assembly; it still decodes 720p at 30 frames a second on one core.
- Building OpenH264 from source is not covered by Cisco's H.264 patent
  license, which covers only the binaries Cisco distributes. An application
  that ships the decoder answers for its own H.264 licensing.
- There is no sound, seeking or progress bar: a clip plays, pauses and loops.
