use super::{caption, row, section, Cx, Section};
use limn::{
    Breadcrumb, BreadcrumbItem, Button, ButtonVariant, Collapsible, Color, Component, Composer,
    ComponentPalette, ControlSize, DictationButton, DictationState, DictationWaveform, Element, FindBar, FindIds,
    FindLabels, FindOptions, Icon, IconButton, Marker,
};

const DANGER: Color = Color::rgb(229, 83, 75);
const WIDTHS: [&str; 3] = ["260", "480", "520"];
const PATH: &str = "crates/limn/src/find_bar.rs";
/// Two seconds of someone speaking, a level every 80 ms.
const SPEECH: [f32; 24] = [
    0.0, 0.05, 0.3, 0.55, 0.4, 0.7, 0.9, 0.6, 0.35, 0.1, 0.0, 0.0, 0.2, 0.5, 0.8, 1.0, 0.75, 0.45, 0.6, 0.3, 0.15, 0.4, 0.65, 0.35,
];

fn find_labels() -> FindLabels {
    FindLabels {
        query: "Find in file".into(),
        replacement: "Replace with".into(),
        case: "Match case".into(),
        word: "Whole words only".into(),
        regex: "Regular expression".into(),
        in_selection: "Find in selection".into(),
        toggle_replace: "Show or hide replacement".into(),
        previous: "Previous match".into(),
        next: "Next match".into(),
        dismiss: "Close search".into(),
        replace_one: "Replace match".into(),
        replace_all: "Replace all matches".into(),
        count: "Current match / total".into(),
        ..FindLabels::default()
    }
}

// The gallery flips a flag per click, so the option IDs are the flags.
fn find_bar(cx: &Cx, width: f32) -> Element {
    let state = cx.state;
    let query = state.text("editor-find-query");
    let found = !query.is_empty() && !query.contains("zzz");
    let regex = state.flag("editor-find-regex");
    FindBar::new("editor-find", query, cx.palette)
        .ids(FindIds {
            query: "editor-find-query".into(),
            replacement: "editor-find-replacement".into(),
            case: "editor-find-case".into(),
            word: "editor-find-word".into(),
            regex: "editor-find-regex".into(),
            in_selection: "editor-find-scope".into(),
            toggle_replace: "editor-find-replace".into(),
            ..FindIds::new("editor-find")
        })
        .labels(find_labels())
        .replacement(Some(state.text("editor-find-replacement")))
        .replace_open(state.flag("editor-find-replace"))
        .options(FindOptions {
            case: state.flag("editor-find-case"),
            word: state.flag("editor-find-word"),
            regex,
            in_selection: state.flag("editor-find-scope"),
        })
        .count_label(match (query.is_empty(), found) {
            (true, _) => "",
            (false, true) => "2 / 14",
            (false, false) => "0 / 0",
        })
        .has_matches(found)
        .has_current(found)
        .no_matches(!query.is_empty() && !found)
        .error((regex && query.contains('(') && !query.contains(')')).then_some("Unclosed parenthesis in the expression"))
        .ink(DANGER)
        .width(width)
        .render()
}

fn composer(cx: &Cx, width: f32) -> Element {
    let palette = cx.palette;
    let strip = Element::row(vec![
        Button::new("editor-composer-project", "limn", palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Compact)
            .icon(Icon::Folder)
            .render(),
        Button::new("editor-composer-branch", "ui-separation", palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Compact)
            .icon(Icon::GitBranch)
            .render(),
        Element::spacer().flex(1.0),
    ])
    .gap(4.0)
    .size(Some(Composer::header_strip_width(width)), Some(28.0));
    let tool = |id: &str, icon| {
        IconButton::new(id, icon, palette).variant(ButtonVariant::Ghost).size(ControlSize::Compact).render()
    };
    // Each click moves the session on: idle, starting, listening,
    // transcribing, idle.
    let (state, next) = match cx.state.choice("editor-dictation", "idle") {
        "starting" => (DictationState::Starting, "listening"),
        "listening" => (DictationState::Listening, "transcribing"),
        "transcribing" => (DictationState::Transcribing, "idle"),
        _ => (DictationState::Idle, "starting"),
    };
    // While the microphone is open its level stands beside it; before and after, it is silence.
    let levels: &[f32] = if state == DictationState::Listening { &SPEECH } else { &[] };
    let mut trailing = vec![
        DictationButton::new(format!("editor-dictation={next}"), state, palette)
            .size(ControlSize::Compact)
            .render(),
        tool("editor-composer-send", Icon::Send),
    ];
    if state != DictationState::Idle {
        trailing.insert(0, DictationWaveform::new("editor-composer-levels", levels, palette).size(ControlSize::Compact).width(48.0).render());
    }
    let trailing = Element::row(trailing).gap(4.0);
    Composer::new("editor-composer", cx.state.text("editor-composer"), "Describe a task", width, palette)
        .pill_controls(tool("editor-composer-add", Icon::Add), trailing)
        .auto_grow(28.0, 120.0)
        .header_strip(strip, ComponentPalette { surface: palette.raised, ..palette })
        .render()
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let chosen = state.choice("editor-find-width", "520");
    let width: f32 = chosen.parse().unwrap_or(520.0);
    let widths = WIDTHS
        .iter()
        .map(|option| {
            Button::new(format!("editor-find-width={option}"), *option, palette)
                .variant(if *option == chosen { ButtonVariant::Secondary } else { ButtonVariant::Outline })
                .render()
        })
        .collect();
    let crumb_width = cx.width.min(420.0);
    let linked = |id: &str, dirty: bool| {
        let folder = format!("{id}-folder");
        let file = format!("{id}-file");
        Breadcrumb::from_path(id, PATH, palette)
            .dirty(dirty)
            .segment_action(move |segment| Some(if segment == PATH { file.clone() } else { format!("{folder}={segment}") }))
            .width(crumb_width)
    };
    vec![
        section(
            cx,
            "Find and replace",
            vec![
                caption(cx, "The app searches; the bar shows the query, options and count. “zzz” finds nothing."),
                row(widths),
                row(vec![find_bar(cx, width.min(cx.width))]),
                caption(cx, "Below 480 px, or with replacement open, the options move to their own row."),
            ],
        ),
        section(
            cx,
            "File path",
            vec![
                caption(cx, "Passive: only shows where the file is."),
                Breadcrumb::from_path("editor-path", PATH, palette).width(crumb_width).render(),
                caption(cx, "With unsaved changes and actions supplied by the app for each segment."),
                linked("editor-path-dirty", true).render(),
                caption(cx, "A symbol from the file, added after the path."),
                linked("editor-path-symbol", false)
                    .item(BreadcrumbItem::new("render").action("editor-path-symbol-render"))
                    .render(),
            ],
        ),
        section(
            cx,
            "Composer with header strip",
            vec![
                caption(cx, "The strip holds message metadata, attached to the card’s top edge."),
                row(vec![composer(cx, cx.width.min(480.0))]),
                caption(cx, "The composer microphone advances with each click: idle, starting, listening, transcribing."),
                row(vec![
                    DictationButton::new("editor-dictation-idle", DictationState::Idle, palette).render(),
                    DictationButton::new("editor-dictation-starting", DictationState::Starting, palette).render(),
                    DictationButton::new("editor-dictation-listening", DictationState::Listening, palette).render(),
                    DictationButton::new("editor-dictation-accent", DictationState::Listening, palette)
                        .live_palette(ComponentPalette { surface: DANGER, text: Color::WHITE, ..palette })
                        .render(),
                    DictationButton::new("editor-dictation-off", DictationState::Listening, palette)
                        .enabled(false)
                        .render(),
                ]),
                caption(cx, "DictationButton: idle, starting, listening, listening with the app palette, and dictation unavailable."),
                row(vec![
                    DictationWaveform::new(
                        "editor-waveform",
                        match cx.state.choice("editor-levels", "silence") {
                            "speech" => &SPEECH,
                            "loud" => &[1.0; 24],
                            _ => &[],
                        },
                        palette,
                    )
                    .render(),
                    Button::new("editor-levels=silence", "Silence", palette).variant(ButtonVariant::Outline).render(),
                    Button::new("editor-levels=speech", "Speech", palette).variant(ButtonVariant::Outline).render(),
                    Button::new("editor-levels=loud", "Maximum", palette).variant(ButtonVariant::Outline).render(),
                ]),
                caption(cx, "DictationWaveform: microphone levels, newest at the end. Silence appears as dots, and each bar moves to its level with a spring."),
            ],
        ),
        section(
            cx,
            "Status ink",
            vec![
                caption(cx, "A marker and a collapsible with the app’s error color in their secondary text."),
                Marker::new("editor-marker", "Response received", palette).icon(Icon::Check).render(),
                Marker::new("editor-marker-failed", "Model request failed", palette)
                    .icon(Icon::Code)
                    .ink(DANGER)
                    .render(),
                Collapsible::new(
                    "editor-collapsible-failed",
                    "Exploration · 5",
                    caption(cx, "crates/limn/src/missing.rs: does not exist"),
                    palette,
                )
                .icon(Icon::Search)
                .detail("3 reads · 2 failed")
                .ink(DANGER)
                .expanded(state.flag("editor-collapsible-failed"))
                .render(),
            ],
        ),
    ]
}
