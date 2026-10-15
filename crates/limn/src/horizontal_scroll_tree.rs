use super::*;

#[derive(Default)]
pub(super) struct HorizontalScrollState {
    offset: f32,
    viewport: f32,
    extent: f32,
    fade_width: f32,
    selected: Option<String>,
    focused: Option<String>,
    vertical_wheel: bool,
    drag_pan: bool,
}

impl UiTree {
    /// The `drag_to_pan` viewport a press at `point` would drag, with what
    /// to remember of the grab, unless something inside it takes the press.
    pub(super) fn horizontal_pan_at(&self, point: Point) -> Option<(String, f32)> {
        let region = self.frame.regions.iter().rev().find(|region| region.rect.contains(point))?;
        let state = self.horizontal_scrolls.get(&region.id).filter(|state| {
            region.kind == HitKind::Scroll && state.drag_pan && state.extent > state.viewport
        })?;
        Some((region.id.clone(), point.x + state.offset))
    }

    pub(super) fn drag_horizontal_pan(&mut self, point: Point) {
        if let Some((id, grab)) = &self.horizontal_pan {
            if let Some(state) = self.horizontal_scrolls.get_mut(id) {
                state.offset = (grab - point.x).clamp(0.0, (state.extent - state.viewport).max(0.0));
            }
        }
    }

    /// Where a `horizontal_scroll` currently sits, including any wheel or
    /// trackpad panning since the last layout.
    pub fn horizontal_scroll_offset(&self, id: &str) -> f32 {
        self.horizontal_scrolls
            .get(id)
            .map_or(0.0, |state| state.offset)
    }

    /// Position a `horizontal_scroll` from the host, for example from a
    /// spring. The next layout clamps it to the content.
    pub fn set_horizontal_scroll_offset(&mut self, id: &str, offset: f32) {
        if offset.is_finite() {
            self.horizontal_scrolls.entry(id.to_owned()).or_default().offset = offset;
        }
    }

    pub(super) fn handle_horizontal_scroll(&mut self, event: &UiEvent) -> bool {
        let (point, delta) = match event {
            UiEvent::Scroll { position, delta_y } | UiEvent::ScrollDirect { position, delta_y } => {
                (*position, *delta_y)
            }
            UiEvent::Scroll2D {
                position,
                delta_x,
                delta_y,
                ..
            } => (
                *position,
                if delta_x.abs() > delta_y.abs() {
                    *delta_x
                } else {
                    *delta_y
                },
            ),
            _ => return false,
        };
        if !delta.is_finite() || delta == 0.0 {
            return false;
        }
        // Floating menus own input before the dock beneath them.
        let modal = self
            .frame
            .popover_focus
            .iter()
            .rev()
            .find(|scope| scope.modal)
            .map(|scope| format!("popover_surface:{}", scope.dismiss_id));
        let floor = self
            .frame
            .regions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, region)| {
                (region.kind == HitKind::Surface
                    && region.id.starts_with("popover_surface:")
                    && (region.rect.contains(point)
                        || modal.as_deref() == Some(region.id.as_str())))
                .then_some(index)
            })
            .unwrap_or(0);
        let id = self
            .frame
            .regions
            .iter()
            .skip(floor)
            .rev()
            .find(|region| {
                region.kind == HitKind::Scroll
                    && region.rect.contains(point)
                    && self.frame.horizontal_viewports.contains_key(&region.id)
            })
            .map(|region| region.id.clone());
        let Some(id) = id else {
            return false;
        };
        let Some(state) = self.horizontal_scrolls.get_mut(&id) else {
            return false;
        };
        let (delta, propagate_vertical) = if state.vertical_wheel {
            (delta, false)
        } else {
            match event {
                UiEvent::Scroll2D {
                    delta_x,
                    delta_y,
                    shift,
                    ..
                } => {
                    if *shift && *delta_x == 0.0 {
                        (*delta_y, false)
                    } else {
                        (*delta_x, !*shift && *delta_y != 0.0)
                    }
                }
                _ => return false,
            }
        };
        if !delta.is_finite() || delta == 0.0 {
            return false;
        }
        state.offset = (state.offset - delta).clamp(0.0, (state.extent - state.viewport).max(0.0));
        self.pointer = Some(point);
        // Do not turn wheel motion over the tab bar into scrolling the chat.
        // A document trackpad gesture can move both independent axes at once.
        !propagate_vertical
    }
}

impl LayoutBuilder<'_> {
    fn horizontal_item_rect(&mut self, element: &Element, rect: Rect, id: &str) -> Option<Rect> {
        if focusable_id(element) == Some(id)
            || matches!(&element.kind, ElementKind::Tooltip { anchor_id, .. } if anchor_id == id)
        {
            return Some(rect);
        }
        let content = rect.inset(element.style.padding);
        match &element.kind {
            ElementKind::Row(children)
            | ElementKind::Column(children)
            | ElementKind::TabStrip { children, .. } => children
                .iter()
                .zip(self.child_rects(element, children, content))
                .find_map(|(child, rect)| self.horizontal_item_rect(child, rect, id)),
            ElementKind::Panel(child)
            | ElementKind::Clickable { child, .. }
            | ElementKind::Tooltip { anchor: child, .. }
            | ElementKind::Popover { anchor: child, .. } => {
                self.horizontal_item_rect(child, content, id)
            }
            ElementKind::Stack(children) => children.iter().find_map(|child| self.horizontal_item_rect(child, content, id)),
            _ => None,
        }
    }

    pub(super) fn paint_horizontal_scroll(
        &mut self,
        id: &str,
        child: &Element,
        reveal: Option<&str>,
        content: Rect,
        clip: Option<Rect>,
        fade: Option<(Color, f32)>,
        vertical_wheel: bool,
        pan_bar: Option<Color>,
        visibility: crate::ScrollbarVisibility,
    ) {
        self.add_region(id, content, HitKind::Scroll, clip);
        self.frame
            .horizontal_viewports
            .insert(id.to_owned(), content);
        let width = self
            .measure(child, Size::new(f32::INFINITY, content.height))
            .width
            .max(content.width);
        let base = Rect::new(0.0, 0.0, width, content.height);
        let focus = self.keyboard_focus.then_some(self.focused).flatten();
        let selected_rect =
            reveal.and_then(|target| self.horizontal_item_rect(child, base, target));
        let focus_rect = focus.and_then(|target| self.horizontal_item_rect(child, base, target));
        let fade_width = fade.map_or(0.0, |(_, width)| width.min(content.width / 2.0));
        let state = self.horizontal_scrolls.entry(id.to_owned()).or_default();
        let resized = state.viewport != content.width
            || state.extent != width
            || state.fade_width != fade_width;
        let selection_changed = state.selected.as_deref() != reveal;
        let focus_changed = state.focused.as_deref() != focus;
        let target = if focus_changed { focus_rect } else { None }.or_else(|| {
            if selection_changed {
                selected_rect
            } else if resized {
                focus_rect.or(selected_rect)
            } else {
                None
            }
        });
        if let Some(target) = target {
            let right = target.x + target.width;
            let margin = fade_width.min(((content.width - target.width) / 2.0).max(0.0));
            // Keep the revealed title and its close target clear of overflow
            // scrims whenever the viewport has room for both margins.
            if target.x < state.offset + margin {
                state.offset = target.x - margin;
            } else if right > state.offset + content.width - margin {
                state.offset = right + margin - content.width;
            }
        }
        state.offset = state.offset.clamp(0.0, (width - content.width).max(0.0));
        state.viewport = content.width;
        state.extent = width;
        state.fade_width = fade_width;
        state.selected = reveal.map(str::to_owned);
        state.focused = focus.map(str::to_owned);
        state.vertical_wheel = vertical_wheel;
        state.drag_pan = pan_bar.is_some();
        let offset = state.offset;
        let visible = clip.map_or(Some(content), |outer| outer.intersection(content));
        if let Some(visible) = visible {
            self.frame.commands.push(DrawCommand::PushClip(visible));
            self.place(
                child,
                Rect::new(content.x - offset, content.y, width, content.height),
                Some(visible),
            );
            if let Some((color, _)) = fade.filter(|_| fade_width > 0.0) {
                // A fixed small number of paint bands works in every native
                // renderer/exporter without adding hit regions or overlays.
                for left in [true, false] {
                    let hidden = if left {
                        offset
                    } else {
                        width - content.width - offset
                    };
                    if hidden <= 0.5 {
                        continue;
                    }
                    for band in 0..16 {
                        let fraction = (band as f32 + 0.5) / 16.0;
                        let opacity = if left { 1.0 - fraction } else { fraction };
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(
                                if left {
                                    content.x
                                } else {
                                    content.x + content.width - fade_width
                                } + band as f32 * fade_width / 16.0,
                                content.y,
                                fade_width / 16.0,
                                content.height,
                            ),
                            color: Color::rgba(
                                color.r,
                                color.g,
                                color.b,
                                (f32::from(color.a) * opacity).round() as u8,
                            ),
                            radius: 0.0,
                        });
                    }
                }
            }
            if let Some(color) = pan_bar.filter(|_| width > content.width + 0.5) {
                let opacity = self.scrollbar_opacity(id, offset, self.dragging_horizontal == Some(id), visibility);
                if opacity > 0.0 {
                    let scale = content.width / width;
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            content.x + offset * scale,
                            content.y + content.height - 3.0,
                            content.width * scale,
                            3.0,
                        ),
                        color: Color::rgba(color.r, color.g, color.b, (f32::from(color.a) * 0.45 * opacity) as u8),
                        radius: 1.5,
                    });
                }
            }
            self.frame.commands.push(DrawCommand::PopClip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_pointer_pan_preserves_touch_scrolling_across_later_presses() {
        let shelf = |width| Element::horizontal_scroll_2d(
            "shelf", Element::spacer().size(Some(width), Some(80.0)),
        ).drag_to_pan().size(Some(240.0), Some(80.0));
        let size = Size::new(240.0, 80.0);
        let mut tree = UiTree::new(shelf(1000.0));
        tree.layout(size);

        // A touch begins on the viewport itself, then the host takes over
        // after the pan slop and sends direct scroll deltas instead of pointer moves.
        tree.handle_event(UiEvent::PointerDown(Point::new(160.0, 40.0)));
        tree.handle_event(UiEvent::PointerLeave);
        tree.handle_event(UiEvent::Scroll2D {
            position: Point::new(160.0, 40.0), delta_x: -80.0, delta_y: 0.0,
            shift: false, direct: true,
        });
        tree.handle_event(UiEvent::PointerLeave);
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 80.0);

        // Playback repaints, a temporarily different route, and more loaded
        // content must retain the named shelf's position.
        tree.set_root(shelf(1000.0));
        tree.layout(size);
        tree.set_root(Element::text("Another route"));
        tree.layout(size);
        tree.set_root(shelf(1200.0));
        tree.layout(size);
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 80.0);

        // The emulated pointer moves before the next touch's down event.
        // It must not resume the cancelled drag from the previous gesture.
        tree.handle_event(UiEvent::PointerMove(Point::new(220.0, 40.0)));
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 80.0);
        tree.handle_event(UiEvent::PointerDown(Point::new(220.0, 40.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(180.0, 40.0)));
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 120.0);
        tree.handle_event(UiEvent::PointerUp(Point::new(180.0, 40.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 40.0)));
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 120.0);
    }

    #[test]
    fn rebuilding_anonymous_code_viewports_bounds_state_and_preserves_explicit_ids() {
        use crate::{CodeBlock, Component, ComponentPalette, Markdown};
        let palette = ComponentPalette::neutral_dark();
        let source = "```\nlong code line with enough text to scroll horizontally\n```\n\n```\nlong code line with enough text to scroll horizontally\n```";
        let root = || Element::column(vec![
            CodeBlock::new("long code line with enough text to scroll horizontally", palette)
                .id("stable-code").word_wrap(false).render(),
            CodeBlock::new("anonymous first block", palette).word_wrap(false).render(),
            CodeBlock::new("anonymous second block", palette).word_wrap(false).render(),
            Markdown::new(source, palette).width(140.0).code_word_wrap(false).render().element,
            Markdown::new(source, palette).id("stable-message").width(140.0)
                .code_word_wrap(false).render().element,
        ]);
        let mut tree = UiTree::new(Element::column(vec![
            Element::horizontal_scroll_2d("unrelated", Element::spacer().size(Some(1000.0), Some(20.0))),
            root(),
        ]));
        let size = Size::new(140.0, 900.0);
        let frame = tree.layout(size);
        let region = frame.regions.iter().find(|region| region.id == "stable-code").unwrap().rect;
        tree.handle_event(UiEvent::Scroll2D {
            position: Point::new(region.x + 10.0, region.y + 10.0),
            delta_x: -40.0, delta_y: 0.0, shift: false, direct: true,
        });
        tree.horizontal_scrolls.get_mut("unrelated").unwrap().offset = 80.0;
        let stable_message = "markdown:stable-message:code:0";
        tree.horizontal_scrolls.get_mut(stable_message).unwrap().offset = 20.0;
        for _ in 0..100 {
            tree.set_root(root());
            tree.layout(size);
            assert_eq!(tree.horizontal_scrolls.len(), 8, "anonymous viewport state must stay bounded");
            assert_eq!(tree.horizontal_scroll_offset("stable-code"), 40.0);
            assert_eq!(tree.horizontal_scroll_offset(stable_message), 20.0);
            assert_eq!(tree.horizontal_scroll_offset("unrelated"), 80.0);
        }
    }

    #[test]
    fn document_canvas_preserves_vertical_wheel_and_pans_each_trackpad_axis() {
        let canvas = |width| {
            Element::scroll(
                "document-y",
                Element::horizontal_scroll_2d(
                    "document-x",
                    Element::spacer()
                        .background(Color::WHITE)
                        .size(Some(width), Some(600.0)),
                )
                .size(Some(200.0), Some(600.0)),
            )
            .size(Some(200.0), Some(100.0))
        };
        let mut tree = UiTree::new(canvas(600.0));
        tree.set_reduced_motion(true);
        let size = Size::new(200.0, 100.0);
        let position = Point::new(50.0, 50.0);
        tree.layout(size);
        tree.handle_event(UiEvent::ScrollDirect {
            position,
            delta_y: -60.0,
        });
        tree.layout(size);
        assert_eq!(tree.scroll_offset("document-y"), 60.0);
        assert_eq!(tree.horizontal_scrolls["document-x"].offset, 0.0);
        for (delta_x, delta_y, shift, expected_x, expected_y) in [
            (0.0, -40.0, true, 40.0, 60.0),
            (-35.0, 0.0, false, 75.0, 60.0),
            (-25.0, -30.0, false, 100.0, 90.0),
        ] {
            tree.handle_event(UiEvent::Scroll2D {
                position,
                delta_x,
                delta_y,
                shift,
                direct: true,
            });
            tree.layout(size);
            assert_eq!(tree.horizontal_scrolls["document-x"].offset, expected_x);
            assert_eq!(tree.scroll_offset("document-y"), expected_y);
        }
        assert!(tree.frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if *color == Color::WHITE && rect.x == -100.0 && rect.y == -90.0
        )));
        // Fitting to width clears X without swallowing Y over a tall PDF page.
        tree.set_root(canvas(180.0));
        tree.layout(size);
        tree.handle_event(UiEvent::ScrollDirect {
            position,
            delta_y: -20.0,
        });
        tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["document-x"].offset, 0.0);
        assert_eq!(tree.scroll_offset("document-y"), 110.0);
    }

    fn strip(selected: usize) -> Element {
        Element::horizontal_scroll(
            "tabs",
            Element::row(
                (0..8)
                    .map(|i| {
                        Element::button(format!("tab:{i}"), format!("Archivo {i}"))
                            .size(Some(100.0), Some(32.0))
                    })
                    .collect(),
            ),
        )
        .reveal_horizontal_item(format!("tab:{selected}"))
        .size(None, Some(32.0))
    }

    #[test]
    fn tab_keyboard_navigation_reaches_and_reveals_offscreen_tabs() {
        let mut ids: Vec<_> = (0..8).map(|i| format!("tab:{i}")).collect();
        // A stale declaration must never become a focus destination.
        ids.insert(4, "removed-tab".into());
        let tabs = Element::tab_strip(
            (0..8)
                .map(|i| {
                    Element::button(format!("tab:{i}"), format!("Terminal {i}"))
                        .size(Some(100.0), Some(32.0))
                })
                .collect(),
            ids,
        )
        .size(Some(800.0), Some(32.0));
        let mut tree = UiTree::new(
            Element::horizontal_scroll("tabs", tabs)
                .horizontal_edge_fade(Color::rgb(20, 20, 20), 16.0)
                .size(None, Some(32.0)),
        );
        tree.set_reduced_motion(true);
        let size = Size::new(250.0, 32.0);
        tree.layout(size);
        assert!(tree.focus("tab:0"));
        for (key, index) in [
            (Key::End, 7),
            (Key::ArrowRight, 0),
            (Key::ArrowLeft, 7),
            (Key::Home, 0),
            (Key::ArrowRight, 1),
            (Key::ArrowRight, 2),
            (Key::ArrowRight, 3),
            (Key::ArrowRight, 4),
            (Key::ArrowRight, 5),
        ] {
            let id = format!("tab:{index}");
            let actions = tree.handle_event(UiEvent::Key {
                key,
                ctrl: false,
                shift: false,
            });
            assert!(
                actions.iter().any(|action| matches!(action,
                UiAction::Clicked { id: target } if target == &id)),
                "{key:?}: {actions:?}"
            );
            assert_eq!(tree.focused_id(), Some(id.as_str()));
            let frame = tree.layout(size);
            let region = frame.regions.iter().find(|region| region.id == id).unwrap();
            assert_eq!(
                region.rect.width, 100.0,
                "offscreen tab must be fully revealed"
            );
            assert!(region.rect.x >= 0.0 && region.rect.x + region.rect.width <= size.width);
        }
    }

    #[test]
    fn wheel_trackpad_selection_resize_and_keyboard_keep_tabs_reachable() {
        let mut tree = UiTree::new(strip(0));
        let size = Size::new(250.0, 32.0);
        tree.layout(size);
        let point = Point::new(120.0, 16.0);
        tree.handle_event(UiEvent::ScrollDirect {
            position: point,
            delta_y: -140.0,
        });
        let moved = tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 140.0);
        assert!(!moved.regions.iter().any(|r| r.id == "tab:0"));
        // Rebuilding the same selection cannot undo manual scrolling.
        tree.set_root(strip(0));
        tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 140.0);
        tree.handle_event(UiEvent::Scroll2D {
            position: point,
            delta_x: 40.0,
            delta_y: 0.0,
            direct: true,
            shift: false,
        });
        tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 100.0);
        tree.set_root(strip(7));
        let selected = tree.layout(size);
        let last = selected.regions.iter().find(|r| r.id == "tab:7").unwrap();
        assert_eq!(last.rect.width, 100.0);
        assert!(last.rect.x >= 0.0 && last.rect.x + last.rect.width <= size.width);
        tree.layout(Size::new(150.0, 32.0));
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 650.0);
        assert!(tree.focus("tab:0"));
        tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 0.0);
        // A resize must keep keyboard focus visible even before activation.
        let narrower = tree.layout(Size::new(150.0, 32.0));
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 0.0);
        assert!(narrower.regions.iter().any(|r| r.id == "tab:0"));
    }

    #[test]
    fn overflow_fades_are_conditional_and_never_take_pointer_ownership() {
        let color = Color::rgb(3, 7, 11);
        let faded = || strip(0).horizontal_edge_fade(color, 16.0);
        let mut tree = UiTree::new(faded());
        let size = Size::new(250.0, 32.0);
        let bands = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color: actual,
                        ..
                    } if actual.r == color.r
                        && actual.g == color.g
                        && actual.b == color.b
                        && actual.a < 255 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let first = tree.layout(size);
        let initial = bands(&first);
        assert_eq!(initial.len(), 16);
        assert!(initial.iter().all(|rect| rect.x >= size.width - 16.0));
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(120.0, 16.0),
            delta_y: -140.0,
        });
        let middle = tree.layout(size);
        assert_eq!(bands(&middle).len(), 32);
        let edge = Point::new(8.0, 16.0);
        assert_eq!(middle.hit_test(edge), Some("tab:1"));
        tree.handle_event(UiEvent::PointerDown(edge));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(edge)),
            vec![UiAction::Clicked { id: "tab:1".into() }]
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(120.0, 16.0),
            delta_y: -10_000.0,
        });
        let last = tree.layout(size);
        assert_eq!(bands(&last).len(), 16);
        assert!(bands(&last).iter().all(|rect| rect.x < 16.0));
        let wide = tree.layout(Size::new(900.0, 32.0));
        assert!(bands(&wide).is_empty());
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, 0.0);
    }

    #[test]
    fn active_and_keyboard_reveal_leave_the_fade_margin_and_preserve_manual_scroll() {
        let color = Color::rgb(3, 7, 11);
        let mut tree = UiTree::new(strip(4).horizontal_edge_fade(color, 16.0));
        let check = |frame: &Frame, id: &str, width: f32| {
            let rect = frame
                .regions
                .iter()
                .find(|region| region.id == id)
                .unwrap()
                .rect;
            assert_eq!(rect.width, 100.0);
            assert!(
                rect.x >= 16.0 && rect.x + rect.width <= width - 16.0,
                "{id}: {rect:?}"
            );
        };
        let size = Size::new(250.0, 32.0);
        check(&tree.layout(size), "tab:4", size.width);
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(120.0, 16.0),
            delta_y: -30.0,
        });
        tree.layout(size);
        let manual = tree.horizontal_scrolls["tabs"].offset;
        tree.set_root(strip(4).horizontal_edge_fade(color, 16.0));
        tree.layout(size);
        assert_eq!(tree.horizontal_scrolls["tabs"].offset, manual);
        assert!(tree.focus("tab:6"));
        check(&tree.layout(size), "tab:6", size.width);
        check(&tree.layout(Size::new(150.0, 32.0)), "tab:6", 150.0);
    }

    #[test]
    fn host_positions_the_viewport_and_layout_clamps_it() {
        let wide = Element::row(vec![
            Element::button("a", "a").size(Some(200.0), Some(20.0)),
            Element::button("b", "b").size(Some(200.0), Some(20.0)),
        ]);
        let mut tree = UiTree::new(Element::horizontal_scroll_2d("strip", wide));
        let size = Size::new(300.0, 20.0);
        assert_eq!(tree.layout(size).hit_test(Point::new(10.0, 10.0)), Some("a"));
        tree.set_horizontal_scroll_offset("strip", 60.0);
        tree.layout(size);
        assert_eq!(tree.horizontal_scroll_offset("strip"), 60.0);
        tree.set_horizontal_scroll_offset("strip", 9000.0);
        let frame = tree.layout(size);
        assert_eq!(tree.horizontal_scroll_offset("strip"), 100.0);
        assert_eq!(frame.hit_test(Point::new(110.0, 10.0)), Some("b"));
    }
}
