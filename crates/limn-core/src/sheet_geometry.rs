//! iOS 26's sheet: the heights it rests at, the shape its height gives it, and where a
//! release sends it. Apple's sheet floats inset at partial heights, its corners concentric
//! with the display's, and anchors to the screen's edges, opaque, as it reaches its large
//! height, where what it was opened over recedes behind it.
//!
//! Sources: WWDC25 "Build a SwiftUI app with the new design" and "Get to know the new design
//! system"; the HIG's Sheets; UISheetPresentationController; WWDC18 "Designing Fluid
//! Interfaces" for the release projection. Apple publishes few numbers: the 8-point inset,
//! the 20% dimming and the large detent's 10-point gap were measured on iOS 26 simulators by
//! others, and the recession's scale is an estimate.

use crate::Rect;

/// How far a partial-height sheet stands in from the screen's sides and foot.
pub const FLOAT_INSET: f32 = 8.0;
/// The room the large height leaves under the safe area's top.
pub const LARGE_GAP: f32 = 10.0;
/// Black at 20%, behind a sheet at every height.
#[doc(hidden)]
pub const DIM_ALPHA: u8 = 51;
/// How small what the sheet was opened over gets at its large height.
pub const RECEDE_SCALE: f32 = 0.92;
/// UIScrollView's normal deceleration, which projects a release's speed into distance.
const DECELERATION: f32 = 0.998;
/// The grabber: a pill at the top of the sheet, as iOS draws it.
#[doc(hidden)]
pub const GRABBER: (f32, f32) = (36.0, 5.0);
/// How far the grabber stands under the sheet's top edge.
#[doc(hidden)]
pub const GRABBER_TOP: f32 = 5.0;

/// A height a sheet rests at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SheetDetent {
    /// As tall as what it holds, up to its large height.
    Fit,
    /// About half its large height.
    Medium,
    /// Its full height, under the safe area's top.
    Large,
    /// This many points, up to its large height.
    Height(f32),
}

/// What makes a bottom sheet iOS 26's: the heights it rests at, smallest first, where the
/// screen's safe area starts, the display's corner radius its corners are concentric with,
/// whether what it was opened over recedes at its large height, and whether a drag can send
/// it away.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetSpec {
    /// The heights it rests at, in any order.
    pub detents: Vec<SheetDetent>,
    /// Where the safe area starts at the screen's top: the large height stops `LARGE_GAP`
    /// under it.
    pub safe_top: f32,
    /// The display's corner radius. With one, a partial-height sheet floats inset with
    /// corners concentric with it; 0 keeps the sheet on the screen's edges at every height.
    pub screen_corner: f32,
    /// Whether what it was opened over recedes, smaller and rounded, at its large height.
    pub recede: bool,
    /// Whether a drag past its smallest height sends it away.
    pub dismissible: bool,
    /// The detent it opens at, an index into `detents`.
    pub opens_at: usize,
    /// How much of the screen's foot the keyboard covers: the sheet grows by it, its content
    /// kept above the keyboard and its glass reaching down behind it, as UIKit's does.
    pub keyboard: f32,
}

impl Default for SheetSpec {
    fn default() -> Self {
        Self { detents: vec![SheetDetent::Fit, SheetDetent::Large], safe_top: 0.0, screen_corner: 0.0, recede: true, dismissible: true, opens_at: 0, keyboard: 0.0 }
    }
}

/// Its largest height in a viewport this tall.
pub fn large(spec: &SheetSpec, viewport: f32) -> f32 {
    (viewport - spec.safe_top - LARGE_GAP).max(0.0)
}

/// The heights it rests at, ascending, for content `natural` points tall: two closer than a
/// point are one.
pub fn heights(spec: &SheetSpec, natural: f32, viewport: f32) -> Vec<f32> {
    let large = large(spec, viewport);
    let mut heights: Vec<f32> = spec.detents.iter().map(|detent| match detent {
        SheetDetent::Fit => natural,
        SheetDetent::Medium => large * 0.5,
        SheetDetent::Large => large,
        SheetDetent::Height(points) => *points,
    }.clamp(0.0, large)).collect();
    heights.sort_by(f32::total_cmp);
    heights.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    heights
}

/// How far toward its large, edge-anchored form a sheet `height` tall is: 0 up to three
/// quarters of `large`, where it still floats inset, easing to 1 at `large`.
pub fn anchored(height: f32, large: f32) -> f32 {
    if large <= 0.0 { return 1.0; }
    let t = ((height - large * 0.75) / (large * 0.25)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A sheet's shape at one height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// Where its content is laid out: it keeps the smallest resting height when it is taken
    /// lower, and goes down whole; it ends above a keyboard.
    pub rect: Rect,
    /// Where its surface stands: its content's, reaching on down behind a keyboard.
    pub shell: Rect,
    /// Its top corners' radius.
    pub top_radius: f32,
    /// Its bottom corners', which straighten as it anchors to the screen's foot.
    pub bottom_radius: f32,
    /// 0 floating glass, 1 anchored and opaque.
    pub anchored: f32,
}

/// The shape of a sheet `height` tall whose smallest resting height is `smallest`, in a
/// `width` by `viewport` screen. Over the keyboard `spec` names, it grows by the keyboard's
/// height, up to its large one, keeping its sides' inset.
pub fn shape(spec: &SheetSpec, height: f32, smallest: f32, width: f32, viewport: f32) -> Shape {
    let large = large(spec, viewport);
    let floats = spec.screen_corner > 0.0;
    let anchored = if floats { anchored(height, large) } else { 1.0 };
    let inset = FLOAT_INSET * (1.0 - anchored);
    let radius = if floats { (spec.screen_corner - FLOAT_INSET).max(10.0) } else { 12.0 };
    let keyboard = spec.keyboard.clamp(0.0, viewport);
    // Below its smallest height it is not squeezed: it is taken down as it is.
    let laid = height.max(smallest.min(large));
    let down = (laid - height).max(0.0);
    // Its foot: the screen's, behind a keyboard, else 8 over it while it floats.
    let foot = if keyboard > 0.0 { viewport } else { viewport - inset };
    let top = (foot - keyboard - laid + down).max(viewport - large.max(laid) + down);
    let side = (width - 2.0 * inset).max(0.0);
    // Over a keyboard its content ends above it and its glass goes on down behind it; with
    // none, both keep the height it was laid out at, taken down past the foot as they are.
    let (content, surface) = if keyboard > 0.0 { ((foot - keyboard - top).max(0.0), (foot - top).max(0.0)) } else { (laid, laid) };
    let rect = Rect::new(inset, top, side, content);
    let shell = Rect::new(inset, top, side, surface);
    Shape { rect, shell, top_radius: radius, bottom_radius: if floats { radius * (1.0 - anchored) } else { 0.0 }, anchored }
}

/// Past its largest height a sheet stretches as UIScrollView's content does past its end:
/// (1 − 1/(x·0.55/d + 1))·d, less the further it is pulled.
pub fn stretch(excess: f32, dimension: f32) -> f32 {
    if excess <= 0.0 || dimension <= 0.0 { return excess.max(0.0); }
    (1.0 - 1.0 / (excess * 0.55 / dimension + 1.0)) * dimension
}

/// The height a finger has taken a sheet to: all of the way between its resting heights,
/// a rubber band's past its largest.
pub fn followed(raw: f32, largest: f32, viewport: f32) -> f32 {
    if raw > largest { largest + stretch(raw - largest, viewport) } else { raw.max(0.0) }
}

/// Where a sheet let go at `height`, growing at `speed` points a second, comes to rest: the
/// resting height nearest where that speed would carry it, as UIKit projects a release; or
/// `None`, gone, when that is nearer to the foot of the screen and it may be sent away.
pub fn release(heights: &[f32], height: f32, speed: f32, dismissible: bool) -> Option<usize> {
    let projected = height + speed / 1000.0 * DECELERATION / (1.0 - DECELERATION);
    let away = |rest: f32| (projected - rest).abs();
    let nearest = (0..heights.len()).min_by(|a, b| away(heights[*a]).total_cmp(&away(heights[*b])));
    // Gone is one more resting place, at no height at all.
    if dismissible && nearest.is_none_or(|index| away(0.0) < away(heights[index])) { return None; }
    Some(nearest.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> SheetSpec {
        SheetSpec { safe_top: 47.0, screen_corner: 47.33, ..SheetSpec::default() }
    }

    #[test]
    fn a_partial_sheet_floats_inset_with_concentric_corners_and_anchors_at_its_large_height() {
        let spec = spec();
        let large = large(&spec, 844.0);
        assert_eq!(large, 787.0, "10 under the safe area's top");
        let half = shape(&spec, 300.0, 300.0, 390.0, 844.0);
        assert_eq!(half.rect, Rect::new(8.0, 844.0 - 8.0 - 300.0, 374.0, 300.0));
        assert!((half.top_radius - 39.33).abs() < 0.01 && half.bottom_radius == half.top_radius, "concentric with the display's 47.33");
        let full = shape(&spec, large, 300.0, 390.0, 844.0);
        assert_eq!(full.rect, Rect::new(0.0, 57.0, 390.0, large), "on the screen's edges, under the gap");
        assert_eq!((full.anchored, full.bottom_radius), (1.0, 0.0));
        // Still floating at three quarters, easing in after.
        assert_eq!(shape(&spec, large * 0.75, 300.0, 390.0, 844.0).rect.x, 8.0);
        let between = shape(&spec, large * 0.9, 300.0, 390.0, 844.0).rect.x;
        assert!(between > 0.0 && between < 8.0, "{between}");
        // Without a display radius it never floats.
        assert_eq!(shape(&SheetSpec::default(), 300.0, 300.0, 390.0, 844.0).rect.x, 0.0);
    }

    #[test]
    fn over_a_keyboard_it_grows_by_it_its_content_above_and_its_glass_behind() {
        let spec = SheetSpec { keyboard: 300.0, ..spec() };
        let raised = shape(&spec, 300.0, 300.0, 390.0, 844.0);
        assert_eq!(raised.rect, Rect::new(8.0, 844.0 - 300.0 - 300.0, 374.0, 300.0), "its content as tall as before, above the keyboard");
        assert_eq!(raised.shell, Rect::new(8.0, 244.0, 374.0, 600.0), "its glass on down behind the keyboard, its sides' inset kept");
        // A taller one stops at its large height: its content gives up what does not fit.
        let tall = shape(&spec, 700.0, 300.0, 390.0, 844.0);
        assert_eq!((tall.shell.y, tall.rect.height), (57.0, 844.0 - 300.0 - 57.0));
    }

    #[test]
    fn taken_below_its_smallest_height_a_sheet_goes_down_whole() {
        let spec = spec();
        let shape = shape(&spec, 200.0, 300.0, 390.0, 844.0);
        assert_eq!(shape.rect.height, 300.0, "not squeezed");
        assert_eq!(shape.rect.y, 844.0 - 8.0 - 300.0 + 100.0, "a hundred lower");
    }

    #[test]
    fn its_heights_are_its_detents_ascending_within_its_largest() {
        let spec = SheetSpec { detents: vec![SheetDetent::Large, SheetDetent::Fit, SheetDetent::Medium], ..spec() };
        assert_eq!(heights(&spec, 240.0, 844.0), vec![240.0, 393.5, 787.0]);
        // Content taller than the screen fits at the large height: one resting height.
        assert_eq!(heights(&SheetSpec::default(), 2000.0, 844.0), vec![844.0 - LARGE_GAP]);
    }

    #[test]
    fn a_release_rests_where_its_speed_would_carry_it_or_leaves() {
        let heights = [300.0, 787.0];
        // Let go still, it rests at the nearest height.
        assert_eq!(release(&heights, 600.0, 0.0, true), Some(1));
        assert_eq!(release(&heights, 400.0, 0.0, true), Some(0));
        // Flung up from near the smallest, it goes on to the large.
        assert_eq!(release(&heights, 350.0, 1200.0, true), Some(1));
        // Flung down, it leaves; still and low, too.
        assert_eq!(release(&heights, 280.0, -900.0, true), None);
        assert_eq!(release(&heights, 120.0, 0.0, true), None);
        // One that may not be sent away comes back.
        assert_eq!(release(&heights, 120.0, -900.0, false), Some(0));
    }

    #[test]
    fn past_its_largest_height_it_stretches_less_the_further_it_is_pulled() {
        assert_eq!(followed(500.0, 787.0, 844.0), 500.0, "one to one between its heights");
        let a = followed(787.0 + 100.0, 787.0, 844.0) - 787.0;
        let b = followed(787.0 + 200.0, 787.0, 844.0) - 787.0;
        assert!(a > 40.0 && a < 100.0 && b - a < a, "{a} then {b}");
    }
}
