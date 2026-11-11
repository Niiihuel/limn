//! A small, original UI engine for desktop applications.
//!
//! Application code constructs an [`Element`] tree. [`UiTree`] lays it out,
//! retains interaction state by stable IDs, and emits backend-neutral drawing
//! commands. A host such as eframe translates pointer and keyboard events to
//! [`UiEvent`] and paints the resulting [`Frame`].
#![warn(missing_docs)]

#[cfg(test)]
mod seams_tests;

mod caret_blink;
mod code_diff;
mod code_editor;
mod code_syntax;
mod components;
mod context_menu;
mod diagram;
mod element;
mod file_icons;
mod flowchart;
mod geometry;
mod zoom_view;
mod fonts;
mod text_layout;
mod text_layout_cache;
mod motion;
mod raster_image;
mod scroll_area;
mod selector_sizing;
mod sequence;
#[doc(hidden)]
pub mod shortcut;
mod sketch;
mod scribble;
mod spinner;
mod text_width_cache;
mod access;
pub mod hot;
mod labels;
mod tokens;
mod tree;
mod typography;
pub use sheet_geometry::{SheetDetent, SheetSpec};
pub use code_diff::{CodeDiff, CodeDiffLine, CodeDiffLineKind, DiffColors, DiffFileHeader};
pub use code_editor::{CaretStyle, CodeBreakpoint, CodeBreakpointState, CodeChangeMark, CodeDiagnostic, CodeEditor, CodeExecutionLine, CodeFoldCandidate, CodeHighlight, CodeReviewHunk, CodeReviewHunkActions, CodeReviewPosition, CodeStickyScope, EditorController, VimMode};
pub use code_syntax::{CodeLanguage, CodeSyntaxColors};
pub use components::{
    ActiveHover, Button, ButtonContent, ButtonSize, ButtonSizing, ButtonVariant, Component,
    ComponentPalette, ControlSize, ControlState, IconButton, Kbd,
    Search, SearchField, SegmentedControl, Tab, Tabs, TabsVariant, TextArea, TextField,
    Tooltip,
    TooltipSpec,
};
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem};
pub use context_menu::{ContextMenuView, ContextMenuViewFn};
pub use element::{
    Cursor,
    Border, BoxShadow, CoachAlign, CoachSide, BarPlot, BarSeries, BarValue, CornerRadii, Drawing, Element, ElementKind, SurfaceFit, FontFamily, HeatCell, HeatPlot, Icon, LevelBars,
    LinePlot,
    PiePlot, PieSlice, PlotSeries, RadarPlot,
    PopoverPlacement,
    SpinnerVariant, Style, TabSelection, TextEffect, TextHighlight, TextHint, TextSpan, InlineImage, INLINE_IMAGE_SCALE, TooltipFace, TooltipItem, TooltipPlacement, TooltipSection, TOOLTIP_FACE, TOOLTIP_FACE_ICON, TOOLTIP_FACE_OVERLAP,
    TooltipOptions, TooltipShape, TooltipStyle, UniformList, MeasuredRows, UnitGrid, UnitGridPart, MarkRail, RailMark, DEFAULT_FONT_SIZE, ICON_SIZE, SMALL_ICON_SIZE,
    ZoomControl, ZoomView,
};
pub use element::{CoachAnchor, Ring};
pub use tree::{COACH_CARET, COACH_RING_GAP, COACH_RING_WIDTH};
pub use zoom_view::{zoom_percent, ZOOM_VIEW_MAX_SCALE};
pub use diagram::{Diagram, DiagramError};
pub use flowchart::{
    FlowDirection, FlowEdge, FlowEdgeLayout, FlowEdgeLine, FlowGroup, FlowGroupLayout, FlowNode,
    FlowNodeLayout, FlowNodeShape, Flowchart, FlowchartError, FlowchartGraph, FlowchartLayout,
};
pub use file_icons::{file_icon, folder_icon, language_icon, FileIcon};
pub use geometry::{Color, Point, Rect, Size};
#[doc(hidden)]
pub use geometry::{blend, contrast, px, readable_ink};
#[doc(hidden)]
pub mod action_style;
#[doc(hidden)]
pub use code_syntax::{highlight_spans, MAX_HIGHLIGHT_BYTES, MAX_HIGHLIGHT_LINES};
#[doc(hidden)]
pub use components::{
    clear_button, control_radius, darker, disabled_field, hover_ms, input_surface, lighter,
    surface_radius, CLEAR_SIDE,
};
#[doc(hidden)]
pub use diagram::MAX_SOURCE_BYTES;
#[doc(hidden)]
pub use sheet_geometry::{DIM_ALPHA, GRABBER, GRABBER_TOP};
#[doc(hidden)]
pub use tree::{slot_settle, TOOLTIP_HOVER_DELAY};
pub use fonts::{fonts, set_fonts, Font, FontError, Fonts};
pub use text_layout::{Glyph, GlyphRun, Leading, TextLayout, TextLine, TextStyle};
pub use motion::{
    Easing, IconKeyframe, IconMotion, IconTransform, MenuMotion, ScalarSpring, ScalarTransition, ScrollMotion, SpringMotion,
};
pub use raster_image::{RasterImage, RasterImageFormat};
pub use scroll_area::{ScrollArea, ScrollbarAppearance, ScrollbarPlacement, ScrollbarVisibility};
pub use scroll_area::ScrollbarTrack;
pub use selector_sizing::SelectorSizing;
pub use sequence::{
    SequenceArrow, SequenceBlockKind, SequenceBlockLayout, SequenceDiagram, SequenceEvent, SequenceGraph,
    SequenceLayout, SequenceMessage, SequenceMessageLayout, SequenceNoteLayout, SequenceNotePlace,
    SequenceParticipant, SequenceParticipantLayout,
};
pub use sketch::{rasterize as rasterize_sketch, Sketch, SketchMark, SketchTool};
pub use scribble::{Mark, Scribble};
pub use spinner::Spinner;
pub use access::{Access, AccessNode, Role};
pub use labels::{labels, set_labels, Labels};
/// A map hashed for speed, for the tables the layout fills and reads for
/// every element: SipHash, std's default, took a third of a long list's
/// layout. Seeded at random per process, since some keys are text from
/// outside, such as a chat's messages.
pub(crate) type FastMap<K, V> = std::collections::HashMap<K, V, foldhash::fast::RandomState>;
/// A set hashed as [`FastMap`] is.
pub(crate) type FastSet<T> = std::collections::HashSet<T, foldhash::fast::RandomState>;
pub use tokens::{set_tokens, tokens, with_tokens, Motion, Radius, RadiusRole, ShortcutStyle, Tokens};
pub use tree::{
    DrawCommand, FontMetrics, Frame, HitKind, HitRegion, ImeArea, Key, Modifiers, OtherButton, PasteTarget,
    TextLayoutBlock, TextLayoutRun, UiAction, UiEvent, UiTree, RECORD_REACH, RECORD_SHARE,
};
pub use typography::{truncate_measured, wrap_measured, TextRole};
mod glass;
mod liquid;
mod sheet_geometry;
pub use liquid::{GLASS_POP_CLOSE_MS, GLASS_POP_OPEN_MS};
pub use glass::GlassMaterial;
