use super::*;

/// A bottom sheet's spring: back to its place when let go short, and to a new height when
/// what it holds changes; a little bounce, as iOS's sheets.
fn sheet_motion() -> crate::SpringMotion {
    aurora_spring(0.45, 0.12)
}

/// How deep an iOS sheet's rim bends what is behind it.
const SHEET_BEZEL: f32 = 12.0;
/// How far below its large height an iOS sheet starts to push back what it was opened over.
const RECEDE_FROM: f32 = 40.0;

/// What an iOS sheet was laid out as: the heights it rests at, ascending, the height it
/// stood at, the screen's height, and whether a drag may send it away.
#[derive(Clone, Debug)]
pub(super) struct SheetFrame {
    pub(super) heights: Vec<f32>,
    pub(super) resting: usize,
    pub(super) height: f32,
    viewport: f32,
    dismissible: bool,
}

/// How an iOS sheet is painted this frame: its shape; how much of its smallest height shows,
/// which the dimming follows; how far it goes to leave the screen; and how far it pushes
/// back what it was opened over.
#[derive(Clone, Copy, Debug)]
pub(super) struct SheetPaint {
    pub(super) rect: Rect,
    /// Its surface: its content's rectangle, reaching on down behind a keyboard.
    pub(super) shell: Rect,
    pub(super) entered: f32,
    pub(super) leave: f32,
    pub(super) recede: f32,
}

/// An iOS sheet held: by its grabber, from where the finger went down, or by its content
/// as that pans. `raw` is where the finger has taken it, before its rubber band; `speed`,
/// how fast it grows, in points a second.
#[derive(Clone, Debug)]
pub(super) struct SheetDrag {
    pub(super) dismiss: String,
    pub(super) grabber: Option<(String, f32, f32)>,
    raw: f32,
    pub(super) height: f32,
    speed: f32,
    last: Instant,
}

impl UiTree {
    /// Takes the iOS sheet `dismiss` where a finger has, `raw` points tall before its rubber
    /// band past its largest height; `grabber`: held by it, with the finger's first `y` and
    /// the sheet's height then.
    pub(super) fn move_sheet(&mut self, dismiss: &str, grabber: Option<(String, f32, f32)>, raw: f32, now: Instant) {
        let Some(frame) = self.sheet_frames.get(dismiss) else { return };
        let largest = frame.heights.last().copied().unwrap_or(frame.height);
        let height = crate::sheet_geometry::followed(raw, largest, frame.viewport);
        match self.sheet_drag.as_mut().filter(|drag| drag.dismiss == dismiss) {
            Some(drag) => {
                let seconds = now.saturating_duration_since(drag.last).as_secs_f32();
                if seconds > 0.004 {
                    drag.speed = drag.speed * 0.3 + (height - drag.height) / seconds * 0.7;
                }
                drag.raw = raw;
                drag.height = height;
                drag.last = now;
            }
            None => {
                // As UIKit's: taking the sheet first puts the keyboard away.
                let typing = self.focused.as_deref().is_some_and(|id| self.frame.regions.iter()
                    .any(|region| region.id == id && matches!(region.kind, HitKind::TextInput | HitKind::TextArea)));
                if typing {
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, None);
                }
                self.sheet_drag = Some(SheetDrag { dismiss: dismiss.to_owned(), grabber, raw, height, speed: 0.0, last: now });
            }
        }
    }

    /// Puts an iOS sheet's spring where a release or a cancel leaves it. The layout that
    /// holds the sheet keeps it from there.
    pub(super) fn set_sheet_spring(&mut self, dismiss: &str, value: ScalarSpring, now: Instant) {
        self.states.get_mut().touch(&format!("__sheet:{dismiss}"), now, || AnimatedSpring { value }).value = value;
    }

    /// Lets go of the iOS sheet held: its spring sets off from where the finger left it, at its
    /// speed, to the height that speed carries it nearest to; or it leaves, sliding on down
    /// rather than into the button it grew out of, which reports its dismissal.
    pub(super) fn let_go_sheet(&mut self, now: Instant) -> Option<UiAction> {
        let drag = self.sheet_drag.take()?;
        let frame = self.sheet_frames.get(&drag.dismiss)?;
        // A finger that stopped before letting go flicks nothing.
        let speed = if now.saturating_duration_since(drag.last) > Duration::from_millis(80) { 0.0 } else { drag.speed };
        let rest = crate::sheet_geometry::release(&frame.heights, drag.height, speed, frame.dismissible);
        self.set_sheet_spring(&drag.dismiss, ScalarSpring::moving(drag.height, speed, now), now);
        match rest {
            Some(index) => {
                self.sheet_detents.insert(drag.dismiss, index);
                None
            }
            None => {
                if let Some(snapshot) = self.popover_snapshots.get_mut(&drag.dismiss) {
                    snapshot.morph = None;
                }
                Some(UiAction::Clicked { id: drag.dismiss })
            }
        }
    }

    /// A direct pan over an iOS sheet takes the sheet where its content cannot go that way:
    /// down while the content is at its top, up while the sheet is short of its largest
    /// height. Back at its largest going up, the content scrolls again. Whether it took it.
    pub(super) fn pan_sheet(&mut self, position: Point, delta: f32, now: Instant) -> bool {
        if let Some(drag) = self.sheet_drag.as_ref().filter(|drag| drag.grabber.is_none()) {
            let dismiss = drag.dismiss.clone();
            let raw = drag.raw - delta;
            let Some(frame) = self.sheet_frames.get(&dismiss) else { return false };
            let largest = frame.heights.last().copied().unwrap_or(frame.height);
            if raw >= largest && delta < 0.0 {
                let last = frame.heights.len().saturating_sub(1);
                self.set_sheet_spring(&dismiss, ScalarSpring::new(largest, now), now);
                self.sheet_detents.insert(dismiss, last);
                self.sheet_drag = None;
                return false;
            }
            self.move_sheet(&dismiss, None, raw, now);
            return true;
        }
        if delta == 0.0 {
            return false;
        }
        // The surface under the finger, topmost first: only an iOS sheet's is taken.
        let Some((index, dismiss)) = self.frame.regions.iter().enumerate().rev()
            .filter(|(_, region)| region.rect.contains(position))
            .find_map(|(index, region)| region.id.strip_prefix("popover_surface:").map(|id| (index, id.to_owned())))
        else { return false };
        let Some(frame) = self.sheet_frames.get(&dismiss) else { return false };
        let largest = frame.heights.last().copied().unwrap_or(frame.height);
        // What scrolls under the finger, inside the sheet, and whether it is at its top.
        let at_top = self.frame.regions[index..].iter().rev()
            .find(|region| region.kind == HitKind::Scroll && region.rect.contains(position))
            .is_none_or(|region| self.scroll_offset(&region.id) <= 0.5);
        let takes = (delta > 0.0 && at_top) || (delta < 0.0 && frame.height < largest - 0.5);
        if takes {
            let raw = frame.height - delta;
            self.move_sheet(&dismiss, None, raw, now);
        }
        takes
    }
}

impl LayoutBuilder<'_> {
    /// Lays an iOS sheet out at the height a finger has taken it to or its spring stands at:
    /// `floating` takes the size, corners and material that height gives it. `morphs`: it
    /// grows out of the button that opened it, so it does not rise from the foot.
    pub(super) fn sheet_layout(&mut self, floating: &mut Element, spec: &crate::SheetSpec, dismiss_id: &str, viewport: Size, morphs: bool) -> SheetPaint {
        use crate::sheet_geometry as geometry;
        let floats = spec.screen_corner > 0.0;
        let width = (viewport.width - if floats { 2.0 * geometry::FLOAT_INSET } else { 0.0 }).max(0.0);
        floating.style.width = Some(width);
        floating.style.height = None;
        floating.style.max_height = None;
        let natural = self.measure(floating, Size::new(width, f32::INFINITY)).height;
        let heights = geometry::heights(spec, natural, viewport.height);
        let smallest = heights.first().copied().unwrap_or(natural);
        let resting = self.sheet_detents.get(dismiss_id).copied().unwrap_or(spec.opens_at).min(heights.len().saturating_sub(1));
        let target = heights.get(resting).copied().unwrap_or(natural);
        let key = format!("__sheet:{dismiss_id}");
        let height = match self.sheet_hold.filter(|(id, _)| *id == dismiss_id) {
            Some((_, held)) => {
                // The spring waits, with the speed the finger gives it, for the release.
                states_mut!(self).keep::<AnimatedSpring>(&key, self.now);
                held
            }
            None => {
                // First seen, it rises from the foot of the screen; grown out of a button, it
                // is there at once as the button's glass becomes it.
                if !morphs && !self.reduced_motion {
                    let now = self.now;
                    states_mut!(self).touch(&key, now, || AnimatedSpring { value: ScalarSpring::new(0.0, now) });
                }
                self.spring_moving(&key, target, sheet_motion())
            }
        };
        let shape = geometry::shape(spec, height, smallest, viewport.width, viewport.height);
        floating.style.width = Some(shape.rect.width);
        floating.style.height = Some(shape.rect.height);
        floating.style.corner_radii = Some(CornerRadii { top_left: shape.top_radius, top_right: shape.top_radius, bottom_right: shape.bottom_radius, bottom_left: shape.bottom_radius });
        // Glass and its shadow round all four corners alike: the screen's own corners hide the
        // foot's once the sheet anchors to it.
        floating.style.radius = shape.top_radius;
        // Glass at partial heights, growing opaque as it anchors to the screen's edges.
        if let Some(glass) = floating.kept.as_mut().and_then(|kept| kept.glass.as_mut()) {
            glass.tint.a = (f32::from(glass.tint.a) + (255.0 - f32::from(glass.tint.a)) * shape.anchored).round() as u8;
            // Its corners are round arcs, concentric with the display's, not a pane's squircle,
            // and it bends what is behind it only at its rim, not a corner's depth into it.
            glass.power = 2.0;
            glass.bezel = SHEET_BEZEL;
        }
        let large = geometry::large(spec, viewport.height);
        self.sheet_frames.insert(dismiss_id.to_owned(), SheetFrame { heights, resting, height, viewport: viewport.height, dismissible: spec.dismissible });
        // Only its large height pushes back what it was opened over.
        let recede = if spec.recede && large > 0.0 { ((height - (large - RECEDE_FROM)) / RECEDE_FROM).clamp(0.0, 1.0) } else { 0.0 };
        if recede > 0.0 {
            self.recede = Some((self.recede.map_or(0.0, |(deepest, _)| deepest).max(recede), spec.screen_corner));
        }
        SheetPaint {
            rect: shape.rect,
            shell: shape.shell,
            entered: if smallest > 0.0 { (height / smallest).clamp(0.0, 1.0) } else { 1.0 },
            leave: viewport.height - shape.shell.y + SHEET_EXIT_PAST,
            recede,
        }
    }

    /// How far a bottom sheet stands below its place `rect`: where the finger holding its
    /// handle has taken it, or where its spring is on its way back or to a new height. It
    /// stands in its place while it enters.
    pub(super) fn sheet_spring(&self, dismiss_id: &str, rect: Rect, entered: f32) -> f32 {
        let key = format!("__sheet:{dismiss_id}");
        let held = self.sheet_held.filter(|(id, _, _)| *id == dismiss_id);
        let rest = |top: f32, speed: f32| {
            let value = ScalarSpring::moving(top, speed, self.now);
            states_mut!(self).touch(&key, self.now, || AnimatedSpring { value }).value = value;
        };
        if let Some((_, down, speed)) = held {
            rest(rect.y + down, speed);
            return down;
        }
        if entered < 1.0 {
            rest(rect.y, 0.0);
            return 0.0;
        }
        self.spring_moving(&key, rect.y, sheet_motion()) - rect.y
    }
}
