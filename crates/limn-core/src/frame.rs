use super::*;

/// What a [`HitRegion`] belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HitKind {
    /// A code editor's viewport: it takes focus, text, the editor's pointer
    /// gestures and scrolling on both axes.
    CodeEditor,
    /// A read-only diff's viewport: it scrolls on both axes and hands vertical
    /// travel it cannot use to the scrolling areas around it.
    CodeDiff,
    /// Anything pressable: a press and a release on it report
    /// `UiAction::Clicked`. A slider uses it too.
    Button,
    /// A region that takes the keyboard focus on a press or Tab and reports
    /// nothing, such as a `TerminalView`: the host reads the keys while it
    /// has the focus. A press inside it still selects text.
    Focus,
    /// Read-only pointer target, such as a chart observation's tooltip.
    Hover,
    /// A single-line text field: a press focuses it and places the caret.
    TextInput,
    /// A multi-line text field: as `TextInput`, and the wheel scrolls its
    /// lines.
    TextArea,
    /// A non-activating floating surface that blocks backdrop dismissal.
    Surface,
    /// A scrolling area's viewport: the wheel over it moves its content. A
    /// press neither presses nor focuses it.
    Scroll,
    /// A scrollbar's rail: its thumb drags, its buttons step and a press on the
    /// rail pages.
    Scrollbar,
    /// A zoom view's box: the wheel and a pinch zoom it, a drag pans it and a
    /// double click zooms in or back out; see [`Element::zoom_view`].
    Zoom,
}

/// A rectangle of the frame that answers the pointer, with the ID of its
/// element. Of two regions under a point, the later one is on top.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    /// ID of the element the region belongs to.
    pub id: String,
    /// The part of the element that answers the pointer: its rectangle cut to
    /// the clip in force, relative to the frame's top-left corner.
    pub rect: Rect,
    /// What the tree does with a press, the wheel or focus here.
    pub kind: HitKind,
}

/// The result of one layout: what to paint, where the pointer lands and
/// what the platform is told.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Paint instructions in the order a backend carries them out: a later one
    /// covers an earlier one.
    pub commands: Vec<DrawCommand>,
    /// Pointer targets from bottom to top: of two under a point, the later one
    /// answers.
    pub regions: Vec<HitRegion>,
    /// What the platform should know of the frame: one node for each control
    /// and each thing a component described, in the order they were placed.
    pub access: Vec<crate::AccessNode>,
    pub(super) action_menu_items: HashSet<String>,
    /// Rows lit under a press alone: a press slid from one onto another chooses that one.
    pub(super) press_highlight_rows: HashSet<String>,
    /// The places in `regions` that are a popover's backdrop: under the pointer, they raise no
    /// tooltip, though they answer to the id of a control (an image viewer's close).
    pub(super) backdrops: Vec<usize>,
    pub(super) menu_secondary_actions: HashSet<String>,
    pub(super) draggables: HashSet<String>,
    pub(super) sketches: HashMap<String, (Rect, crate::sketch::SketchStyle)>,
    pub(super) cursors: HashMap<String, Cursor>,
    pub(super) drop_targets: HashSet<String>,
    /// The clickables that answer the pointer alone; see `Element::pointer_only`.
    pub(super) pointer_only: HashSet<String>,
    pub(super) submenu_targets: HashMap<String, String>,
    pub(super) submenu_triggers: HashSet<String>,
    pub(super) sliders: HashMap<String, SliderGeometry>,
    pub(super) glass_switches: HashMap<String, GlassSwitchGeometry>,
    /// The tabs of each strip whose glass lens a finger can drag along, as iOS 26's tab bar's:
    /// each tab's action and where it is.
    pub(super) glass_tabs: Vec<Vec<(String, Rect)>>,
    pub(super) scroll_extents: HashMap<String, f32>,
    pub(super) horizontal_viewports: HashMap<String, Rect>,
    /// Each button inside a `drag_to_pan` viewport, with the viewport a drag
    /// that starts on it pans.
    pub(super) pan_items: HashMap<String, String>,
    pub(super) scrollbars: HashMap<String, ScrollbarGeometry>,
    pub(super) scroll_motion: HashMap<String, ScrollMotion>,
    pub(super) scroll_focus_targets: HashMap<String, ScrollFocusTarget>,
    pub(super) scroll_focus_candidates: HashSet<String>,
    pub(super) text_blocks: Vec<SelectableTextBlock>,
    pub(super) popover_focus: Vec<PopoverFocusScope>,
    pub(super) tab_focus: Vec<TabFocusScope>,
    pub(super) inline_menu_focus: Vec<InlineMenuFocusScope>,
    pub(super) virtual_lists: HashMap<String, VirtualListFocus>,
    pub(super) context_menu_regions: Vec<ContextMenuRegion>,
    pub(super) coach: Option<CoachFrame>,
    /// The zoom views laid out, in order, and the controls that move them.
    pub(super) zoom_views: Vec<zoom_view_tree::ZoomFrame>,
    pub(super) zoom_controls: HashMap<String, (String, crate::ZoomControl)>,
    pub(super) duplicate_ids: Vec<String>,
    /// Where each floating layer's regions begin, in paint order: a popover,
    /// a free overlay, a context menu. The base tree is layer 0.
    pub(super) layers: Vec<usize>,
}

impl Frame {
    /// The IDs that more than one control of this layout took, in tree order:
    /// buttons, inputs, clickables and editors. Two controls under one ID share
    /// its hover, press, focus and transitions, and Tab stops at it once, so
    /// a test asserts this is empty. A link's action in formatted
    /// text is not counted: the same target may be linked twice.
    pub fn duplicate_ids(&self) -> &[String] {
        &self.duplicate_ids
    }

    /// Whether the control `id` was kept with its rectangle inside a scroll
    /// area, for focus to reveal it.
    #[doc(hidden)]
    pub fn has_scroll_focus_target(&self, id: &str) -> bool {
        self.scroll_focus_targets.contains_key(id)
    }

    /// Whether the control `id` was kept by its ID alone, out of sight in a
    /// scroll area, for focus to reach it.
    #[doc(hidden)]
    pub fn has_scroll_focus_candidate(&self, id: &str) -> bool {
        self.scroll_focus_candidates.contains(id)
    }

    /// The dismiss IDs of the popovers that hold keyboard focus, in the
    /// order their scopes were laid out.
    #[doc(hidden)]
    pub fn popover_focus_dismiss_ids(&self) -> Vec<String> {
        self.popover_focus.iter().map(|scope| scope.dismiss_id.clone()).collect()
    }

    /// Whether anything is painted over the host-fed surface `id` after it: a
    /// menu, a tooltip, a dialog, a panel sliding in. A host that shows a
    /// native view where the surface is laid out hides the view while this
    /// holds and lets the surface show its last frame instead, since nothing
    /// limn paints can cover a native view. Only what lands on the surface's
    /// visible part counts; `false` when no surface `id` was painted.
    pub fn drawn_over(&self, id: &str) -> bool {
        // The clip in force: `None` outside every clip, `Some(None)` inside one that shows nothing.
        let mut clips: Vec<Option<Rect>> = Vec::new();
        let mut surface: Option<Rect> = None;
        for command in &self.commands {
            let within = |rect: Rect| match clips.last() {
                None => Some(rect),
                Some(clip) => clip.and_then(|clip| rect.intersection(clip)),
            };
            match command {
                DrawCommand::PushClip(rect) | DrawCommand::RoundClip { rect, .. } => clips.push(within(*rect)),
                DrawCommand::PopClip => {
                    clips.pop();
                }
                DrawCommand::Surface { rect, id: drawn, opacity, .. } if drawn == id => {
                    surface = (*opacity > 0.0).then(|| within(*rect)).flatten();
                }
                other => {
                    let (Some(shown), Some(bounds)) = (surface, other.bounds()) else { continue };
                    if within(bounds).and_then(|bounds| bounds.intersection(shown)).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether the host-fed surface `id` was painted with some of it in view:
    /// laid out, not faded out, and not wholly clipped away by a scroll area
    /// or a panel around it. A host that decodes the surface's pixels, such
    /// as a video, decodes and repaints only while this holds.
    pub fn surface_shown(&self, id: &str) -> bool {
        let mut clips: Vec<Option<Rect>> = Vec::new();
        self.commands.iter().any(|command| {
            let within = |rect: Rect| match clips.last() {
                None => Some(rect),
                Some(clip) => clip.and_then(|clip| rect.intersection(clip)),
            };
            match command {
                DrawCommand::PushClip(rect) | DrawCommand::RoundClip { rect, .. } => {
                    let clip = within(*rect);
                    clips.push(clip);
                }
                DrawCommand::PopClip => {
                    clips.pop();
                }
                DrawCommand::Surface { rect, id: drawn, opacity, .. } if drawn == id => {
                    return *opacity > 0.0 && within(*rect).is_some();
                }
                _ => {}
            }
            false
        })
    }

    /// The target of a coach mark that was rendered but could not be placed:
    /// no control with that ID is laid out in view, so nothing was painted.
    /// The host moves on to another step.
    pub fn missing_coach_target(&self) -> Option<&str> {
        self.coach
            .as_ref()
            .filter(|coach| coach.missing)
            .map(|coach| coach.target.as_str())
    }
}

/// Read-only diagnostic copy of laid-out text, including wrapped runs and
/// their visible clip rectangles. Selection bookkeeping remains private.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayoutBlock {
    /// The block's whole text, before wrapping.
    pub source: String,
    /// The pieces `source` was laid out in.
    pub runs: Vec<TextLayoutRun>,
}

/// One run of a [`TextLayoutBlock`]: its text, its rectangle and its
/// visible clip rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayoutRun {
    /// The characters of this run.
    pub text: String,
    /// Where the run was laid out, before clipping.
    pub rect: Rect,
    /// The part of `rect` that is shown after clipping. A run with `None` does
    /// not answer the pointer.
    pub visible: Option<Rect>,
}

impl Frame {
    /// A copy of the selectable text blocks laid out in this frame, for
    /// diagnostics and tests.
    pub fn text_layouts(&self) -> Vec<TextLayoutBlock> {
        self.text_blocks
            .iter()
            .map(|block| TextLayoutBlock {
                source: block.source.clone(),
                runs: block
                    .runs
                    .iter()
                    .map(|run| TextLayoutRun {
                        text: run.text.clone(),
                        rect: run.rect,
                        visible: run.visible,
                    })
                    .collect(),
            })
            .collect()
    }

    /// Returns the topmost interactive element at a point.
    pub fn hit_test(&self, point: Point) -> Option<&str> {
        self.hit_region(point).map(|region| region.id.as_str())
    }

    /// The topmost region at a point, with its rectangle and kind. For a press
    /// use `press_region`, which looks through passive hover targets.
    pub fn hit_region(&self, point: Point) -> Option<&HitRegion> {
        self.region_at(point, false)
    }

    /// The region a press at `point` belongs to. A passive hover target, such
    /// as the anchor of a hint on an icon, shows its hint but lets the press
    /// reach the control it sits on.
    pub fn press_region(&self, point: Point) -> Option<&HitRegion> {
        self.region_at(point, true)
    }

    /// The layer of the topmost region at `point`, 0 for the base tree or
    /// none: the layers under it do not see the pointer.
    pub(super) fn layer_at(&self, point: Point) -> usize {
        self.regions
            .iter()
            .rposition(|region| region.rect.contains(point))
            .map_or(0, |index| self.layers.partition_point(|&start| start <= index))
    }

    fn region_at(&self, point: Point, pressing: bool) -> Option<&HitRegion> {
        for (index, region) in self.regions.iter().enumerate().rev() {
            if !region.rect.contains(point) {
                continue;
            }
            if pressing
                && region.kind == HitKind::Hover
                && self.regions[..index].iter().any(|underneath| {
                    underneath.rect.contains(point) && underneath.kind != HitKind::Hover
                })
            {
                continue;
            }
            if region.kind == HitKind::Scrollbar
                && self.scrollbars.get(&region.id).is_some_and(|bar| {
                    bar.placement == crate::ScrollbarPlacement::Overlay
                        && !bar.thumb.contains(point)
                        && bar.button_at(point).is_none()
                })
                && self.regions[..index].iter().any(|underneath| {
                    underneath.rect.contains(point)
                        && matches!(
                            underneath.kind,
                            HitKind::Button | HitKind::TextInput | HitKind::TextArea
                        )
                })
            {
                // A transparent overlay track should not swallow controls at
                // the content edge. The visible thumb still wins for dragging.
                continue;
            }
            return Some(region);
        }
        None
    }
}
