# Video

A short muted clip that plays in place and loops, such as a feature shown in
a "what's new" dialog, with a round play and pause button over its
bottom-right corner. limn draws it and reports the button; the host decodes
the clip and presents its frames. For a `limn-egui` host the `limn-video`
crate does that part, with Cisco's OpenH264 library, which the host fetches
from Cisco onto the person's machine: see [The codec](#the-codec).

## Import

```rust
use limn::{Component, Video};
// The host's side, in a limn-egui window:
use limn_video::{cisco, Clip, Codec, Status, VideoPlayer};
```

## Usage

The video is controlled: the host passes whether it plays on every rebuild
and flips it when the button reports `Video::toggle_id(id)`. A
`VideoPlayer` keeps that state, so the host answers the button and presents
frames with two calls.

```rust
// Once, away from the UI thread. The codec is Cisco's library, fetched and
// checked by the host, or None while the person has not allowed it.
let clip = Clip::from_mp4(&bytes)?;
let codec = codec_path.and_then(|path| Codec::load(path).ok());
// Under reduced motion it waits on its poster.
let mut player = VideoPlayer::new("whats-new:clip", clip, !reduced_motion, codec);

// The view. Without a codec nothing can play: the poster stands alone.
Video::new("whats-new:clip", "The new sidebar", player.is_playing(), palette)
    .poster(poster)
    .controls(player.status() != Status::NoCodec)
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

## Without controls

```rust
video.controls(false)
```

No play and pause button: the video is its poster, and whatever frames the
host presents. A host that cannot play the clip, as without a codec, shows
it so rather than with a button that does nothing.

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
paused: `VideoPlayer::new(id, clip, !reduced_motion, codec)` never decodes
a clip that is not played.

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

`player.status()` says where it stands: `NoCodec` (it never decodes),
`Waiting` (it has not played in view yet), `Decoding`, or `Stopped` (the
codec or the clip failed; it holds its last frame or the poster).

## The codec

`limn-video` compiles in nothing of OpenH264. It loads Cisco's own build at
run time, the way Firefox does, because Cisco pays the H.264 patent
royalties only for the binaries it distributes, under conditions the host
must meet. `limn-video` does no downloading, no caching and no settings: the
host does, and passes the path of the library to `Codec::load`.

`cisco::VERSION` is the release the bindings load, 2.6.0.
`cisco::current()` (or `cisco::release_for(os, arch)`) gives the host the
file to fetch for this platform:

| Platform | `file_name` (decompressed) | SHA-256 of the decompressed file |
| --- | --- | --- |
| Linux x86-64 | `libopenh264-2.6.0-linux64.8.so` | `2f0cde7c6a6abcf5cae76942894ea42897fa677bce4ed6c91a24dd1b041d5f04` |
| Linux Arm64 | `libopenh264-2.6.0-linux-arm64.8.so` | `12e7b33623667cdab0e575170c147b1b36eadb77d0d2aa7ceb5afd3e58902140` |
| Windows x86-64 | `openh264-2.6.0-win64.dll` | `2076cb5675ec6c1a4c70e7a2a322552f547b6eeed649d6dfcd9e02a543b24691` |
| macOS x86-64 | `libopenh264-2.6.0-mac-x64.dylib` | `e3dc8bc01fe69363f61fd3c02fd27798537a585eadd38cd808f303d1ee505a19` |
| macOS Arm64 | `libopenh264-2.6.0-mac-arm64.dylib` | `052e98bfcf7a9167d22f3bbb3f5988ef79065591f36af8b52924b22b13624551` |

`url` is `https://ciscobinary.openh264.org/<file_name>.bz2`. Cisco publishes
an MD5 beside each file, as `<file_name>.signed.md5.txt`, not a SHA-256: the
SHA-256s above were computed from Cisco's downloads, which match those MD5s,
and are the ones the `openh264` crate accepts. The host:

1. asks the person, and fetches nothing until they agree;
2. downloads `url` from Cisco onto their machine, decompresses it (bzip2),
   checks its SHA-256, and keeps it in the application's data directory;
3. calls `Codec::load(path)` away from the UI thread, which checks the
   SHA-256 again, against the releases the bindings load, before loading it.

`Codec::load` refuses any other file. Each `VideoPlayer` loads the library
again on its own decoding thread. On Linux, Cisco's library needs the
system's `libstdc++.so.6`, present on desktop distributions; where it is
missing, `Codec::load` fails and the video keeps its poster.

### Cisco's conditions

Cisco's binary license (`cisco::LICENSE_URL`,
https://www.openh264.org/BINARY_LICENSE.txt) covers the H.264 patents only
if the application meets these conditions. They bind the host, not limn:

- The library is downloaded from Cisco onto the person's device, separately
  from the application: never bundled with it, mirrored or sent along with
  an update.
- The person can enable, disable and enable again the use of the library,
  such as with a switch in the settings.
- Where that switch is, the application shows `cisco::NOTICE`:
  "OpenH264 Video Codec provided by Cisco Systems, Inc."
- The application reproduces the text of Cisco's binary license, including
  those conditions, where it presents licensing information, such as an
  "About" or "Licenses" page.

Cisco's FAQ adds that every past release stays available for download, so a
host can stay on 2.6.0 until `limn-video` moves to another.

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

- Cisco builds the library for Linux, Windows and macOS on x86-64 and
  Arm64, except Windows on Arm, which this crate does not list. Elsewhere,
  and in the browser, where `limn-video` does not build, the video shows its
  poster.
- Until the person agrees to the download, and while it runs, there is no
  codec: the video shows its poster.
- There is no sound, seeking or progress bar: a clip plays, pauses and loops.
