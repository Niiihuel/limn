use super::{caption, section, Cx, Section};
use crate::app::State;
use limn::{Component, RasterImage, RasterImageFormat, Video};
use std::sync::{Arc, OnceLock};

/// The clip's first frame; the clip itself is played only where
/// `limn-video` builds, which the browser is not.
const POSTER: &[u8] = include_bytes!("../../../limn-video/assets/pattern.png");
#[cfg(not(target_arch = "wasm32"))]
const CLIP: &[u8] = include_bytes!("../../../limn-video/assets/pattern.mp4");

/// The videos on the page: the one that plays by itself, and the one that
/// waits on its poster, as every clip does under reduced motion.
const PLAYING: &str = "demo-video";
const WAITING: &str = "demo-video-paused";

fn poster() -> RasterImage {
    static POSTER_IMAGE: OnceLock<RasterImage> = OnceLock::new();
    POSTER_IMAGE
        .get_or_init(|| {
            use base64::Engine as _;
            use sha2::Digest as _;
            RasterImage::new(
                format!("{:x}", sha2::Sha256::digest(POSTER)),
                RasterImageFormat::Png,
                POSTER.len() as u32,
                Arc::from(base64::engine::general_purpose::STANDARD.encode(POSTER)),
                "A test pattern",
            )
            .expect("the poster is a small PNG")
        })
        .clone()
}

/// Whether `id` plays: the first one unless the person asked for less
/// motion, each until its button is clicked, which flips the flag of the
/// button's ID.
fn playing(state: &State, id: &str) -> bool {
    let toggled = state.flag(&Video::toggle_id(id));
    let autoplay = id == PLAYING && !state.flag("tokens-reduced");
    autoplay != toggled
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let width = cx.width.min(480.0);
    let video = |id: &str, label: &str| Video::new(id, label, playing(cx.state, id), cx.palette).poster(poster()).width(width);
    vec![
        section(cx, "Video", vec![
            video(PLAYING, "A test pattern that loops").render(),
            caption(cx, "Muted, looping, playing as it comes into view. The button shows under the pointer or the focus."),
        ]),
        section(cx, "Waiting on its poster", vec![
            video(WAITING, "A test pattern, paused").render(),
            caption(cx, "As under reduced motion: nothing is decoded until the play button is pressed."),
        ]),
    ]
}

/// Play the page's videos on the frame just laid out: each presents its due
/// frame and asks for the next repaint only while it plays in view.
#[cfg(not(target_arch = "wasm32"))]
pub fn present_videos(ctx: &eframe::egui::Context, frame: &limn::Frame, state: &State) {
    use std::cell::RefCell;
    thread_local! {
        static PLAYERS: RefCell<Vec<limn_video::VideoPlayer>> = const { RefCell::new(Vec::new()) };
    }
    PLAYERS.with(|players| {
        let mut players = players.borrow_mut();
        if players.is_empty() {
            let clip = limn_video::Clip::from_mp4(CLIP).expect("the sample clip is an MP4");
            for id in [PLAYING, WAITING] {
                players.push(limn_video::VideoPlayer::new(id, clip.clone(), playing(state, id)));
            }
        }
        for player in players.iter_mut() {
            player.set_playing(playing(state, player.id()));
            player.update(ctx, frame);
        }
    });
}
