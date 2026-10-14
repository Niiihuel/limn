use super::{caption, row, section, Cx, Section};
use limn::{Avatar, AvatarGroup, Button, ButtonVariant, Component, Element};

const NAMES: [&str; 8] = ["Ada", "Grace", "Linus", "Margaret", "Barbara", "Edsger", "Katherine", "Donald"];

/// The first `count` people of the demo, the names repeating past the eighth.
fn people(cx: &Cx, count: usize) -> Vec<Avatar> {
    NAMES.iter().cycle().take(count).map(|name| Avatar::new(*name, cx.palette)).collect()
}

/// What a group says to a screen reader. The words are the host's.
fn named(count: usize) -> String {
    match count {
        1 => "1 person".to_owned(),
        count => format!("{count} people"),
    }
}

/// A group of `count` people, named for a screen reader.
fn group(cx: &Cx, id: &str, count: usize) -> AvatarGroup {
    AvatarGroup::new(people(cx, count), cx.palette).id(id).accessibility_label(named(count))
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let count: usize = cx.state.text("avatars-count").parse().unwrap_or(6);
    let step = |label: &str, to: usize| {
        Button::new(format!("avatars-count={to}"), label, palette).variant(ButtonVariant::Secondary).render()
    };
    vec![
        section(cx, "Group", vec![
            row(vec![
                group(cx, "avatars-team", count).limit(4).render().centered_y(),
                step("Add one", count.saturating_add(1)),
                step("Remove one", count.saturating_sub(1).max(1)),
                step("Times ten", count.saturating_mul(10)),
            ]),
            caption(cx, &format!("{}, limit(4). The first lies on top; the rest are counted.", named(count))),
        ]),
        section(cx, "Limit", vec![
            row(vec![
                group(cx, "avatars-limit-none", 5).render(),
                group(cx, "avatars-limit-3", 5).limit(3).render(),
                group(cx, "avatars-limit-fits", 3).limit(3).render(),
                group(cx, "avatars-limit-0", 5).limit(0).render(),
            ]),
            caption(cx, "Five with no limit, five with limit(3), three with limit(3), and a limit of zero, which is one."),
        ]),
        section(cx, "Size", vec![
            row([16.0, 24.0, 32.0, 48.0]
                .into_iter()
                .map(|size| group(cx, &format!("avatars-size-{size}"), 6).limit(3).size(size).render().centered_y())
                .collect()),
            caption(cx, "16, 24, 32 (the default) and 48. The size replaces each avatar's own."),
        ]),
        section(cx, "Overlap", vec![
            row([0.0, 0.25, 0.4, 0.6]
                .into_iter()
                .map(|overlap| group(cx, &format!("avatars-overlap-{overlap}"), 4).overlap(overlap).render())
                .collect()),
            caption(cx, "0, a quarter (the default), 0.4 and 0.6, the most there is."),
        ]),
        section(cx, "Ring", vec![
            row(vec![Element::panel(group(cx, "avatars-ring", 6).limit(4).ring(palette.raised).render())
                .padding(12.0)
                .background(palette.raised)
                .radius(limn::tokens().radius.card)]),
            caption(cx, "On a raised ground the ring takes its color with ring(palette.raised)."),
        ]),
        section(cx, "Edge cases", vec![
            row(vec![
                group(cx, "avatars-one", 1).render(),
                group(cx, "avatars-many", 150).limit(2).render(),
                AvatarGroup::new(Vec::new(), palette).render(),
                AvatarGroup::new(vec![Avatar::new("", palette), Avatar::new("linus", palette)], palette)
                    .id("avatars-unnamed")
                    .accessibility_label(named(2))
                    .render(),
            ]),
            caption(cx, "One person, 150 with limit(2), which reads 99+, nobody, which draws nothing, and a group with a blank name in it."),
        ]),
    ]
}
