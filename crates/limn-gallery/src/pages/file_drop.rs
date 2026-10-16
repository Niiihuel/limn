//! A static lifetime illustration, without native handles or protocol activity.

use super::{caption, section, Cx, Section};
use limn::{Element, RadiusRole};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let endpoint = |title: &str, description: &str| {
        Element::panel(
            Element::column(vec![
                Element::text(title)
                    .font_size(14.0)
                    .foreground(palette.text),
                Element::text(description)
                    .font_size(13.0)
                    .foreground(palette.muted),
            ])
            .gap(6.0),
        )
        .padding(14.0)
        .background(palette.raised)
        .border(palette.border, 1.0)
        .radius(RadiusRole::Card.px())
    };
    let illustration = Element::panel(Element::column(vec![
        Element::text("Owner: Ready (illustrated)").font_size(17.0).foreground(palette.text),
        Element::text("One connection and one worker; devices remain alive.").font_size(13.0).foreground(palette.muted),
        endpoint("Endpoint A: Detached", "Its surface permit is returned. Its old terminal stays drainable."),
        endpoint("Endpoint B: Ready", "A can retire while B remains registered."),
        Element::text("Detached is local. Keep host dispatch alive until owner Closed after cleanup and join.")
            .font_size(13.0).foreground(palette.text),
        Element::text("No native reception is performed here. An SDK host demo needs a shared owner with working clipboard support.")
            .font_size(12.0).foreground(palette.muted),
    ]).gap(12.0))
    .padding(20.0)
    .background(palette.surface)
    .border(palette.border, 1.0)
    .radius(RadiusRole::Panel.px())
    .size(Some(cx.width.min(560.0)), None);

    let text_flow = Element::panel(Element::column(vec![
        endpoint("1. Reserve", "Reserve the focused Selection and shared incoming slot. No read yet."),
        endpoint("2. ACK", "Capture the destination, then acknowledge once to queue the worker read."),
        endpoint("3. Reading", "Wait for EOF and strict UTF-8 validation. A partial body is not a result."),
        endpoint("4. Publish after watcher removal", "The worker removes the actual reader watcher before publishing valid text."),
        endpoint("5. Poll once or expire", "Poll transfers one result, including empty text. Completion TTL expires without polling."),
        Element::text("Cancel or request Drop cuts delivery; the worker retires the reader and body before reusing the slot.")
            .font_size(13.0).foreground(palette.text),
        Element::text("RAW text stays in RAM. CRLF is unchanged; no trimming or normalization.")
            .font_size(13.0).foreground(palette.muted),
    ]).gap(12.0))
    .padding(20.0)
    .background(palette.surface)
    .border(palette.border, 1.0)
    .radius(RadiusRole::Panel.px())
    .size(Some(cx.width.min(560.0)), None);

    vec![
        section(
            cx,
            "Synthetic owner lifecycle",
            vec![
                caption(
                    cx,
                    "Synthetic lifecycle illustration: no native owner attached.",
                ),
                illustration,
            ],
        ),
        section(
            cx,
            "Synthetic text paste",
            vec![
                caption(
                    cx,
                    "Synthetic text flow: no OS clipboard access; no native owner attached.",
                ),
                text_flow,
            ],
        ),
    ]
}
