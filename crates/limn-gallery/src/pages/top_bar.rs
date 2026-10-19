use super::{caption, section, Cx, Section};
use crate::app::State;
use limn::{
    Avatar, Badge, ButtonVariant, Component, Element, FontFamily, Icon, IconButton, RadiusRole, ScrollArea, TopBar,
    UiAction,
};

/// The bars whose page the demo keeps, by ID.
const BARS: [&str; 4] = ["top-bar", "top-bar-caption", "top-bar-narrow", "top-bar-page"];
/// The searches whose query it keeps; each bar's reports under `{bar}:search`.
const SEARCHES: [&str; 3] = ["top-bar:search", "top-bar-caption:search", "top-bar-page:search"];
const PLACEHOLDER: &str = "What do you want to play?";

/// Home and Browse choose the page shown; a clear button empties its query.
pub fn update(state: &mut State, action: &UiAction) {
    let UiAction::Clicked { id } = action else { return };
    if let Some(search) = id.strip_suffix(":clear").filter(|search| SEARCHES.contains(search)) {
        state.set(search, "");
    } else if let Some((bar, page)) =
        id.rsplit_once(':').filter(|(bar, page)| BARS.contains(bar) && matches!(*page, "home" | "browse"))
    {
        state.set(&format!("{bar}:page"), page);
    }
}

/// The searches a clear button changes, each with its query now.
pub fn fields(state: &State) -> Vec<(&'static str, String)> {
    SEARCHES.iter().map(|search| (*search, state.text(search).to_owned())).collect()
}

/// A bar as the demo keeps it: its query, the page shown, and the host's
/// controls at both ends.
fn bar(cx: &Cx, id: &str, width: f32) -> TopBar {
    let palette = cx.palette;
    let control = |name: &str, icon: Icon, label: &str| {
        IconButton::new(format!("{id}-{name}"), icon, palette)
            .variant(ButtonVariant::Ghost)
            .round()
            .tooltip(label)
    };
    let page = cx.state.choice(&format!("{id}:page"), "home");
    TopBar::new(id, cx.state.text(&format!("{id}:search")), PLACEHOLDER, palette)
        .width(width)
        .home_active(page == "home")
        .browse_active(page == "browse")
        .leading(vec![
            control("back", Icon::ArrowLeft, "Go back").render(),
            // Nothing to go forward to: the host's own control, disabled.
            control("forward", Icon::ArrowRight, "Go forward").disabled().render(),
        ])
        .trailing(vec![
            Badge::new(control("news", Icon::Bell, "What's new").render(), palette).dot().description("New releases").render(),
            control("friends", Icon::Users, "Friend activity").render(),
            Avatar::new("Nihuel", palette).size(32.0).render(),
        ])
}

/// A bar on the window's ground, in an outline as wide as it is laid out.
fn framed(cx: &Cx, width: f32, content: Element) -> Element {
    Element::panel(content)
        .background(cx.palette.surface)
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Surface.px())
        .clip_children()
        .size(Some(width), None)
}

/// A page under the bar whose header fills in as it scrolls.
fn page(cx: &Cx, width: f32) -> Element {
    let palette = cx.palette;
    let sans = |text: &str, size: f32| Element::text(text).font_family(FontFamily::Sans).font_size(size).foreground(palette.text);
    let header = Element::row(vec![sans("Daily Mix 1", 20.0).centered_y()])
        .padding(16.0)
        .size(None, Some(64.0))
        .background(palette.selected)
        .sticky_top_background_fade_in(10.0, 80.0);
    let mut children = vec![
        header,
        Element::column(vec![
            Element::spacer().flex(1.0),
            sans("Daily Mix 1", 32.0),
            Element::text("Made for you · 50 songs").font_family(FontFamily::Sans).font_size(13.0).foreground(palette.muted),
        ])
        .gap(8.0)
        .padding(16.0)
        .size(None, Some(160.0)),
    ];
    children.extend((1..=24).map(|track| {
        Element::row(vec![
            Element::text(track.to_string()).font_family(FontFamily::Sans).font_size(13.0).foreground(palette.muted).size(Some(24.0), None),
            sans(&format!("Track {track}"), 14.0),
        ])
        .padding(16.0)
        .size(None, Some(48.0))
    }));
    Element::column(vec![
        bar(cx, "top-bar-page", width).render(),
        ScrollArea::new("top-bar-page-scroll", Element::column(children)).render().size(None, Some(320.0)),
    ])
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let width = cx.width;
    let narrow = width.min(760.0);
    vec![
        section(cx, "Top bar", vec![
            framed(cx, width, bar(cx, "top-bar", width).render()),
            caption(cx, "Home and Browse choose the page and draw it solid; type, and a button before the rule clears the query. The search takes half the bar, between 350 and 546, centered in it."),
        ]),
        section(cx, "Window controls", vec![
            framed(cx, width, bar(cx, "top-bar-caption", width).caption(72.0, 138.0).render()),
            caption(cx, ".caption(72, 138): room at both ends for the window's own buttons. The search stays centered in the whole bar, or follows the controls at the start when it would meet them."),
        ]),
        section(cx, "Narrow", vec![
            framed(cx, narrow, bar(cx, "top-bar-narrow", narrow).render()),
            caption(cx, "At 800 or less the search would be 400 wide or less, so it is a round button that opens the search."),
        ]),
        section(cx, "Disabled", vec![
            framed(cx, width, TopBar::new("top-bar-disabled", "abba", PLACEHOLDER, cx.palette).width(width).disabled().render()),
            caption(cx, "Home and the search keep their place and their query, and take no pointer or focus."),
        ]),
        section(cx, "A page that scrolls under it", vec![
            framed(cx, width, page(cx, width)),
            caption(cx, "The header over the page fills in from 10 to 80 px of scroll while the page slides under it: sticky_top_background_fade_in(10, 80)."),
        ]),
    ]
}
