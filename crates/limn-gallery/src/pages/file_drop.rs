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

    vec![section(
        cx,
        "Synthetic owner lifecycle",
        vec![
            caption(
                cx,
                "Synthetic lifecycle illustration: no native owner attached.",
            ),
            illustration,
        ],
    )]
}
