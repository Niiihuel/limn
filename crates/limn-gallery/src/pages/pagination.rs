use super::{caption, section, Cx, Section};
use limn::{Component, ControlSize, Pagination};

/// A row over `pages` pages whose position the gallery keeps under `group`:
/// each button reports `group=page`, which the gallery takes as a choice.
fn pagination(cx: &Cx, group: &str, name: &str, pages: usize) -> Pagination {
    let page = cx.state.choice(group, "1").parse().unwrap_or(1);
    Pagination::new(group, page, pages, cx.palette).accessibility_label(name)
}

pub fn view(cx: &Cx) -> Vec<Section> {
    vec![
        section(cx, "Pagination", vec![
            pagination(cx, "pagination-main", "Results pages", 12).render(),
            caption(cx, &format!("Page {} of 12. The row keeps its cells in place as the page changes.", cx.state.choice("pagination-main", "1"))),
        ]),
        section(cx, "Siblings", vec![
            pagination(cx, "pagination-none", "Pages, no siblings", 40).siblings(0).render(),
            caption(cx, "siblings(0): the first, the current and the last."),
            pagination(cx, "pagination-two", "Pages, two siblings", 40).siblings(2).render(),
            caption(cx, "siblings(2): two pages on each side of the current one."),
        ]),
        section(cx, "Compact", vec![
            pagination(cx, "pagination-compact", "Results pages, compact", 12).compact().render(),
            caption(cx, "The arrows around the position, for a narrow place."),
        ]),
        section(
            cx,
            "Sizes",
            [
                ("Dense", ControlSize::Dense),
                ("Compact", ControlSize::Compact),
                ("Regular", ControlSize::Regular),
                ("Spacious", ControlSize::Spacious),
            ]
            .into_iter()
            .flat_map(|(name, size)| {
                let id = format!("pagination-size-{}", name.to_lowercase());
                [pagination(cx, &id, &format!("{name} pages"), 12).size(size).render(), caption(cx, name)]
            })
            .collect(),
        ),
        section(cx, "Disabled", vec![
            pagination(cx, "pagination-off", "Results pages, disabled", 12).disabled().render(),
            caption(cx, "A disabled row shows its page and takes no pointer or focus."),
            pagination(cx, "pagination-off-compact", "Results pages, compact and disabled", 12).compact().disabled().render(),
        ]),
        section(cx, "Edge cases", vec![
            pagination(cx, "pagination-few", "Seven pages", 7).render(),
            caption(cx, "Seven pages fit, so none is left out."),
            pagination(cx, "pagination-one", "One page", 1).render(),
            caption(cx, "One page: both arrows are disabled."),
            pagination(cx, "pagination-empty", "No pages", 0).render(),
            caption(cx, "No pages: the two arrows alone, disabled."),
            pagination(cx, "pagination-long", "Two thousand pages", 2000).render(),
            caption(cx, "Every number takes the width of the widest, so the row does not grow on the way to page 2000."),
        ]),
    ]
}
