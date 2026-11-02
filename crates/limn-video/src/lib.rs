//! Plays a short muted clip on a [`limn::Video`] in a `limn-egui` window.
//!
//! A [`Clip`] is an MP4 file with one H.264 video track, read into memory.
//! A [`Codec`] is Cisco's OpenH264 library, which the host fetches from Cisco
//! onto the person's machine (see [`cisco`]); nothing of OpenH264 is built
//! into this crate. A [`VideoPlayer`] decodes the clip with it on a thread of
//! its own, a few frames ahead,
//! loops it without a gap, and presents each frame on the video's surface
//! when it falls due. It decodes and asks for repaints only while the video
//! plays and its surface is in view; paused or hidden, it costs nothing.
//! Without a codec it never decodes, and the video shows its poster.
//!
//! ```no_run
//! # fn view(playing: bool) -> limn::Element { limn::Element::spacer() }
//! # let ctx = egui::Context::default();
//! # let mut tree = limn::UiTree::new(limn::Element::spacer());
//! # let bytes: Vec<u8> = Vec::new();
//! # let reduced_motion = false;
//! # let codec_path: Option<std::path::PathBuf> = None;
//! let clip = limn_video::Clip::from_mp4(&bytes).expect("a clip");
//! // The library the host fetched from Cisco and checked, if the person allowed it.
//! let codec = codec_path.and_then(|path| limn_video::Codec::load(path).ok());
//! // Under reduced motion it waits on its poster for the person to press play.
//! let mut player = limn_video::VideoPlayer::new("whats-new:clip", clip, !reduced_motion, codec);
//! // Each frame: lay the tree out, let the player answer its button, present.
//! tree.set_root(view(player.is_playing()));
//! let frame = tree.layout(limn::Size::new(800.0, 600.0));
//! # let actions: Vec<limn::UiAction> = Vec::new();
//! for action in &actions {
//!     player.handle(action);
//! }
//! player.update(&ctx, &frame);
//! ```
//!
//! Clips must be H.264 Constrained Baseline: no B-frames, as OpenH264
//! decodes. See `docs/components/video.md` for the encoding recipe.

#![warn(missing_docs)]

pub mod cisco;

use limn_egui::SurfaceFrame;
use openh264::decoder::{Decoder, DecoderConfig};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How many decoded frames wait for their turn: enough to cover a slow
/// frame, few enough that a 720p clip holds about 11 MB.
const QUEUE: usize = 3;
/// How late a frame may be before the clock gives up keeping pace and goes
/// on from it: later than this, frames are no longer skipped to catch up.
const STALL: Duration = Duration::from_millis(250);
/// The largest frame a surface takes, as `limn_egui::SurfaceFrame` checks.
const MAX_SIDE: u16 = 8192;
const MAX_PIXELS: u32 = 16 * 1024 * 1024;

/// Why a file is not a clip this crate plays.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClipError {
    /// The MP4 container could not be read.
    Unreadable(String),
    /// There is no H.264 video track.
    NoVideo,
    /// The track is empty, too large, has no timing, or does not start on a
    /// key frame.
    Unsupported(&'static str),
}

impl fmt::Display for ClipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(reason) => write!(formatter, "the MP4 could not be read: {reason}"),
            Self::NoVideo => formatter.write_str("the MP4 has no H.264 video track"),
            Self::Unsupported(reason) => write!(formatter, "the clip is not supported: {reason}"),
        }
    }
}

impl std::error::Error for ClipError {}

/// Why a file is not a codec this crate loads.
#[derive(Debug)]
pub struct CodecError(String);

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the OpenH264 library could not be loaded: {}", self.0)
    }
}

impl std::error::Error for CodecError {}

/// Cisco's OpenH264 library, loaded from a file the host fetched from Cisco
/// and checked; see [`cisco`]. Cloning it shares the path: each player
/// loads the library again on its decoding thread, checking it again.
#[derive(Clone, Debug)]
pub struct Codec {
    path: Arc<Path>,
}

impl Codec {
    /// Load the library at `path` and make a decoder with it, to be sure it
    /// works. A file whose SHA-256 is not one of a release the `openh264`
    /// crate's bindings load ([`cisco::VERSION`]) is refused before it is
    /// loaded. It reads the file: call it away from the UI thread.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, CodecError> {
        let codec = Self { path: Arc::from(path.into()) };
        codec.decoder()?;
        Ok(codec)
    }

    /// The library's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn decoder(&self) -> Result<Decoder, CodecError> {
        let api = OpenH264API::from_blob_path(&*self.path).map_err(|error| CodecError(error.to_string()))?;
        Decoder::with_api_config(api, DecoderConfig::new()).map_err(|error| CodecError(error.to_string()))
    }
}

/// One sample of the track, as start-coded NAL units OpenH264 reads, and
/// when it is shown from the start of the clip.
struct Sample {
    data: Vec<u8>,
    at: Duration,
}

struct Track {
    samples: Vec<Sample>,
    /// One pass through the clip: where the next lap starts.
    length: Duration,
    size: [usize; 2],
}

/// An MP4 file's H.264 track, read into memory and checked. Cloning it
/// shares the samples.
#[derive(Clone)]
pub struct Clip(Arc<Track>);

impl fmt::Debug for Clip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Clip")
            .field("size", &self.0.size)
            .field("frames", &self.0.samples.len())
            .field("length", &self.0.length)
            .finish()
    }
}

impl Clip {
    /// Read the first H.264 track of an MP4 file. Nothing is decoded yet.
    pub fn from_mp4(bytes: &[u8]) -> Result<Self, ClipError> {
        let unreadable = |error: mp4::Error| ClipError::Unreadable(error.to_string());
        let mut reader = mp4::Mp4Reader::read_header(std::io::Cursor::new(bytes), bytes.len() as u64).map_err(unreadable)?;
        let track = reader
            .tracks()
            .values()
            .filter(|track| matches!(track.media_type(), Ok(mp4::MediaType::H264)))
            .min_by_key(|track| track.track_id())
            .ok_or(ClipError::NoVideo)?;
        let (width, height) = (track.width(), track.height());
        if !(1..=MAX_SIDE).contains(&width) || !(1..=MAX_SIDE).contains(&height) || u32::from(width) * u32::from(height) > MAX_PIXELS {
            return Err(ClipError::Unsupported("each side must be 1 to 8192 pixels, and the frame at most 16 megapixels"));
        }
        let timescale = u64::from(track.timescale());
        if timescale == 0 {
            return Err(ClipError::Unsupported("the track has no timescale"));
        }
        let avcc = track.trak.mdia.minf.stbl.stsd.avc1.as_ref().map(|avc1| &avc1.avcc).ok_or(ClipError::NoVideo)?;
        let length_size = usize::from(avcc.length_size_minus_one & 3) + 1;
        // Parameter sets go before every key frame, so a lap can start over from one.
        let mut parameters = Vec::new();
        for nal in avcc.sequence_parameter_sets.iter().chain(&avcc.picture_parameter_sets) {
            parameters.extend_from_slice(&[0, 0, 0, 1]);
            parameters.extend_from_slice(&nal.bytes);
        }
        let (id, count) = (track.track_id(), track.sample_count());
        let time = |units: u64| Duration::from_nanos((u128::from(units) * 1_000_000_000 / u128::from(timescale)) as u64);
        let mut samples = Vec::with_capacity(count as usize);
        let mut end = 0;
        for index in 1..=count {
            let Some(sample) = reader.read_sample(id, index).map_err(unreadable)? else { continue };
            if samples.is_empty() && !sample.is_sync {
                return Err(ClipError::Unsupported("the first frame is not a key frame"));
            }
            let mut data = if sample.is_sync { parameters.clone() } else { Vec::new() };
            let mut rest: &[u8] = &sample.bytes;
            while !rest.is_empty() {
                let (length, after) = rest.split_at_checked(length_size).ok_or(ClipError::Unsupported("a sample ends inside a length"))?;
                let length = length.iter().fold(0usize, |sum, byte| sum << 8 | usize::from(*byte));
                let (nal, after) = after.split_at_checked(length).ok_or(ClipError::Unsupported("a NAL unit runs past its sample"))?;
                data.extend_from_slice(&[0, 0, 0, 1]);
                data.extend_from_slice(nal);
                rest = after;
            }
            end = end.max(sample.start_time + u64::from(sample.duration));
            samples.push(Sample { data, at: time(sample.start_time) });
        }
        let length = time(end);
        if samples.is_empty() || length.is_zero() {
            return Err(ClipError::Unsupported("the track has no frames or no duration"));
        }
        Ok(Self(Arc::new(Track { samples, length, size: [usize::from(width), usize::from(height)] })))
    }

    /// Width and height in pixels, as the container says.
    pub fn size(&self) -> [usize; 2] {
        self.0.size
    }

    /// How many frames one lap shows.
    pub fn frame_count(&self) -> usize {
        self.0.samples.len()
    }

    /// How long one lap lasts.
    pub fn duration(&self) -> Duration {
        self.0.length
    }

    /// The time between two frames on average: how soon to look again for a
    /// frame the decoder has not delivered yet.
    fn interval(&self) -> Duration {
        self.0.length / self.0.samples.len() as u32
    }
}

/// Decode one lap of `track`, handing each picture to `frame` as its
/// sample's index, its size and its pixels in RGB. Stops early, with
/// `false`, when `frame` returns `false`. A sample that does not decode is
/// skipped; `None` when no sample decodes at all.
fn decode_lap(
    decoder: &mut Decoder,
    track: &Track,
    rgb: &mut Vec<u8>,
    mut frame: impl FnMut(usize, [usize; 2], &[u8]) -> bool,
) -> Option<bool> {
    let mut any = false;
    for (index, sample) in track.samples.iter().enumerate() {
        let Ok(Some(picture)) = decoder.decode(&sample.data) else { continue };
        let (width, height) = picture.dimensions();
        rgb.resize(width * height * 3, 0);
        picture.write_rgb8(rgb);
        any = true;
        if !frame(index, [width, height], rgb) {
            return Some(false);
        }
    }
    any.then_some(true)
}

/// A frame decoded ahead: which sample it is, and when it is due counted
/// from the first lap's start.
struct Decoded {
    index: usize,
    at: Duration,
    frame: SurfaceFrame,
}

/// Decode lap after lap into `frames` until the player hangs up or the clip
/// stops decoding. The queue being full is what holds it back.
fn decode(track: Arc<Track>, codec: Codec, frames: SyncSender<Decoded>) {
    let Ok(mut decoder) = codec.decoder() else { return };
    let mut rgb = Vec::new();
    for lap in 0u32.. {
        let start = track.length * lap;
        let more = decode_lap(&mut decoder, &track, &mut rgb, |index, size, pixels| {
            // A cover or a contain fit ignores the scale.
            let Some(frame) = SurfaceFrame::from_rgb(size, pixels, 1.0) else { return false };
            frames.send(Decoded { index, at: start + track.samples[index].at, frame }).is_ok()
        });
        if more != Some(true) {
            return;
        }
    }
}

/// What one [`VideoPlayer::tick`] did: the frame to present, if one fell
/// due, and how soon to look again, `None` while there is nothing to wait
/// for (paused, hidden, or the clip stopped decoding).
#[derive(Debug, Default)]
pub struct Tick {
    /// The latest frame due, with its index in the clip.
    pub frame: Option<(usize, SurfaceFrame)>,
    /// When the next frame is due, or when to look again for it.
    pub repaint_after: Option<Duration>,
}

/// What a [`VideoPlayer`] is doing about decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// It has no [`Codec`]: it never decodes, and the video shows its
    /// poster. Pass the `Video` `controls(false)` then, as nothing can play.
    NoCodec,
    /// It has not played in view yet, so it has decoded nothing.
    Waiting,
    /// Its decoding thread runs, or waits for room in the queue.
    Decoding,
    /// The codec or the clip failed while decoding: it holds its last frame,
    /// or the poster.
    Stopped,
}

/// Plays a [`Clip`] on the surface of a [`limn::Video`] with the same ID:
/// decodes it on a thread of its own, loops it, and presents each frame as
/// it falls due. Its clock runs only while it plays and the surface is in
/// view; the decoding thread starts the first time it does, so a clip that
/// never plays, as under reduced motion, is never decoded, and nor is one
/// without a [`Codec`]. Dropping the
/// player stops the thread and frees the surface's texture.
pub struct VideoPlayer {
    id: String,
    clip: Clip,
    playing: bool,
    codec: Option<Codec>,
    stopped: bool,
    frames: Option<Receiver<Decoded>>,
    /// The next frame, taken from the queue before it is due.
    next: Option<Decoded>,
    /// Where the clock stood when it last stopped.
    position: Duration,
    /// When the clock last started, while it runs.
    resumed: Option<Instant>,
}

impl fmt::Debug for VideoPlayer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VideoPlayer")
            .field("id", &self.id)
            .field("clip", &self.clip)
            .field("playing", &self.playing)
            .field("position", &self.position)
            .finish()
    }
}

impl VideoPlayer {
    /// A player for the video `id`, playing from the start once it is in
    /// view if `playing`, or waiting to be played. Without a `codec` it
    /// never plays: the video shows its poster.
    pub fn new(id: impl Into<String>, clip: Clip, playing: bool, codec: Option<Codec>) -> Self {
        Self {
            id: id.into(),
            clip,
            playing,
            codec,
            stopped: false,
            frames: None,
            next: None,
            position: Duration::ZERO,
            resumed: None,
        }
    }

    /// The surface it presents on: the [`limn::Video`]'s ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Whether it plays: what the [`limn::Video`] is built with. Never
    /// without a codec.
    pub fn is_playing(&self) -> bool {
        self.playing && self.codec.is_some()
    }

    /// Play or pause. A paused clip holds its frame and resumes from it.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    /// What it is doing about decoding: nothing before the clip first
    /// plays in view, nor ever without a codec.
    pub fn status(&self) -> Status {
        match (&self.codec, &self.frames) {
            (None, _) => Status::NoCodec,
            (Some(_), None) => Status::Waiting,
            (Some(_), Some(_)) if self.stopped => Status::Stopped,
            (Some(_), Some(_)) => Status::Decoding,
        }
    }

    /// Answer an action of the tree: a click on the video's play and pause
    /// button, [`limn::Video::toggle_id`], flips it. `true` when it was one.
    pub fn handle(&mut self, action: &limn::UiAction) -> bool {
        let toggled = matches!(action, limn::UiAction::Clicked { id } if *id == limn::Video::toggle_id(&self.id));
        if toggled {
            self.playing = !self.playing;
        }
        toggled
    }

    /// Advance the clock to `now`, if it plays and is `shown`, and take the
    /// frame that is due. The clock stops while it is paused or hidden, and
    /// while the decoder has nothing ready, so no frame is skipped for want
    /// of one; it resumes where it stopped. Frames a little late are skipped
    /// to keep pace; after a longer gap between ticks (over 250 ms) it goes on
    /// from the frame it stopped at.
    pub fn tick(&mut self, now: Instant, shown: bool) -> Tick {
        let Some(codec) = self.codec.as_ref().filter(|_| self.playing && shown && !self.stopped) else {
            self.stop_clock(now);
            return Tick::default();
        };
        let frames = self.frames.get_or_insert_with(|| {
            let (sender, receiver) = mpsc::sync_channel(QUEUE);
            let (track, codec) = (self.clip.0.clone(), codec.clone());
            // The thread ends when the receiver is dropped, at its next frame.
            // A thread that cannot start drops the sender: the player then holds its poster.
            std::thread::Builder::new().name("limn-video".into()).spawn(move || decode(track, codec, sender)).ok();
            receiver
        });
        let mut clock = self.position + self.resumed.map_or(Duration::ZERO, |at| now.saturating_duration_since(at));
        self.resumed.get_or_insert(now);
        let mut due = None;
        loop {
            if self.next.is_none() {
                match frames.try_recv() {
                    Ok(frame) => self.next = Some(frame),
                    Err(TryRecvError::Empty) => break,
                    // The clip stopped decoding: hold the last frame.
                    Err(TryRecvError::Disconnected) => {
                        self.stopped = true;
                        self.stop_clock(now);
                        return Tick { frame: due.map(|due: Decoded| (due.index, due.frame)), repaint_after: None };
                    }
                }
            }
            // A window that was not repainted for a while, as a minimized one, goes on
            // from the frame it stopped at rather than racing to catch up.
            if let Some(next) = self.next.as_ref().filter(|next| clock > next.at + STALL) {
                (self.position, self.resumed, clock) = (next.at, Some(now), next.at);
            }
            if self.next.as_ref().is_some_and(|next| next.at <= clock) {
                due = self.next.take();
            } else {
                break;
            }
        }
        let repaint_after = match &self.next {
            Some(next) => next.at - clock,
            None => {
                self.stop_clock(now);
                self.clip.interval()
            }
        };
        Tick { frame: due.map(|due| (due.index, due.frame)), repaint_after: Some(repaint_after) }
    }

    fn stop_clock(&mut self, now: Instant) {
        if let Some(at) = self.resumed.take() {
            self.position += now.saturating_duration_since(at);
        }
    }

    /// Each frame, after the layout: present the frame due if the video
    /// plays and its surface is in `frame` ([`limn::Frame::surface_shown`])
    /// in a window that is not minimized, and ask `ctx` for a repaint when
    /// the next one falls due. Paused or hidden, it asks for nothing.
    pub fn update(&mut self, ctx: &egui::Context, frame: &limn::Frame) {
        let minimized = ctx.input(|input| input.viewport().minimized == Some(true));
        let tick = self.tick(Instant::now(), !minimized && frame.surface_shown(&self.id));
        if let Some((_, picture)) = tick.frame {
            limn_egui::present_surface(ctx, &self.id, picture);
        }
        if let Some(after) = tick.repaint_after {
            ctx.request_repaint_after(after);
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        limn_egui::forget_surface(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limn::Component as _;

    /// 2 s of a synthesized test pattern, 320×180 at 30 frames a second;
    /// see `assets/README.md`.
    const PATTERN: &[u8] = include_bytes!("../assets/pattern.mp4");

    /// The path of a Cisco OpenH264 library to decode with; tests that decode
    /// are skipped without it. `downloads_ciscos_library_and_decodes_the_pattern`
    /// fetches one and prints its path.
    const CODEC_VARIABLE: &str = "LIMN_VIDEO_OPENH264";

    fn clip() -> Clip {
        Clip::from_mp4(PATTERN).expect("the pattern is a clip")
    }

    fn codec() -> Option<Codec> {
        let Some(path) = std::env::var_os(CODEC_VARIABLE) else {
            eprintln!("skipped: set {CODEC_VARIABLE} to the path of Cisco's OpenH264 {}", cisco::VERSION);
            return None;
        };
        Some(Codec::load(path).expect("the library in LIMN_VIDEO_OPENH264 loads"))
    }

    /// A codec whose library is missing: it loads nothing, and fails only
    /// once a player starts decoding with it.
    fn missing_codec() -> Codec {
        Codec { path: Arc::from(Path::new("/nonexistent/libopenh264.so")) }
    }

    /// Tick until `count` frames were presented, letting exactly the time
    /// each tick asks for pass before the next, as a window's repaints do.
    fn play(player: &mut VideoPlayer, count: usize) -> Vec<usize> {
        let mut now = Instant::now();
        let mut shown = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(30);
        while shown.len() < count {
            assert!(Instant::now() < deadline, "the decoder delivered {} frames in 30 s", shown.len());
            let tick = player.tick(now, true);
            assert!(tick.repaint_after.is_some(), "a playing clip asks to be looked at again");
            now += tick.repaint_after.unwrap();
            match tick.frame {
                Some((index, frame)) => {
                    assert_eq!(frame.size(), [320, 180]);
                    shown.push(index);
                }
                // The decoder is behind: give it a moment.
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        }
        shown
    }

    /// Decode one lap of the pattern with `codec`, checking each frame.
    fn decode_pattern(codec: &Codec) {
        let mut decoder = codec.decoder().expect("a decoder");
        let mut decoded = Vec::new();
        let finished = decode_lap(&mut decoder, &clip().0, &mut Vec::new(), |index, size, pixels| {
            assert_eq!(pixels.len(), size[0] * size[1] * 3);
            decoded.push((index, size));
            true
        });
        assert_eq!(finished, Some(true));
        assert_eq!(decoded, (0..60).map(|index| (index, [320, 180])).collect::<Vec<_>>());
    }

    #[test]
    fn the_pattern_reads_as_two_seconds_of_320_by_180_at_30_frames() {
        let clip = clip();
        assert_eq!((clip.size(), clip.frame_count(), clip.duration()), ([320, 180], 60, Duration::from_secs(2)));
        assert_eq!(clip.interval(), Duration::from_nanos(33_333_333));
    }

    #[test]
    fn what_is_not_a_clip_is_refused() {
        assert!(matches!(Clip::from_mp4(b"not an mp4"), Err(ClipError::Unreadable(_))));
        assert!(Clip::from_mp4(&PATTERN[..PATTERN.len() / 2]).is_err());
    }

    #[test]
    fn what_is_not_ciscos_library_is_not_loaded() {
        assert!(Codec::load("/nonexistent/libopenh264.so").is_err());
        // A file that exists but is not a known release is refused before it is loaded.
        assert!(Codec::load(file!()).is_err());
    }

    #[test]
    fn without_a_codec_the_poster_stays_and_nothing_is_decoded() {
        let mut player = VideoPlayer::new("clip", clip(), true, None);
        assert_eq!(player.status(), Status::NoCodec);
        assert!(!player.is_playing(), "the video is built paused");
        for step in 0..5 {
            let tick = player.tick(Instant::now() + Duration::from_millis(step * 40), true);
            assert!(tick.frame.is_none() && tick.repaint_after.is_none());
        }
        assert_eq!(player.status(), Status::NoCodec);
    }

    #[test]
    fn a_clip_that_never_plays_is_never_decoded_and_asks_for_nothing() {
        // As under reduced motion: the poster shows until the person presses play.
        let mut player = VideoPlayer::new("clip", clip(), false, Some(missing_codec()));
        let now = Instant::now();
        for step in 0..5 {
            let tick = player.tick(now + Duration::from_millis(step * 40), true);
            assert!(tick.frame.is_none() && tick.repaint_after.is_none());
        }
        assert_eq!(player.status(), Status::Waiting);
    }

    #[test]
    fn a_hidden_clip_is_never_decoded() {
        let mut player = VideoPlayer::new("clip", clip(), true, Some(missing_codec()));
        let tick = player.tick(Instant::now(), false);
        assert!(tick.frame.is_none() && tick.repaint_after.is_none());
        assert_eq!(player.status(), Status::Waiting);
    }

    #[test]
    fn a_codec_that_fails_stops_on_the_poster_and_asks_for_nothing_more() {
        let mut player = VideoPlayer::new("clip", clip(), true, Some(missing_codec()));
        let deadline = Instant::now() + Duration::from_secs(10);
        while player.status() != Status::Stopped {
            assert!(Instant::now() < deadline, "the player never noticed");
            assert!(player.tick(Instant::now(), true).frame.is_none());
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(player.tick(Instant::now(), true).repaint_after.is_none());
    }

    #[test]
    fn the_decoder_decodes_every_frame_of_the_pattern_at_its_size() {
        if let Some(codec) = codec() {
            decode_pattern(&codec);
        }
    }

    #[test]
    fn frames_arrive_in_order_and_loop() {
        let Some(codec) = codec() else { return };
        let mut player = VideoPlayer::new("clip", clip(), true, Some(codec));
        let shown = play(&mut player, 150);
        assert_eq!(shown, (0..150).map(|at| at % 60).collect::<Vec<_>>());
        assert_eq!(player.status(), Status::Decoding);
    }

    #[test]
    fn the_button_pauses_and_resumes_where_it_stopped() {
        let Some(codec) = codec() else { return };
        let mut player = VideoPlayer::new("clip", clip(), true, Some(codec));
        play(&mut player, 10);
        let toggle = limn::UiAction::Clicked { id: limn::Video::toggle_id("clip") };
        assert!(player.handle(&toggle));
        assert!(!player.is_playing());
        assert!(!player.handle(&limn::UiAction::Clicked { id: "other:toggle".into() }));
        // Paused, or playing off screen, a long while passes: nothing is presented or asked for.
        let later = Instant::now() + Duration::from_secs(60);
        for shown in [true, false] {
            let tick = player.tick(later, shown);
            assert!(tick.frame.is_none() && tick.repaint_after.is_none());
        }
        player.handle(&toggle);
        assert!(player.tick(later, false).repaint_after.is_none(), "off screen it waits");
        // Back in view it goes on from the frame after the last one shown.
        assert_eq!(play(&mut player, 3), [10, 11, 12]);
    }

    /// Run `player` in passes of a window until its surface has a texture
    /// or `passes` ran out: whether one did, and the repaint delay egui was
    /// last asked for.
    fn run(player: &mut VideoPlayer, passes: usize) -> (bool, Duration) {
        let ctx = egui::Context::default();
        let video = limn::Video::new("clip", "Pattern", player.is_playing(), limn::ComponentPalette::neutral_dark()).render();
        let frame = limn::UiTree::new(video).layout(limn::Size::new(320.0, 180.0));
        let mut delay = Duration::MAX;
        for _ in 0..passes {
            // The context takes textures and repaint requests inside a pass.
            let mut output = ctx.run_ui(egui::RawInput::default(), |_| player.update(&ctx, &frame));
            // No renderer takes the textures here.
            output.textures_delta.clear();
            delay = output.viewport_output.values().map(|viewport| viewport.repaint_delay).min().unwrap_or(Duration::MAX);
            let textures = ctx.tex_manager();
            if textures.read().allocated().any(|(_, meta)| meta.name == "surface:clip" && meta.size == [320, 180]) {
                return (true, delay);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        (false, delay)
    }

    #[test]
    fn a_frame_reaches_the_surface_and_the_window_repaints_at_the_frame_rate() {
        let Some(codec) = codec() else { return };
        let (presented, delay) = run(&mut VideoPlayer::new("clip", clip(), true, Some(codec)), 2000);
        assert!(presented, "no frame reached the surface in 10 s");
        assert!(delay <= Duration::from_millis(34), "{delay:?}");
    }

    #[test]
    fn a_paused_clip_presents_nothing_and_asks_for_no_repaint() {
        for codec in [None, Some(missing_codec())] {
            let (presented, delay) = run(&mut VideoPlayer::new("clip", clip(), false, codec), 3);
            assert!(!presented);
            assert_eq!(delay, Duration::MAX);
        }
    }

    /// Fetches Cisco's library for this platform with `curl` and `bunzip2`,
    /// as a host would, checks it against [`cisco::current`], and decodes the
    /// pattern with it. Ignored: it downloads. Its path, printed, is what
    /// `LIMN_VIDEO_OPENH264` takes for the other tests.
    #[test]
    #[ignore = "downloads Cisco's OpenH264 library"]
    fn downloads_ciscos_library_and_decodes_the_pattern() {
        use sha2::Digest as _;
        use std::process::Command;
        let release = cisco::current().expect("Cisco has a release for this platform");
        let dir = std::env::temp_dir().join(format!("limn-video-openh264-{}", cisco::VERSION));
        std::fs::create_dir_all(&dir).unwrap();
        let library = dir.join(release.file_name);
        let compressed = dir.join(format!("{}.bz2", release.file_name));
        let fetched = Command::new("curl").args(["-sfSL", "-o"]).arg(&compressed).arg(release.url).status().unwrap();
        assert!(fetched.success(), "curl {}", release.url);
        let unpacked = Command::new("bunzip2").arg("-kf").arg(&compressed).status().unwrap();
        assert!(unpacked.success());
        let sha256: String = sha2::Sha256::digest(std::fs::read(&library).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(sha256, release.sha256, "{} is not the file the table names", library.display());
        decode_pattern(&Codec::load(&library).expect("Cisco's library loads"));
        println!("{CODEC_VARIABLE}={}", library.display());
    }
}
