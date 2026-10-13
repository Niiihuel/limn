use super::{caption, row, section, Cx, Section};
use limn::{AlertColors, AlertTone, Component, Element, Skeleton, SkeletonGroup, TextShimmer};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(336.0);
    // A group's lines are shares of the width it is given.
    let framed = |group: SkeletonGroup| Element::panel(group.render()).size(Some(width), None);
    vec![
        section(cx, "Skeleton", vec![
            row(vec![
                Skeleton::circle("loading-avatar", palette, 40.0).render(),
                Element::column(vec![
                    Skeleton::line("loading-title", palette, 180.0).render(),
                    Skeleton::line("loading-text", palette, 240.0).render(),
                ])
                .gap(8.0),
            ]),
            caption(cx, "Skeleton::circle and two Skeleton::line of different widths: a person and two lines of text."),
            row(vec![Skeleton::block("loading-preview", palette, width, 120.0).render()]),
            caption(cx, "Skeleton::block: a card, an image or a preview."),
            Element::panel(Skeleton::new("loading-bar", palette).height(20.0).radius(4.0).render())
                .size(Some(width), None),
            caption(cx, "Skeleton::new without a width fills the room it is given; .height(20.0).radius(4.0)."),
            framed(SkeletonGroup::rows("loading-rows", palette, 4)),
            caption(cx, "SkeletonGroup::rows(id, palette, 4): lines of varying length, 28 px apart."),
            framed(SkeletonGroup::rows("loading-files", palette, 3).icon(true).row_height(36.0).fractions(&[0.8, 0.5])),
            caption(cx, ".icon(true).row_height(36.0).fractions(&[0.8, 0.5]): a leading square, and the shares repeat in order."),
        ]),
        section(cx, "Text shimmer", vec![
            row(vec![TextShimmer::new("loading-shimmer", "Reading the project files", palette).render()]),
            caption(cx, "Muted text with a highlight in the palette's text color passing over it."),
            row(vec![TextShimmer::new("loading-shimmer-ink", "Waiting for the build", palette)
                .colors(palette.muted, AlertColors::for_tone(AlertTone::Info, palette).accent)
                .render()]),
            caption(cx, ".colors(base, highlight): the same over the host's own two colors."),
        ]),
    ]
}
