use super::{caption, row, section, Cx, Section};
use limn::{
    Attachment, AttachmentOrientation, Avatar, Button, ButtonVariant, Component, Element, Icon, ImageViewer,
    RadiusRole, RasterImage, RasterImageFormat, Size, SurfaceFit,
};
use std::sync::{Arc, OnceLock};
use web_time::{Duration, Instant};

/// The gallery ships no picture files: these are three PNGs a few pixels
/// wide, which the renderer stretches into soft gradients. Each is its name,
/// its SHA-256, its size in bytes and its bytes in base64.
const PICTURES: [(&str, &str, u32, &str); 3] = [
    (
        "dusk.png",
        "f82c6035f6f8edd7ca9caf348326b1646139d83b6d8e92fc082711de20907c38",
        528,
        "iVBORw0KGgoAAAANSUhEUgAAABAAAAAKCAIAAAAy3EnLAAAB10lEQVR42mNQM8gjCTHYWGTbWmbbWWU7WGc72mQ722a72GW72me7O2R7OGV7OWd7u+T4uOb4ueX4u+cEeuQwBDlkhDhlhLlkRLplRHtkxHllJvhkJvtlpgZkZgRlZYVm5YZn5UdmFUVnl8Rml8dnM2R4pGZ7peb5phYGpJUEp1WEpVdHptfFpDfGZ7QmZXSkZnZnZPZlZ03Ky5pamDWjJJuhLiCpMTipNTy5Mzq5Jy5lQlLqlLTU6Vlps3PT5xemLyrNWFaZuaomc21D1saW7K0d2QxTIuKnx8TPTkiYn5q4ODNpWW7yqqKUdWWpm6rSttWn72zO2NuRebAn6+iE7BNTs8/MzGFYlRCzNiV2Y2bclrz4HcUJuyuS9tcmH2pKOdaeeqon/eyEjItTM6/MyroxP+f2ktz7K/MYDmREHs6NOlYUc7I89kxt/PmmxEvtSVd7U25MSr09Pf3enMyHi7KeLM95vib31ab8t9sLGa4VhN0ojbhVHXWnIeZeW9yD7oRHE5OeTEt5Pift5aKM18uz3q7Neb8p7+OOgs97i74dLmF4UxHypjbsbXPEu87o932xH6YkfJyV9GlByuel6V9WZ37dmPNte973PQU/DhX/PFH661wFAG3/5mNI+mKkAAAAAElFTkSuQmCC",
    ),
    (
        "forest.png",
        "35046616bb8b64fbb086502e415fbd1a21d6f6406598150c96a6ca59c41bde3b",
        546,
        "iVBORw0KGgoAAAANSUhEUgAAAAoAAAAQCAIAAACgHXkXAAAB6UlEQVR42mMQcLDSi/X2rohNm5RTv7xq2u721acn7bs5+/zjxfferGIASuvGeHmURSf1Z1UtqZiwvWXJsf5tV2Ycv7/g6otlIGntaC+3kqi4nozSBaVdmxrnHupZf2Hq/ttzzj5ZDJLWivJ0LoqI6kwrmFPcsq5u+r6O5acnbr828+iDBSBpzUgPh4LwsLaU7JkFdauqJ+xsXXC8b/2laXvuzAFJq4e72+aGBjUnpU3Nq1hW0bm1aebh7uXnJm25MRMkrRbmbp0d4teQkDgxu2hRWdPG+on72uef7F9zZRpIWiXUzSIzyKs2LqYvM2decc2amq5dLdOP9iy5MAkkrRziapoe6FYVE96VljaroHRFZdPWxv6DHbPP9IOkFYNdjFL9ncqjgtpSEqbl5i0prd5Y276ndfKJHpC0QqCzfpKfXUmEb3Ni1KSs9PlFJWur6nc0dh/pAEnLBTjrJvhYFYZ51MeH9KUnzs7PWVFevqWu+UArSFrW30krztssL8S5OsavKyVqek7qkuL89dVVe5pA0tJ+juoxnkbZQXYVUR5ticGTMmPnF6SvLi/aUQeSlvJ1UIny0MsIsCwJd2qK8+lLC5udG7+8JGtLNUha0sdBKcJdO9XPpDDUti7GrSvZf3pWxOLCpA0VANmR1V7mQ0PEAAAAAElFTkSuQmCC",
    ),
    (
        "tide.png",
        "fb407d4dce684133adf7a5c96f2f0b0e38a36bf97a0f571778b9243b905fedde",
        436,
        "iVBORw0KGgoAAAANSUhEUgAAAAwAAAAMCAIAAADZF8uwAAABe0lEQVR42mOQiJomGT1NPn6mRupck7zFLlWrIjo258/c17Xm1IpDN07devHh2y8G+ZTZCimzVdLnaucsMC1e6lSzOqh9U8qUXVWLj0zacn7t8dun77xkUM9bqJG3ULtwkWHJUouqFU4Na/07N8VO2pk370DjqhPTdl5ae/IOg37FciAyqlphVrPKpmGNS+sG3+4tEZN2pMzeV7zkSPP601N3XWIwb1oLRJbN62xa1zt0bHTr3uzbvy106s64WXszFx4sWXGsccMZBvuezUDk0LvZqW+L64StnpO2+03dGTJjd9ScfUkLD2YtO1K8+gSD27QdQOQ+bYfH9J1eM3b6ztoVMHtPyLy9EQv2xy4+mLzscObKowx+C/ZBkP+CfQEL9wct2h+y+EDYkoORyw7FLD8cv/JI8uqjDGGrjsBR+KojEauPRK4+GrXmaMyaY7Frj8WvO564/gRD3JZTaCgejBK2nkrcehqIkradZkjbdwE/St93gSHvxDWCCACVD/j21UztrQAAAABJRU5ErkJggg==",
    ),
];

pub(super) fn picture(index: usize) -> RasterImage {
    let (name, sha256, size, data) = PICTURES[index];
    RasterImage::new(sha256, RasterImageFormat::Png, size, Arc::from(data), name).expect("the constants above are a picture")
}

/// The surface the host plays a short animation on: a band of light that
/// sweeps across, as a looping GIF would.
pub(super) const ANIMATION: &str = "media-animation";
const ANIMATION_FRAMES: usize = 12;
const ANIMATION_STEP: Duration = Duration::from_millis(90);

/// Present the animation's current frame wherever `frame` draws its surface,
/// and say when the next one is due. Its frames are converted once and
/// presented again, shared, every time it loops.
pub fn present_animation(ctx: &eframe::egui::Context, frame: &limn::Frame) -> Option<Duration> {
    static FRAMES: OnceLock<(Instant, Vec<limn_egui::SurfaceFrame>)> = OnceLock::new();
    if !frame.commands.iter().any(|command| matches!(command, limn::DrawCommand::Surface { id, .. } if id == ANIMATION)) {
        return None;
    }
    let (started, frames) = FRAMES.get_or_init(|| {
        let (width, height) = (96, 64);
        let frames = (0..ANIMATION_FRAMES)
            .map(|step| {
                let center = (step as f32 + 0.5) / ANIMATION_FRAMES as f32 * width as f32;
                let rgba: Vec<u8> = (0..width * height)
                    .flat_map(|at| {
                        let (x, y) = ((at % width) as f32, (at / width) as f32);
                        let light = (1.0 - (x - center).abs() / 24.0).max(0.0);
                        let shade = |base: f32| (base + light * (255.0 - base)) as u8;
                        [shade(40.0 + y), shade(70.0), shade(120.0 + y), 255]
                    })
                    .collect();
                limn_egui::SurfaceFrame::from_rgba([width, height], &rgba, 1.0).expect("a small frame")
            })
            .collect();
        (Instant::now(), frames)
    });
    let elapsed = started.elapsed().as_millis() / ANIMATION_STEP.as_millis();
    limn_egui::present_surface(ctx, ANIMATION, frames[elapsed as usize % ANIMATION_FRAMES].clone());
    Some(ANIMATION_STEP)
}

/// What the viewer shows: `open`, `loading`, `failed`, `animation`, or nothing.
const VIEWER: &str = "media-viewer";
const INDEX: &str = "media-viewer-index";

/// The click that opens the viewer on one picture.
fn open_id(index: usize) -> String {
    format!("{VIEWER}=open&{INDEX}={index}")
}

/// The viewer's pictures, drawn once: a wide dusk with a fine grid to zoom
/// into, a small drawing it shows at its own size, and a dark star on
/// nothing, over the checkerboard. Each is its name, its size and the
/// picture.
fn viewer_pictures() -> &'static [(&'static str, Size, RasterImage); 3] {
    static PICTURES: OnceLock<[(&'static str, Size, RasterImage); 3]> = OnceLock::new();
    PICTURES.get_or_init(|| {
        let draw = |name: &'static str, (width, height): (u32, u32), pixel: &dyn Fn(f32, f32) -> [u8; 4]| {
            use base64::Engine as _;
            use image::ImageEncoder as _;
            use sha2::Digest as _;
            let rgba: Vec<u8> = (0..width * height)
                .flat_map(|at| pixel((at % width) as f32, (at / width) as f32))
                .collect();
            let mut png = Vec::new();
            image::codecs::png::PngEncoder::new(&mut png)
                .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
                .expect("a picture drawn here encodes");
            let picture = RasterImage::new(
                format!("{:x}", sha2::Sha256::digest(&png)),
                RasterImageFormat::Png,
                png.len() as u32,
                Arc::from(base64::engine::general_purpose::STANDARD.encode(&png)),
                name,
            )
            .expect("a picture drawn here is small");
            (name, Size::new(width as f32, height as f32), picture)
        };
        [
            draw("dusk.png", (1200, 800), &|x, y| {
                let ridge = 500.0 + 70.0 * (x / 150.0).sin();
                let grid = y > ridge && (x as u32 % 40 < 2 || y as u32 % 40 < 2);
                let t = y / 800.0;
                match (grid, y > ridge) {
                    (true, _) => [235, 228, 210, 255],
                    (false, true) => [(40.0 + 30.0 * t) as u8, (52.0 + 20.0 * t) as u8, 70, 255],
                    _ => [(250.0 - 120.0 * t) as u8, (150.0 - 60.0 * t) as u8, (110.0 + 90.0 * t) as u8, 255],
                }
            }),
            draw("sketch.png", (240, 180), &|x, y| {
                let sun = (x - 170.0).hypot(y - 50.0) < 22.0;
                match (y > 120.0 + 12.0 * (x / 30.0).sin(), sun) {
                    (true, _) => [72, 140, 84, 255],
                    (false, true) => [250, 210, 90, 255],
                    _ => [(120.0 + y / 2.0) as u8, 190, 235, 255],
                }
            }),
            draw("star.png", (400, 400), &|x, y| {
                let (dx, dy) = (x - 200.0, y - 200.0);
                let edge = 120.0 + 50.0 * (dy.atan2(dx) * 5.0).cos();
                if dx.hypot(dy) < edge { [36, 30, 64, 255] } else { [0, 0, 0, 0] }
            }),
        ]
    })
}

/// Demos side by side where the page is `width` wide, and one under another where it is not.
fn fit(cx: &Cx, width: f32, children: Vec<Element>) -> Element {
    if cx.width >= width {
        row(children)
    } else {
        Element::column(children).gap(12.0)
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let thumbnail = |index: usize| {
        let (name, _, picture) = &viewer_pictures()[index];
        Element::clickable(
            open_id(index),
            Element::raster_image(picture.clone().cover()).radius(RadiusRole::Surface.px()).size(Some(120.0), Some(84.0)),
        )
        .accessibility_label(format!("Open {name}"))
    };
    let outline = |id: &str, label: &str| Button::new(id, label, palette).variant(ButtonVariant::Outline).render();

    // A file the demo removed stays out until it is restored.
    let files = [
        ("media-file-notes", "notes.md", "Markdown · 2 KB", Icon::File),
        ("media-file-take", "take-04.wav", "Audio · 3.1 MB", Icon::Audio),
        ("media-file-build", "build.log", "Text · 48 KB", Icon::Terminal),
    ];
    let mut kept: Vec<Element> = files
        .iter()
        .filter(|(id, ..)| !state.flag(&format!("{id}-removed")))
        .map(|(id, title, description, icon)| {
            Attachment::new(*id, *title, palette)
                .description(*description)
                .icon(*icon)
                .remove_action(format!("{id}-removed"))
                .render()
        })
        .collect();
    if kept.len() < files.len() {
        let restore: Vec<String> = files.iter().map(|(id, ..)| format!("{id}-removed=off")).collect();
        kept.push(outline(&restore.join("&"), "Restore"));
    }

    vec![
        section(cx, "Image viewer", vec![
            caption(cx, "Click a picture: it grows out of its thumbnail and covers the window on a dark backdrop. The wheel zooms about the pointer, a drag moves a zoomed picture and glides on, a double click zooms in two and a half times and back, + - 0 1 2 3 and Ctrl+R do what the header does, and the percentage fits or zooms in. The small sketch stays at its own size; the star's transparent pixels show a checkerboard. Left, Right and the thumbnails move through the three; Esc, the close button or a click beside the picture closes it."),
            row((0..viewer_pictures().len()).map(thumbnail).collect()),
            caption(cx, "An animation the host plays on a surface: covering its thumbnail with rounded corners, then in the viewer at its 96 by 64 pixels, zoomed as a picture is."),
            row(vec![Element::clickable(
                format!("{VIEWER}=animation&{INDEX}=0"),
                Element::surface(ANIMATION, "A band of light")
                    .surface_fit(SurfaceFit::Cover)
                    .radius(RadiusRole::Surface.px())
                    .size(Some(120.0), Some(84.0)),
            )
            .accessibility_label("Open the animation")]),
            caption(cx, ".surface_mirrored(true): the same pixels flipped left to right, as a camera's preview is shown to the person in front of it; the band of light runs the other way."),
            row(vec![Element::surface(ANIMATION, "A band of light, mirrored")
                .surface_fit(SurfaceFit::Cover)
                .surface_mirrored(true)
                .radius(RadiusRole::Surface.px())
                .size(Some(120.0), Some(84.0))]),
            outline("clear-image-cache", "Reload image cache"),
            caption(cx, "The native host clears cached textures and pending decodes. Visible pictures are requested again on the next paint pass."),
            caption(cx, "Without a picture the viewer waits with a spinner, or shows the error the host gives it."),
            row(vec![
                outline(&format!("{VIEWER}=loading&{INDEX}=0"), "Open while loading"),
                outline(&format!("{VIEWER}=failed&{INDEX}=0"), "Open with an error"),
            ]),
        ]),
        section(cx, "Attachment", vec![
            caption(cx, "A file card: icon, title and description. The close button reports the remove action and the host drops the file."),
            fit(cx, 232.0 * kept.len() as f32, kept),
            caption(cx, "With an open action the whole card is a button; without one, hovering the name shows it in full."),
            row(vec![
                Attachment::new("media-open-card", "quarterly-report-2026-final-reviewed.pdf", palette)
                    .description("PDF · 1.2 MB")
                    .help("Shared by Ada on Monday")
                    .open_action("media-opened=quarterly-report-2026-final-reviewed.pdf")
                    .render(),
                Attachment::new("media-plain-card", "a-long-file-name-that-does-not-fit.txt", palette)
                    .description("Text · 640 B")
                    .width(160.0)
                    .render(),
            ]),
            caption(cx, &format!("Opened: {}", state.choice("media-opened", "nothing yet"))),
            caption(cx, "With a preview: beside the caption, above it with AttachmentOrientation::Vertical, and alone with preview_only()."),
            fit(cx, 500.0, vec![
                Attachment::new("media-image-row", PICTURES[0].0, palette)
                    .description("PNG · 528 B")
                    .preview(picture(0).cover())
                    .render(),
                Attachment::new("media-image-column", PICTURES[1].0, palette)
                    .description("PNG · 546 B")
                    .preview(picture(1).cover())
                    .orientation(AttachmentOrientation::Vertical)
                    .remove_action("media-image-column-remove")
                    .width(160.0)
                    .render(),
                Attachment::new("media-image-only", PICTURES[2].0, palette)
                    .description("PNG · 436 B")
                    .preview(picture(2).cover())
                    .preview_only()
                    .width(Attachment::PREVIEW_SIZE)
                    .render(),
            ]),
        ]),
        section(cx, "Avatar", vec![
            caption(cx, "Without a photo, the first letter of the name in upper case. Sizes 16, 24, 32 (the default), 48 and 96."),
            row([16.0, 24.0, 32.0, 48.0, 96.0]
                .into_iter()
                .map(|size| Avatar::new("Ada Lovelace", palette).size(size).render().centered_y())
                .collect()),
            caption(cx, "With a photo, which the host crops to a square."),
            row(vec![
                Avatar::new("Grace Hopper", palette).image(picture(2)).size(48.0).render(),
                Avatar::new("Alan Turing", palette).image(picture(0).cover()).size(48.0).render(),
            ]),
            caption(cx, "A lower-case name, one that starts with spaces, one in another script, and no name at all."),
            row(["linus", "  margaret", "東京", ""]
                .into_iter()
                .map(|name| Avatar::new(name, palette).render())
                .collect()),
            caption(cx, "With color(fill): each person's own color behind the initial, in place of the palette's."),
            row([("Ada", (226, 99, 99)), ("Bruno", (88, 148, 226)), ("Carla", (214, 160, 96)), ("Dario", (168, 112, 214))]
                .into_iter()
                .map(|(name, (r, g, b))| Avatar::new(name, palette).color(limn::Color::rgb(r, g, b)).render())
                .collect()),
        ]),
    ]
}

/// Wrap the window in the image viewer while this page has it open.
pub fn layer(cx: &Cx, base: Element) -> Element {
    let state = cx.state;
    let mode = state.text(VIEWER);
    let pictures = viewer_pictures();
    let last = pictures.len() - 1;
    let index = state.choice(INDEX, "0").parse::<usize>().unwrap_or(0).min(last);
    // The root is as large as the window, which is what the viewer covers.
    let window = Size::new(base.style.width.unwrap_or(900.0), base.style.height.unwrap_or(620.0));
    // Each control has an ID of its own, also where two would set the same value.
    let step = |index: usize, control: &str| format!("{INDEX}={index}&media-viewer-control={control}");
    let (name, size, picture) = &pictures[index];
    let title = if mode == "animation" { "band.gif" } else { name };
    let mut viewer = ImageViewer::new(base, matches!(mode, "open" | "loading" | "failed" | "animation"), title, format!("{VIEWER}=closed"), cx.palette)
        .viewport(window)
        .subtitle(format!("{} × {} · PNG", size.width, size.height))
        .gallery(index, pictures.len())
        .previous_action(step(index.saturating_sub(1), "previous"))
        .next_action(step((index + 1).min(last), "next"))
        .action("media-viewer-control=save", Icon::ArrowDown, "Save as…")
        .zoom_actions("media-viewer-control=out", "media-viewer-control=fit", "media-viewer-control=in")
        .return_focus(open_id(index));
    for (at, (name, _, picture)) in pictures.iter().enumerate() {
        viewer = viewer.thumbnail(step(at, "thumbnail"), Some(picture.clone()), *name);
    }
    match mode {
        "loading" => viewer,
        "failed" => viewer.error("The picture could not be decoded."),
        // Not a picture: the arrows say what they move to.
        "animation" => viewer
            .surface(ANIMATION, "A band of light")
            .picture_size(Size::new(96.0, 64.0))
            .navigation_labels("Previous item", "Next item"),
        _ => viewer.image(picture.clone()).picture_size(*size),
    }
    .render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{self, State};
    use limn::{DrawCommand, Key, UiEvent, UiTree};

    #[test]
    fn the_viewer_opens_on_a_picture_steps_with_the_arrows_and_closes_on_escape() {
        let window = Size::new(900.0, 620.0);
        let mut state = State::starting_at("image-viewer");
        let press = |state: &mut State, key: Key| {
            let mut tree = UiTree::new(app::view(state, window));
            tree.set_reduced_motion(true);
            let frame = tree.layout(window);
            let drawn = frame.commands.iter().filter(|command| matches!(command, DrawCommand::RasterImage { .. })).count();
            for action in tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false }) {
                app::update(state, &action);
            }
            drawn
        };
        app::update(&mut state, &limn::UiAction::Clicked { id: open_id(1) });
        // The three thumbnails on the page, the picture it shows and the
        // three in its strip.
        assert_eq!(press(&mut state, Key::ArrowRight), 7);
        assert_eq!((state.text(VIEWER), state.text(INDEX)), ("open", "2"));
        // Nothing after the last.
        press(&mut state, Key::ArrowRight);
        assert_eq!(state.text(INDEX), "2");
        press(&mut state, Key::Escape);
        assert_eq!(state.text(VIEWER), "closed");
        assert_eq!(press(&mut state, Key::ArrowLeft), 3);
        assert_eq!(state.text(INDEX), "2");
        // The header's zoom moves the picture; the page does not keep it.
        app::update(&mut state, &limn::UiAction::Clicked { id: open_id(0) });
        let mut tree = UiTree::new(app::view(&state, window));
        tree.set_reduced_motion(true);
        tree.layout(window);
        let zoomed = tree.handle_event(UiEvent::Key { key: Key::Plus, shift: false, ctrl: false, alt: false });
        assert!(zoomed.iter().any(|action| matches!(action, limn::UiAction::Zoomed { fitted: false, .. })));
    }
}
