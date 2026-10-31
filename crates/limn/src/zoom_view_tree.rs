//! The tree's side of [`Element::zoom_view`]: the picture laid out where its
//! zoom puts it, and the wheel, pinch, drag, double click, keys and controls
//! that move it. The geometry is in `zoom_view.rs`.

use super::*;
use crate::zoom_view::{view_in, zoom_percent, Bounds, ZoomState, GLIDE_WINDOW};
use crate::{SurfaceFit, ZoomControl, ZoomView};
use std::collections::hash_map::Entry;

/// What the last layout placed of a zoom view, for the events after it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ZoomFrame {
    pub(super) id: String,
    /// The box the picture zooms in.
    pub(super) room: Rect,
    /// The picture, where it was shown.
    pub(super) picture: Rect,
    pub(super) dismiss: Option<String>,
    /// A click on the picture toggles its zoom, and a press does not.
    pub(super) click_toggles: bool,
}

/// A press on a zoom view: a drag that pans it, or a click beside the
/// picture.
#[derive(Debug)]
pub(super) struct ZoomPress {
    id: String,
    origin: Point,
    last: Point,
    /// Where the pointer was lately, for the speed a release flings at.
    samples: VecDeque<(Instant, Point)>,
    dragging: bool,
    beside: bool,
    /// It was the second press of a double click, which zoomed.
    toggled: bool,
}

/// The scale's badge: its text size and its distance from the bottom.
const BADGE_FONT: f32 = 13.0;
const BADGE_HEIGHT: f32 = 28.0;
const BADGE_MARGIN: f32 = 16.0;
/// What Ctrl with an arrow moves the picture by, as a share of the box.
const KEY_PAN: f32 = 0.15;

fn zoomed(id: &str, state: &ZoomState) -> UiAction {
    UiAction::Zoomed {
        id: id.to_owned(),
        scale: state.view.scale,
        fitted: state.fitted,
    }
}

impl LayoutBuilder<'_> {
    pub(super) fn place_zoom_view(&mut self, view: &ZoomView, room: Rect, clip: Option<Rect>, shadow: Option<BoxShadow>) {
        let Some(visible) = clip.map_or(Some(room), |clip| room.intersection(clip)) else {
            return;
        };
        let turnable = matches!(view.picture.kind, ElementKind::RasterImage(_));
        // Without its size the picture fills the box at its proportions.
        let given = view.content.unwrap_or(Size::new(room.width.max(1.0), room.height.max(1.0)));
        let upscale = if view.content.is_some() { view.upscale } else { 1.0 };
        let size = Size::new(room.width, room.height);
        let (now, reduced) = (self.now, self.reduced_motion);
        // The control it was opened from is laid out beneath it by now. Only
        // a view that opens grows out of it: one that takes another's place,
        // as the next picture of a gallery does, finds that one still kept.
        let opener = view
            .opened_from
            .as_ref()
            .filter(|_| self.zoom_views.is_empty())
            .and_then(|id| self.frame.regions.iter().find(|region| &region.id == id))
            .map(|region| region.rect);
        let state = match self.zoom_views.entry(view.id.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let bounds = Bounds::new(size, given, upscale);
                let mut state = ZoomState::new(bounds, now);
                if let Some(from) = opener {
                    state.open_from(view_in(from, room, bounds.content), now, reduced);
                }
                entry.insert(state)
            }
        };
        state.seen = true;
        state.turnable = turnable;
        let turns = if turnable { state.turns } else { 0 };
        let content = if turns % 2 == 1 { Size::new(given.height, given.width) } else { given };
        state.resize(Bounds::new(size, content, upscale), now);
        let picture = state.shown(now).rect(state.bounds.content, room);
        let (scale, badge) = (state.view.scale, state.badge(now));
        self.add_region(&view.id, room, HitKind::Zoom, clip);
        self.frame.commands.push(DrawCommand::PushClip(visible));
        if let Some(shadow) = shadow {
            self.frame.commands.push(DrawCommand::Shadow { rect: picture, shadow, radius: view.picture.style.radius });
        }
        match &view.picture.kind {
            ElementKind::RasterImage(image) => {
                let image = if turns == 0 { image.clone() } else { image.clone().rotated(image.quarter_turns() + turns) };
                self.frame.commands.push(DrawCommand::RasterImage {
                    rect: picture,
                    image,
                    opacity: 1.0,
                    radius: view.picture.style.radius,
                });
            }
            // The rectangle has the surface's proportions: it fills it.
            ElementKind::Surface { id, label, mirrored, .. } => self.frame.commands.push(DrawCommand::Surface {
                rect: picture,
                id: id.clone(),
                label: label.clone(),
                opacity: 1.0,
                fit: SurfaceFit::Contain,
                radius: view.picture.style.radius,
                mirrored: *mirrored,
            }),
            _ => self.place(&view.picture, picture, Some(visible)),
        }
        if badge > 0.0 {
            let text = format!("{} %", zoom_percent(scale));
            let width = self.sans_width(&text, BADGE_FONT) + 24.0;
            let pill = Rect::new(
                room.x + (room.width - width) / 2.0,
                room.y + room.height - BADGE_HEIGHT - BADGE_MARGIN,
                width,
                BADGE_HEIGHT,
            );
            self.frame.commands.push(DrawCommand::FillRect {
                rect: pill,
                color: Color::rgba(0, 0, 0, (180.0 * badge) as u8),
                radius: BADGE_HEIGHT / 2.0,
            });
            let line = self.sans_metrics.at_size(BADGE_FONT).line_height;
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(pill.x + 12.0, (pill.y + (BADGE_HEIGHT - line) / 2.0).round()),
                text,
                color: Color::rgba(255, 255, 255, (255.0 * badge) as u8),
                font_size: BADGE_FONT,
                font_family: FontFamily::Sans,
            });
        }
        self.frame.commands.push(DrawCommand::PopClip);
        let name = match &view.picture.kind {
            ElementKind::RasterImage(image) => image.alt().to_owned(),
            ElementKind::Surface { label, .. } => label.clone(),
            _ => String::new(),
        };
        self.frame.access.push(crate::AccessNode {
            id: view.id.clone(),
            rect: visible,
            role: crate::Role::Image,
            name,
            description: Some(format!("{} %", zoom_percent(scale))),
            disabled: false,
            toggled: None,
            mixed: false,
            selected: None,
            expanded: None,
            value: None,
            value_text: None,
            actionable: false,
            focused: false,
        });
        for (control, command) in &view.controls {
            self.frame.zoom_controls.insert(control.clone(), (view.id.clone(), *command));
        }
        self.frame.zoom_views.push(ZoomFrame {
            id: view.id.clone(),
            room,
            picture,
            dismiss: view.dismiss.clone(),
            click_toggles: view.click_toggles,
        });
    }
}

impl UiTree {
    /// Where a zoom view keeps its picture: its scale, logical pixels on
    /// screen for each pixel of the picture, where it rests or is going to.
    /// `None` for an ID no zoom view was laid out under.
    pub fn zoom_scale(&self, id: &str) -> Option<f32> {
        self.zoom_views.get(id).map(|state| state.view.scale)
    }

    /// The wheel, a pinch, a press, a drag, a release or a key that a zoom
    /// view takes. `None` leaves the event to the rest of the tree.
    pub(super) fn handle_zoom_event(&mut self, event: &UiEvent, now: Instant) -> Option<Vec<UiAction>> {
        match *event {
            UiEvent::Scroll { position, delta_y } => self.zoom_wheel(position, 0.0, delta_y, false, now),
            UiEvent::ScrollDirect { position, delta_y } => self.zoom_wheel(position, 0.0, delta_y, true, now),
            UiEvent::Scroll2D { position, delta_x, delta_y, direct, .. } => {
                self.zoom_wheel(position, delta_x, delta_y, direct, now)
            }
            UiEvent::Zoom { position, factor } => {
                let frame = self.zoom_frame_under(position)?;
                let reduced = self.reduced_motion;
                let state = self.zoom_views.get_mut(&frame.id)?;
                let anchor = Point::new(position.x - frame.room.x, position.y - frame.room.y);
                let changed = state.zoom_at(anchor, factor, now, false, reduced);
                Some(changed.then(|| zoomed(&frame.id, state)).into_iter().collect())
            }
            UiEvent::PointerDown(position) | UiEvent::PointerDownModified { position, .. } => {
                self.zoom_press(position, 1, now)
            }
            UiEvent::PointerDownEditor { position, click_count, .. } => self.zoom_press(position, click_count, now),
            UiEvent::PointerMove(position) => {
                // The rest of the tree still follows the pointer.
                self.zoom_drag(position, now);
                None
            }
            UiEvent::PointerUp(position) => self.zoom_release(position, now),
            UiEvent::PointerLeave => {
                self.zoom_press = None;
                None
            }
            UiEvent::Key { key, shift, ctrl, alt } => self.zoom_key(key, shift, ctrl, alt, now),
            _ => None,
        }
    }

    /// The zoom view the wheel or a pinch at `point` reaches: the topmost
    /// one there in the surface on top, unless a scrolling area or a field
    /// lies over it.
    fn zoom_frame_under(&self, point: Point) -> Option<ZoomFrame> {
        let floor = self.floating_surface_floor(point);
        for region in self.frame.regions[floor..].iter().rev() {
            if !region.rect.contains(point) {
                continue;
            }
            match region.kind {
                HitKind::Zoom => return self.frame.zoom_views.iter().find(|frame| frame.id == region.id).cloned(),
                HitKind::Button | HitKind::Hover | HitKind::Surface => continue,
                _ => return None,
            }
        }
        None
    }

    fn zoom_wheel(&mut self, position: Point, dx: f32, dy: f32, direct: bool, now: Instant) -> Option<Vec<UiAction>> {
        if !(dx.is_finite() && dy.is_finite()) || (dx == 0.0 && dy == 0.0) {
            return None;
        }
        let frame = self.zoom_frame_under(position)?;
        let (ctrl, reduced) = (self.modifiers.ctrl, self.reduced_motion);
        let state = self.zoom_views.get_mut(&frame.id)?;
        let anchor = Point::new(position.x - frame.room.x, position.y - frame.room.y);
        let delta = if dy != 0.0 { dy } else { dx };
        // A mouse wheel zooms, notch by notch; a touchpad moves the picture,
        // and zooms with Ctrl, following the fingers. A view that toggles on
        // clicks has its wheel pan the picture, as a touchpad does.
        let pans = direct || (frame.click_toggles && !ctrl);
        let changed = if !pans {
            state.zoom_at(anchor, ZoomState::wheel_factor(delta), now, true, reduced)
        } else if ctrl {
            state.zoom_at(anchor, ZoomState::pixel_factor(delta), now, false, reduced)
        } else {
            state.pan(dx, dy, now);
            false
        };
        Some(changed.then(|| zoomed(&frame.id, state)).into_iter().collect())
    }

    fn zoom_press(&mut self, position: Point, clicks: u8, now: Instant) -> Option<Vec<UiAction>> {
        let region = self.frame.press_region(position)?;
        if region.kind != HitKind::Zoom {
            return None;
        }
        let frame = self.frame.zoom_views.iter().find(|frame| frame.id == region.id)?.clone();
        let reduced = self.reduced_motion;
        let state = self.zoom_views.get_mut(&frame.id)?;
        let on_picture = frame.picture.contains(position);
        let mut actions = Vec::new();
        // A click on a view that toggles on clicks is a release, not a press.
        let toggled = clicks >= 2 && on_picture && !frame.click_toggles;
        if toggled {
            let anchor = Point::new(position.x - frame.room.x, position.y - frame.room.y);
            if state.toggle_at(anchor, now, reduced) {
                actions.push(zoomed(&frame.id, state));
            }
        } else {
            state.hold(now);
        }
        self.zoom_press = Some(ZoomPress {
            id: frame.id,
            origin: position,
            last: position,
            samples: VecDeque::from([(now, position)]),
            dragging: false,
            beside: !on_picture,
            toggled,
        });
        self.pressed = None;
        self.keyboard_focus = false;
        self.pointer = Some(position);
        Some(actions)
    }

    fn zoom_drag(&mut self, position: Point, now: Instant) {
        let Some(press) = self.zoom_press.as_mut() else {
            return;
        };
        if !press.dragging
            && (position.x - press.origin.x).hypot(position.y - press.origin.y) < DRAG_THRESHOLD
        {
            return;
        }
        press.dragging = true;
        let (dx, dy) = (position.x - press.last.x, position.y - press.last.y);
        press.last = position;
        press.samples.push_back((now, position));
        while press.samples.len() > 2
            && press.samples.front().is_some_and(|(at, _)| now.saturating_duration_since(*at) > GLIDE_WINDOW)
        {
            press.samples.pop_front();
        }
        if let Some(state) = self.zoom_views.get_mut(&press.id) {
            state.pan(dx, dy, now);
        }
    }

    fn zoom_release(&mut self, position: Point, now: Instant) -> Option<Vec<UiAction>> {
        let press = self.zoom_press.take()?;
        let reduced = self.reduced_motion;
        if press.dragging {
            // The speed over the last moments; a pause before letting go
            // leaves nothing to fling.
            let recent = press
                .samples
                .iter()
                .find(|(at, _)| now.saturating_duration_since(*at) <= GLIDE_WINDOW)
                .copied();
            if let (Some((at, from)), Some(state)) = (recent, self.zoom_views.get_mut(&press.id)) {
                let seconds = now.saturating_duration_since(at).as_secs_f32();
                if seconds > 0.0 {
                    let velocity = Point::new((position.x - from.x) / seconds, (position.y - from.y) / seconds);
                    state.fling(velocity, now, reduced);
                }
            }
            return Some(Vec::new());
        }
        let frame = self.frame.zoom_views.iter().find(|frame| frame.id == press.id);
        // A click on the picture of a view that toggles on clicks zooms it
        // about the point let go on, as a lightbox does.
        if let Some(frame) = frame.filter(|frame| frame.click_toggles && !press.beside) {
            let anchor = Point::new(position.x - frame.room.x, position.y - frame.room.y);
            let state = self.zoom_views.get_mut(&press.id)?;
            let changed = state.toggle_at(anchor, now, reduced);
            return Some(changed.then(|| zoomed(&press.id, state)).into_iter().collect());
        }
        let dismiss = frame
            .and_then(|frame| frame.dismiss.clone())
            .filter(|_| press.beside && !press.toggled);
        Some(dismiss.map(|id| UiAction::Clicked { id }).into_iter().collect())
    }

    fn zoom_key(&mut self, key: Key, shift: bool, ctrl: bool, alt: bool, now: Instant) -> Option<Vec<UiAction>> {
        if alt {
            return None;
        }
        // A field takes the keys it types.
        let typing = self.focused.as_ref().is_some_and(|id| {
            self.focus_order.iter().any(|(candidate, kind)| {
                candidate == id && matches!(kind, HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor)
            })
        });
        if typing {
            return None;
        }
        let frame = self.frame.zoom_views.last()?.clone();
        let reduced = self.reduced_motion;
        let state = self.zoom_views.get_mut(&frame.id)?;
        let changed = match key {
            Key::Plus | Key::Equals => state.command(ZoomControl::In, now, reduced),
            Key::Minus => state.command(ZoomControl::Out, now, reduced),
            Key::Num0 => state.command(ZoomControl::Fit, now, reduced),
            Key::Num1 => state.scale_to(1.0, now, reduced),
            Key::Num2 => state.scale_to(2.0, now, reduced),
            Key::Num3 => state.scale_to(3.0, now, reduced),
            Key::R if ctrl => state.rotate(!shift, now),
            Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown if ctrl => {
                let (width, height) = (frame.room.width * KEY_PAN, frame.room.height * KEY_PAN);
                let (dx, dy) = match key {
                    Key::ArrowLeft => (width, 0.0),
                    Key::ArrowRight => (-width, 0.0),
                    Key::ArrowUp => (0.0, height),
                    _ => (0.0, -height),
                };
                state.pan(dx, dy, now);
                return Some(Vec::new());
            }
            _ => return None,
        };
        Some(changed.then(|| zoomed(&frame.id, state)).into_iter().collect())
    }

    /// A click on a zoom view's control also moves the view; its zoom is
    /// reported after the click.
    pub(super) fn apply_zoom_controls(&mut self, actions: &mut Vec<UiAction>, now: Instant) {
        let reduced = self.reduced_motion;
        let mut zooms = Vec::new();
        for action in actions.iter() {
            let UiAction::Clicked { id } = action else {
                continue;
            };
            let Some((view, control)) = self.frame.zoom_controls.get(id) else {
                continue;
            };
            if let Some(state) = self.zoom_views.get_mut(view) {
                if state.command(*control, now, reduced) {
                    zooms.push(zoomed(view, state));
                }
            }
        }
        actions.extend(zooms);
    }

    /// How soon a zoom view needs another frame.
    pub(super) fn zoom_repaint_after(&self, now: Instant) -> Option<Duration> {
        self.zoom_views.values().filter_map(|state| state.repaint_after(now)).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Modifiers, RasterImage, RasterImageFormat};
    use std::sync::Arc;

    const WINDOW: Size = Size::new(900.0, 620.0);
    /// The fit of a 2400 by 1600 picture in the window: 900 pixels wide.
    const FIT: f32 = 0.375;

    /// A window-sized zoom view of a 2400 by 1600 picture, dismissed by `close`.
    fn view(click_toggles: bool) -> UiTree {
        let still = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
        let mut view = Element::zoom_view("view", Element::raster_image(still))
            .zoom_content(Size::new(2400.0, 1600.0))
            .zoom_dismiss("close")
            .size(Some(WINDOW.width), Some(WINDOW.height));
        if click_toggles {
            view = view.zoom_click_toggles();
        }
        let mut tree = UiTree::new(view);
        tree.set_reduced_motion(true);
        tree.layout(WINDOW);
        tree
    }

    fn picture(frame: &Frame) -> Rect {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RasterImage { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("the picture is drawn")
    }

    fn zoomed(actions: &[UiAction]) -> Option<(f32, bool)> {
        actions.iter().find_map(|action| match action {
            UiAction::Zoomed { scale, fitted, .. } => Some((*scale, *fitted)),
            _ => None,
        })
    }

    /// A press and a release at `at`, as a click.
    fn click(tree: &mut UiTree, at: Point) -> Vec<UiAction> {
        tree.handle_event(UiEvent::PointerMove(at));
        tree.handle_event(UiEvent::PointerDown(at));
        tree.handle_event(UiEvent::PointerUp(at))
    }

    fn middle() -> Point {
        Point::new(450.0, 310.0)
    }

    #[test]
    fn a_click_zooms_two_and_a_half_times_about_the_point_and_the_next_fits_again() {
        let mut tree = view(true);
        let at = Point::new(300.0, 200.0);
        let fitted = picture(&tree.layout(WINDOW));
        let (scale, fitted_now) = zoomed(&click(&mut tree, at)).expect("a click zooms");
        assert!((scale - FIT * 2.5).abs() < 0.001 && !fitted_now);
        // The pixel under the pointer stays under it.
        let zoomed_in = picture(&tree.layout(WINDOW));
        let pixel = |rect: Rect| ((at.x - rect.x) / rect.width, (at.y - rect.y) / rect.height);
        let (before, after) = (pixel(fitted), pixel(zoomed_in));
        assert!((before.0 - after.0).abs() < 0.001 && (before.1 - after.1).abs() < 0.001);
        let (scale, fitted_again) = zoomed(&click(&mut tree, at)).expect("the next click fits");
        assert!((scale - FIT).abs() < 0.001 && fitted_again);
        assert_eq!(picture(&tree.layout(WINDOW)), fitted);
    }

    #[test]
    fn a_drag_pans_the_zoomed_picture_and_does_not_zoom_it() {
        let mut tree = view(true);
        let at = middle();
        click(&mut tree, at);
        let before = picture(&tree.layout(WINDOW));
        let to = Point::new(at.x + 60.0, at.y + 10.0);
        tree.handle_event(UiEvent::PointerMove(at));
        tree.handle_event(UiEvent::PointerDown(at));
        tree.handle_event(UiEvent::PointerMove(to));
        let actions = tree.handle_event(UiEvent::PointerUp(to));
        assert!(actions.is_empty(), "{actions:?}");
        let after = picture(&tree.layout(WINDOW));
        assert!((after.x - before.x - 60.0).abs() < 0.5 && (after.y - before.y - 10.0).abs() < 0.5);
    }

    #[test]
    fn the_wheel_pans_a_zoomed_picture_and_with_ctrl_zooms_it() {
        let mut tree = view(true);
        let at = middle();
        click(&mut tree, at);
        let scale = tree.zoom_scale("view").unwrap();
        let before = picture(&tree.layout(WINDOW));
        // A wheel notch up shows what is above: the picture moves down, its scale kept.
        assert!(zoomed(&tree.handle_event(UiEvent::Scroll { position: at, delta_y: 30.0 })).is_none());
        assert_eq!(tree.zoom_scale("view"), Some(scale));
        let panned = picture(&tree.layout(WINDOW));
        assert!((panned.y - before.y - 30.0).abs() < 0.5 && (panned.x - before.x).abs() < 0.5);
        tree.handle_event(UiEvent::ModifiersChanged(Modifiers { shift: false, ctrl: true, alt: false }));
        let (zoomed_scale, _) = zoomed(&tree.handle_event(UiEvent::Scroll { position: at, delta_y: 30.0 }))
            .expect("Ctrl with the wheel zooms");
        assert!((zoomed_scale - scale * 1.3).abs() < 0.001);
    }

    #[test]
    fn a_click_beside_the_picture_still_dismisses_it() {
        let mut tree = view(true);
        // Above the fitted picture, whose top is 10 pixels down.
        let beside = Point::new(450.0, 5.0);
        assert_eq!(click(&mut tree, beside), vec![UiAction::Clicked { id: "close".into() }]);
    }

    #[test]
    fn without_the_option_a_click_does_nothing_and_a_double_click_zooms() {
        let mut tree = view(false);
        let at = middle();
        assert!(click(&mut tree, at).is_empty());
        let double = UiEvent::PointerDownEditor { position: at, shift: false, alt: false, ctrl: false, click_count: 2 };
        let (scale, _) = zoomed(&tree.handle_event(double)).expect("a double click zooms");
        assert!((scale - FIT * 2.5).abs() < 0.001);
        tree.handle_event(UiEvent::PointerUp(at));
    }
}
