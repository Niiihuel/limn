//! A small, original UI engine for OpenIDE.
//!
//! Application code constructs an [`Element`] tree. [`UiTree`] lays it out,
//! retains interaction state by stable IDs, and emits backend-neutral drawing
//! commands. A host such as eframe translates pointer and keyboard events to
//! [`UiEvent`] and paints the resulting [`Frame`].

mod alert;
mod alert_dialog;
mod approval_request;
mod attachment;
mod avatar;
mod breadcrumb;
mod caret_blink;
mod charts;
mod chip;
mod code_diff;
mod code_editor;
mod code_syntax;
mod collapsible;
mod components;
mod composer;
mod context;
mod context_menu;
mod date_picker;
mod date_time_picker;
mod dropdown_menu;
mod element;
mod empty;
mod file_activity;
mod file_icons;
mod geometry;
mod image_viewer;
mod marker;
mod menu;
mod modal;
mod motion;
mod permission_selector;
mod preferences;
mod raster_image;
mod scroll_area;
mod selector_sizing;
mod shortcut;
mod skeleton;
mod spinner;
mod stepper;
mod text_shimmer;
mod text_width_cache;
mod time_picker;
mod tool_activity;
mod tree;
mod tree_view;
mod typography;

#[cfg(test)]
mod code_view_lifecycle_tests;
#[cfg(test)]
mod raw_text_area_tests;
#[cfg(test)]
mod link_presentation_tests;
#[cfg(test)]
mod text_measurement_tests;

pub use alert::{Alert, AlertColors, AlertTone};
pub use alert_dialog::AlertDialog;
pub use approval_request::ApprovalRequest;
pub use attachment::{Attachment, AttachmentOrientation};
pub use avatar::Avatar;
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use charts::{
    BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ColumnChart, HeatmapCell, HeatmapChart, HeatmapScale,
};
pub use chip::Chip;
pub use code_diff::{CodeDiff, CodeDiffLine, CodeDiffLineKind};
pub use code_editor::{CodeBreakpoint, CodeBreakpointState, CodeDiagnostic, CodeEditor, CodeExecutionLine, CodeFoldCandidate, CodeHighlight, CodeReviewHunk, CodeReviewHunkActions, CodeReviewPosition, CodeStickyScope, EditorController, VimMode};
pub use code_syntax::CodeLanguage;
pub use collapsible::Collapsible;
pub use components::{
    Button, ButtonContent, ButtonSize, ButtonSizing, ButtonVariant, Component, ComponentPalette,
    ControlSize, ControlState, DropdownList, DropdownOption, IconButton, Kbd, Popover, Search,
    SearchField, SegmentedControl, Switch, Tab, Tabs, TextArea, TextField, Tooltip,
};
pub use composer::Composer;
pub use context::{
    ContextContent, ContextMeter, ContextRow, ContextSegment, ContextUsage, ContextUsageRow,
};
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem};
pub use date_picker::DatePicker;
pub use date_time_picker::DateTimePicker;
pub use dropdown_menu::{DropdownMenu, DropdownMenuEntry, DropdownMenuItem};
pub use element::{
    Border, BoxShadow, CornerRadii, Element, ElementKind, FontFamily, Icon, PopoverPlacement,
    SpinnerVariant, Style, TabSelection, TextHighlight, TextSpan, TooltipItem, TooltipPlacement, TooltipSection,
    TooltipStyle, UniformList, DEFAULT_FONT_SIZE, ICON_SIZE,
};
pub use empty::Empty;
pub use file_activity::FileActivity;
pub use file_icons::{file_icon, folder_icon, language_icon, FileIcon};
pub use geometry::{Color, Point, Rect, Size};
pub use image_viewer::ImageViewer;
pub use marker::{Marker, MarkerVariant};
pub use menu::{MenuItem, MenuSeparator};
pub use modal::{DismissButton, Modal};
pub use motion::{
    Easing, MenuMotion, ScalarSpring, ScalarTransition, ScrollMotion, SpringMotion,
};
pub use permission_selector::{PermissionOption, PermissionSelector};
pub use preferences::{ActionRow, PreferencesGroup};
pub use raster_image::{RasterImage, RasterImageFormat};
pub use scroll_area::{ScrollArea, ScrollbarAppearance, ScrollbarPlacement};
pub use selector_sizing::SelectorSizing;
pub use skeleton::Skeleton;
pub use spinner::Spinner;
pub use stepper::{Step, Stepper};
pub use text_shimmer::TextShimmer;
pub use time_picker::TimePicker;
pub use tool_activity::ToolActivity;
pub use tree::{
    DrawCommand, FontMetrics, Frame, HitKind, HitRegion, Key, TextLayoutBlock, TextLayoutRun,
    UiAction, UiEvent, UiTree,
};
pub use tree_view::{guides_for_depths, TreeGuides, TreeRow};
pub use typography::{truncate_measured, wrap_measured, TextRole};
