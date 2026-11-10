//! How the tree places containers: columns, rows, tab strips, overlays and
//! popovers.

use super::*;
use std::ops::ControlFlow;

impl LayoutBuilder<'_> {
    pub(super) fn place_column(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style) {
        let ElementKind::Column(children) = &element.kind else { unreachable!() };
        let outer = std::mem::replace(&mut self.flow_origin, Point::new(content.x, content.y));
        let kept = self.with_leaving(element, children);
        let previous_scope = matches!(&kept, Cow::Owned(_)).then(|| self.enter_measurement_scope());
        let children = kept.as_ref();
        if let Some(list) = element.kept.as_ref().and_then(|kept| kept.leaving.as_deref()) {
            let slots = children.iter().filter_map(Self::slot_id).map(str::to_owned).collect();
            *states_mut!(self).touch(list, self.now, || Leaving(Vec::new())) = Leaving(slots);
        }
        let viewport = self
            .scroll_focus_context
            .last()
            .filter(|_| children.iter().any(|child| child.sticky_top.is_some()))
            .map(|scroll| scroll.viewport);
        match viewport {
            Some(viewport) => {
                self.place_sticky_column(children, content, style.gap, clip, viewport)
            }
            None => {
                self.place_flex(children, content, style.gap, false, clip);
            }
        }
        if let Some(scope) = previous_scope { self.measurement_scope = scope; }
        self.flow_origin = outer;
    }

    pub(super) fn place_row(&mut self, element: &Element, content: Rect, clip: Option<Rect>) {
        let ElementKind::Row(children) = &element.kind else { unreachable!() };
        let outer = std::mem::replace(&mut self.flow_origin, Point::new(content.x, content.y));
        for (child, rect) in children
            .iter()
            .zip(self.child_rects(element, children, content))
        {
            self.place(child, rect, clip);
        }
        self.flow_origin = outer;
    }

    pub(super) fn place_tab_strip(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style) {
        let ElementKind::TabStrip {
            children,
            action_ids,
            selection,
            every_arrow,
        } = &element.kind else { unreachable!() };
        let command_start = self.frame.commands.len();
        let rects = self.place_flex(children, content, style.gap, true, clip);
        if let Some(selection) = selection {
            let index = children.iter().position(|child| match &child.kind {
                ElementKind::Button { id, .. } | ElementKind::IconButton { id, .. } => {
                    id == &selection.selected_id
                }
                _ => child.indicator_slot.as_ref() == Some(&selection.selected_id),
            });
            if let Some(mut target) = index.and_then(|index| rects.get(index)).copied() {
                if let Some(width) = selection.width {
                    if index == Some(children.len().saturating_sub(1)) { target.x += target.width - width; }
                    target.width = width;
                }
                if let Some(height) = selection.height {
                    target.y += (target.height - height) / 2.0;
                    target.height = height;
                }
                if selection.glass.is_some() && matches!(selection.selected_id.as_str(), "on" | "off") {
                    if let Some(id) = &selection.press_id {
                        self.frame.glass_switches.insert(id.clone(), GlassSwitchGeometry { bounds: rect, checked: selection.selected_id == "on" });
                    }
                }
                let mut states = states_mut!(self);
                let state = states.touch(&selection.group_id, self.now, || TabIndicatorState {
                        selected_id: selection.selected_id.clone(),
                        x: ScalarSpring::new(target.x, self.now),
                        width: ScalarSpring::new(target.width, self.now),
                        lift_x: ScalarSpring::new(0.0, self.now).motion(if selection.press_id.is_some() { crate::SpringMotion { stiffness: 900.0, damping_ratio: 0.7 } } else { crate::SpringMotion { stiffness: 250.0, damping_ratio: 0.60 } }),
                        lift_y: ScalarSpring::new(0.0, self.now).motion(if selection.press_id.is_some() { crate::SpringMotion { stiffness: 900.0, damping_ratio: 0.7 } } else { crate::SpringMotion { stiffness: 250.0, damping_ratio: 0.70 } }),
                        material_presence: ScalarSpring::new(0.0, self.now).motion(crate::SpringMotion { stiffness: 1000.0, damping_ratio: 1.0 }),
                        travel_from: target.x,
                        press_start: None,
                        dragging: false,
                    });
                // A lens of a strip of tabs, not a switch's thumb nor a segmented knob,
                // under a finger held on one of its tabs.
                let scrubbing = selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none()
                    && action_ids.iter().any(|id| self.pressed == Some(id.as_str()));
                if selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none() {
                    self.frame.glass_tabs.push(children.iter().zip(rects.iter()).filter_map(|(child, rect)| match &child.kind {
                        ElementKind::Button { id, .. } | ElementKind::IconButton { id, .. } | ElementKind::Clickable { id, .. } => Some((id.clone(), *rect)),
                        _ => None,
                    }).collect());
                }
                if self.reduced_motion || selection.animation_ms == 0 {
                    state.x.snap_to(target.x, self.now);
                    state.width.snap_to(target.width, self.now);
                    state.selected_id.clone_from(&selection.selected_id);
                } else if state.selected_id != selection.selected_id {
                    let duration = Duration::from_millis(u64::from(selection.animation_ms));
                    let motion = if selection.glass.is_some() && selection.press_id.is_some() {
                        crate::SpringMotion { stiffness: 700.0, damping_ratio: 0.78 }
                    } else if selection.glass.is_some() && selection.lift.is_some() {
                        aurora_spring(0.30, 0.15)
                    } else if selection.glass.is_some() {
                        crate::SpringMotion { stiffness: 280.0, damping_ratio: 0.938 }
                    } else { crate::SpringMotion::settling_in(duration) };
                    // Setting a pace before rebasing would redraw the
                    // flight so far on the new curve: a jump.
                    state.travel_from = state.x.value(self.now);
                    state.x.launch(target.x, motion, self.now);
                    state.width.launch(target.width, motion, self.now);
                    state.selected_id.clone_from(&selection.selected_id);
                } else if !scrubbing && (state.x.target() != target.x
                    || state.width.target() != target.width)
                {
                    // Resizing a dock or window moves the entire strip;
                    // only changing selection should leave a sliding trail.
                    if state.dragging {
                        // Let go off its tab, the lens travels back to it lifted, as it left.
                        state.travel_from = state.x.value(self.now);
                        state.x.launch(target.x, crate::SpringMotion { stiffness: 700.0, damping_ratio: 0.78 }, self.now);
                    } else { state.x.snap_to(target.x, self.now); }
                    state.width.snap_to(target.width, self.now);
                }
                let parent_pressed = selection.press_id.as_deref().is_some_and(|id| self.pressed == Some(id));
                if let (true, Some(pointer)) = (scrubbing, self.pointer) {
                    // iOS 26's tab bar: held, its lens goes under the finger and follows
                    // it along the bar, centred on it, until the finger lets go.
                    let left = rects.first().map_or(target.x, |rect| rect.x);
                    let right = rects.last().map_or(target.x, |rect| rect.x + rect.width - target.width);
                    state.dragging = true;
                    let goal = (pointer.x - target.width / 2.0).clamp(left, right.max(left));
                    if self.reduced_motion { state.x.snap_to(goal, self.now) } else { state.x.launch(goal, SCRUB_SPRING, self.now) }
                } else if selection.glass.is_some() && parent_pressed {
                    if let Some(pointer) = self.pointer {
                        let (press_x, start_x) = *state.press_start.get_or_insert((pointer.x, state.x.value(self.now)));
                        if (pointer.x - press_x).abs() > 3.0 {
                            state.dragging = true;
                            let left = rects.first().map_or(target.x, |rect| rect.x);
                            let right = rects.last().map_or(target.x, |rect| rect.x + rect.width - target.width);
                            state.x.snap_to((start_x + pointer.x - press_x).clamp(left, right), self.now);
                        }
                    }
                } else {
                    state.press_start = None;
                    state.dragging = false;
                }
                let x = state.x.value(self.now);
                let remaining = (target.x - x).abs();
                let distance = (target.x - state.travel_from).abs();
                let pressed = selection.press_id.as_deref().is_some_and(|id| self.pressed == Some(id))
                    || action_ids.iter().any(|id| self.pressed == Some(id.as_str()));
                // aurora's knob is held up while more than 45% of a segment is left
                // to go; BitChord's lens until 92% of the way.
                let lifting = selection.glass.is_some() && (pressed || match selection.lift {
                    Some(_) => remaining > 0.45 * target.width,
                    None => distance > 0.1 && remaining > distance * 0.08,
                });
                if self.reduced_motion || selection.glass.is_none() {
                    state.lift_x.snap_to(0.0, self.now);
                    state.lift_y.snap_to(0.0, self.now);
                    state.material_presence.snap_to(0.0, self.now);
                } else if selection.lift.is_some() {
                    // Lifted on the press spring, landed on the bouncy one.
                    let goal = if lifting { 1.0 } else { 0.0 };
                    if state.lift_x.target() != goal {
                        let motion = if lifting { aurora_spring(0.30, 0.35) } else { aurora_spring(0.42, 0.55) };
                        state.lift_x.launch(goal, motion, self.now);
                        state.lift_y.launch(goal, motion, self.now);
                    }
                    state.material_presence.retarget(goal, self.now);
                } else {
                    state.lift_x.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                    state.lift_y.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                    state.material_presence.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                }
                let lift_x = state.lift_x.value(self.now);
                let lift_y = state.lift_y.value(self.now);
                let presence = state.material_presence.value(self.now).clamp(0.0, 1.0);
                // A switch's thumb stretches as it is held. A tab's lens stands as tall as
                // the whole strip and LENS_LIFT more, in its cell's proportions, as
                // BitChord's tab bar's does while it travels.
                let (grow_x, grow_y) = if selection.press_id.is_some() {
                    (if pressed { 0.18 } else { 0.12 }, if pressed { 0.18 } else { -0.06 })
                } else if let Some(scale) = selection.lift {
                    (scale - 1.0, scale - 1.0)
                } else {
                    let grow = ((rect.height + LENS_LIFT) / target.height.max(1.0) - 1.0).max(0.0);
                    (grow, grow)
                };
                // A tab bar's lens dragged fast stretches along its way, thinner, as a drop.
                let stretch = if selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none() {
                    (state.x.velocity(self.now).abs() / SCRUB_STRETCH * 0.1).min(0.2)
                } else { 0.0 };
                let width = state.width.value(self.now) * (1.0 + lift_x * grow_x) * (1.0 + stretch);
                let height = target.height * (1.0 + lift_y * grow_y) * (1.0 - stretch / 2.0);
                let indicator = Rect::new(x - (width - state.width.value(self.now)) / 2.0,
                    target.y - (height - target.height) / 2.0, width, height);
                // Its corners grow with it: a capsule lifted is a capsule still, as Apple's lens.
                let radius = (selection.radius * indicator.height / target.height.max(1.0))
                    .min(indicator.width / 2.0).min(indicator.height / 2.0).max(0.0);
                let pad = selection.glass.map_or(0.0, |material| material.refraction.max(material.blur).max(target.height * 0.2));
                let local_rect = Rect::new(rect.x - pad, rect.y - pad, rect.width + pad * 2.0, rect.height + pad * 2.0);
                let command = match selection.glass {
                    Some(material) => DrawCommand::GlassSelection {
                        rect: indicator, radius, material, opacity: presence,
                        backdrop_rect: local_rect,
                        backdrop: self.frame.commands.iter().filter(|command| match command {
                            DrawCommand::PushClip(_) | DrawCommand::RoundClip { .. } | DrawCommand::PopClip => true,
                            _ => command.bounds().is_some_and(|bounds| bounds.intersection(local_rect).is_some()),
                        }).filter_map(|command| match command {
                            DrawCommand::GlassSelection { .. } => None,
                            DrawCommand::Glass { rect, radius, material, opacity } => Some(DrawCommand::FillRect {
                                rect: *rect, radius: *radius, color: Color { a: (material.tint.a as f32 * opacity) as u8, ..material.tint },
                            }),
                            _ => Some(command.clone()),
                        }).collect(),
                    },
                    None => DrawCommand::FillRect { rect: indicator, color: selection.color, radius },
                };
                if selection.glass.is_some() {
                    let color = Color { a: (selection.color.a as f32 * (1.0 - presence)).round() as u8, ..selection.color };
                    self.frame.commands.insert(command_start, DrawCommand::FillRect { rect: indicator, radius, color });
                    self.frame.commands.push(command);
                }
                else { self.frame.commands.insert(command_start, command); }
                if let Some(mut color) = selection.border {
                    if selection.glass.is_some() { color.a = (color.a as f32 * (1.0 - presence)).round() as u8; }
                    self.frame.commands.insert(
                        command_start + 1,
                        DrawCommand::StrokeRect { rect: indicator, color, radius, width: 1.0 },
                    );
                }
            }
        }
        self.frame.tab_focus.push(TabFocusScope {
            action_ids: action_ids.clone(),
            vertical: false,
            every_arrow: *every_arrow,
        });
    }

    pub(super) fn place_overlay(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style) {
        let ElementKind::Overlay { base, floating } = &element.kind else { unreachable!() };
        self.place(base, content, clip);
        if style.clip_children {
            // Local layers (for example a thumbnail's remove button)
            // keep their ancestor scroll clip and ordinary paint order.
            self.place(floating, content, clip);
        } else {
            self.overlays.push_back(DeferredOverlay::Free {
                floating: floating.as_ref().clone(),
                area: content,
            });
        }
    }

    /// `Break` where the arm returned: `place_inner` then returns at once,
    /// before anything that follows its `match`.
    pub(super) fn place_popover_anchor(&mut self, element: &Element, content: Rect, clip: Option<Rect>) -> ControlFlow<()> {
        let ElementKind::Popover {
            anchor,
            floating,
            dismiss_id,
            placement,
            gap,
            animation_ms,
            close_animation_ms,
            open_scale,
            morph,
            view_key,
            hover_open,
            backdrop,
            focus_scope,
            backdrop_dismiss,
            scrim,
            initial_focus,
            return_focus,
            coach,
            arrows,
            at,
            anchor_outset,
            clear_of,
            sheet,
        } = &element.kind else { unreachable!() };
        // A root wrapper receives the entire viewport, unlike a
        // child allocated by a flex parent. Honor its explicit anchor
        // constraints here too. Keep the entire anchor rectangle:
        // a composite header may contain focusable breadcrumbs, but
        // the first breadcrumb is not its positioning anchor.
        let anchor_rect = Rect::new(
            content.x,
            content.y,
            element
                .style
                .width
                .unwrap_or(content.width)
                .min(content.width)
                .max(0.0),
            element
                .style
                .height
                .unwrap_or(content.height)
                .min(content.height)
                .max(0.0),
        );
        self.place(anchor, anchor_rect, clip);
        let mut seen = crate::FastSet::default();
        let mut anchor_focus = Vec::new();
        collect_focus_order(anchor, &mut anchor_focus, &mut seen);
        let trigger_id = anchor_focus.first().map(|(id, _)| id.clone());
        let mut focusables = Vec::new();
        collect_focus_order(floating, &mut focusables, &mut seen);
        if *hover_open {
            if let Some(id) = &trigger_id {
                self.frame.submenu_triggers.insert(id.clone());
                if let Some((target, _)) =
                    focusables.iter().find(|(_, kind)| *kind == HitKind::Button)
                {
                    self.frame
                        .submenu_targets
                        .insert(id.clone(), target.clone());
                }
                // Returning to a parent with Left/Escape must not
                // immediately reopen the child merely because its
                // trigger regained focus. Right or pointer movement
                // into that trigger explicitly opens it again.
                if self.closed_submenu_triggers.contains(id) {
                    return ControlFlow::Break(());
                }
            }
            let hovered = self.hovered;
            let focused = self.focused;
            // A trigger the pointer pressed keeps the focus, and gets it back from a
            // modal it opened: that focus opens nothing, only the keyboard's does; the
            // pointer opens the card by resting on the trigger.
            let trigger_focused = focused.filter(|_| self.keyboard_focus);
            let surface_id = format!("popover_surface:{dismiss_id}");
            let active = trigger_id
                .as_deref()
                .is_some_and(|id| hovered == Some(id) || trigger_focused == Some(id))
                || hovered == Some(surface_id.as_str())
                || focusables.iter().any(|(id, _)| {
                    hovered == Some(id.as_str()) || focused == Some(id.as_str())
                })
                // What lies inside the trigger holds it too, such as a
                // hint's anchor in a row that opens a card on hover.
                || holds_submenu(anchor, hovered, trigger_focused)
                || holds_submenu(floating, hovered, focused)
                // The pointer crosses the space the trigger's outset and the gap leave
                // beside its rows, or beside the parent's surface, on the way in.
                || beside_trigger(
                    anchor_rect,
                    *anchor_outset,
                    *gap,
                    clear_of.as_deref().and_then(|id| surface_rect(&self.frame, id)),
                    self.pointer,
                );
            if !active {
                return ControlFlow::Break(());
            }
        }
        self.overlays.push_back(DeferredOverlay::Anchored {
            floating: floating.as_ref().clone(),
            anchor: at.map_or(anchor_rect, |point| Rect::new(point.x, point.y, 0.0, 0.0)),
            dismiss_id: dismiss_id.clone(),
            placement: *placement,
            gap: *gap,
            trigger_id,
            focusables,
            animation_ms: *animation_ms,
            close_animation_ms: *close_animation_ms,
            open_scale: *open_scale,
            morph: *morph,
            view_key: view_key.clone(),
            backdrop: *backdrop,
            focus_scope: *focus_scope,
            backdrop_dismiss: *backdrop_dismiss,
            scrim: *scrim,
            initial_focus: initial_focus.clone(),
            return_focus: return_focus.clone(),
            coach: coach.clone(),
            arrows: arrows.clone(),
            pointed: at.is_some(),
            anchor_outset: *anchor_outset,
            clear_of: clear_of.clone(),
            sheet: sheet.clone(),
        });
        ControlFlow::Continue(())
    }
}
