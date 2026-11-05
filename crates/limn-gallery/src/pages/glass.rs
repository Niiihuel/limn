use super::{caption, section, Cx, Section};
use limn::{Component, Element, GlassMaterial, Switch, Tab, Tabs, SegmentedControl, FloatingGlassBar, FloatingGlassTab, GlassMenu, DropdownMenuItem, ContextMenu, Icon, Rect, BottomSheet};

pub fn view(cx: &Cx) -> Vec<Section> {
    let p = cx.palette;
    let width = cx.width.min(720.0);
    let material = GlassMaterial::new(limn::Color { a: 28, ..p.surface });
    let enabled = cx.state.flag("glass-switch");
    let selection = cx.state.choice("glass-tab", "home");
    let segments = || vec![Tab::new("glass-segment=day", "Day", 80.0).active(cx.state.choice("glass-segment", "day") == "day"),
        Tab::new("glass-segment=week", "Week", 80.0).active(cx.state.choice("glass-segment", "day") == "week"),
        Tab::new("glass-segment=month", "Month", 80.0).active(cx.state.choice("glass-segment", "day") == "month")];
    let controls = Element::column(vec![
        Element::text("Real backdrop, sharp foreground").font_size(22.0).foreground(p.text),
        Switch::new("glass-switch", enabled, p).track_size(64.0, 28.0).thumb_inset(2.0).thumb_width(40.0).ink(limn::AlertColors::for_tone(limn::AlertTone::Success, p).accent)
            .glass(material.blur(0.0).magnify(1.15).bezel(5.0)).render(),
        Tabs::new(vec![Tab::new("glass-tab=home", "Home", 90.0).active(selection == "home"),
            Tab::new("glass-tab=browse", "Browse", 90.0).active(selection == "browse"),
            Tab::new("glass-tab=library", "Library", 90.0).active(selection == "library")], p)
            .height(38.0).radius(21.0).render().glass_selection(material.blur(0.0).magnify(1.1).bezel(6.0)),
        SegmentedControl::new(segments(), p).height(36.0).radius(18.0).render()
            .glass_selection(material.blur(0.0).magnify(1.1).bezel(6.0)),
    ]).gap(18.0).padding(24.0);
    let lens = Element::panel(controls).glass(material).radius(limn::tokens().radius.panel)
        .size(Some((width - 64.0).max(100.0)), Some(264.0));
    let placement = Element::column(vec![Element::spacer().size(None, Some(155.0)),
        Element::row(vec![Element::spacer().size(Some(32.0), None), lens])]);
    let origin = |id: &str| {
        let values: Vec<f32> = cx.state.text(id).split(',').filter_map(|v| v.parse().ok()).collect();
        if values.len() == 4 { Rect::new(values[0], values[1], values[2], values[3]) }
        else { Rect::new(0.0, 0.0, 36.0, 36.0) }
    };
    let menu = GlassMenu::new("glass-menu", Element::button("glass-menu=open", "•••").foreground(p.text).size(Some(36.0), Some(36.0)).radius(18.0),
        origin("glass-menu-origin"), vec![DropdownMenuItem::new("glass-menu=queue", "Add to queue").into(),
            DropdownMenuItem::new("glass-menu=playlist", "Add to playlist").into(),
            DropdownMenuItem::new("glass-menu=share", "Share song").into()],
        cx.state.choice("glass-menu", "closed") == "open", material, p).render();
    let held = ContextMenu::new("glass-held-context", Element::button("glass-held-row", "Hold or secondary-click this song").foreground(p.text), vec![], p)
        .open_action("glass-held=open").render();
    let held = GlassMenu::new("glass-held", held, origin("glass-held-origin"),
        vec![DropdownMenuItem::new("glass-held=queue", "Play next").into(), DropdownMenuItem::new("glass-held=share", "Share").into()],
        cx.state.choice("glass-held", "closed") == "open", material, p).render();
    let dock_selection = cx.state.choice("glass-dock", "home");
    let bar = FloatingGlassBar::new("glass-dock", vec![
        FloatingGlassTab::new("glass-dock=home", "Home", Icon::Home).selected(dock_selection == "home"),
        FloatingGlassTab::new("glass-dock=browse", "Browse", Icon::Search).selected(dock_selection == "browse"),
        FloatingGlassTab::new("glass-dock=library", "Library", Icon::Library).selected(dock_selection == "library")],
        Element::clickable("glass-search", Element::icon(Icon::Search).foreground(p.text).size(Some(22.0), Some(22.0)).centered()).accessible(limn::Access::new(limn::Role::Button).name("Search")), material, p)
        .width((width - 48.0).min(440.0)).player(Element::button("glass-play", "▶   Aurora — Now playing").foreground(p.text))
        .collapse(cx.state.text("glass-collapse-progress").parse().unwrap_or(if cx.state.flag("glass-collapse") { 1.0 } else { 0.0 })).render();
    let sheet = BottomSheet::new(Element::button("glass-sheet=open", "Open glass sheet").foreground(p.text),
        cx.state.choice("glass-sheet", "closed") == "open", "Aurora", "The optical shell follows the shared sheet transition.", "glass-sheet=closed", p)
        .glass(material).animation_ms(limn::tokens().motion.move_ms.saturating_mul(2)).body(Element::column(vec![Element::text("Now playing").foreground(p.text),
            Element::button("glass-sheet=closed", "Done").foreground(p.text)]).gap(18.0)).render();
    vec![caption(cx, "The page is rendered into a separate GPU texture. The center samples its blur; the rim samples the sharp page. Move the selection and press the switch.").into(),
        section(cx, "Material", vec![Element::stack(vec![Element::surface("glass:backdrop", "Animated page backdrop"), placement])
            .size(Some(width), Some(440.0)).clip_children()]),
        section(cx, "Navigation and menus", vec![Element::button("glass-collapse", "Toggle scroll collapse").foreground(p.text), bar, menu, held, sheet]),
        section(cx, "Limits", vec![Element::text("An external video must supply a sampleable GPU texture. A callback drawn directly to the window cannot become a backdrop.").foreground(p.muted)])]
}

pub fn track_geometry(state: &mut crate::app::State, frame: &limn::Frame) {
    for (control, key) in [("glass-menu=open", "glass-menu-origin"), ("glass-held-row", "glass-held-origin")] {
        if let Some(region) = frame.regions.iter().find(|region| region.id == control) {
            let r = region.rect;
            state.set(key, &format!("{},{},{},{}", r.x, r.y, r.width, r.height));
        }
    }
}

pub fn update(state: &mut crate::app::State, action: &limn::UiAction) {
    if let limn::UiAction::Clicked { id } = action {
        match id.as_str() {
            "glass-menu:dismiss" => state.set("glass-menu", "closed"),
            "glass-held:dismiss" => state.set("glass-held", "closed"),
            _ => {}
        }
    }
}
