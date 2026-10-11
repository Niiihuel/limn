//! A small, original UI engine for desktop applications.
//!
//! Application code constructs an [`Element`] tree. [`UiTree`] lays it out,
//! retains interaction state by stable IDs, and emits backend-neutral drawing
//! commands. A host such as eframe translates pointer and keyboard events to
//! [`UiEvent`] and paints the resulting [`Frame`].

mod alert;
mod alert_dialog;
mod attachment;
mod avatar;
mod breadcrumb;
mod button_group;
mod card;
mod caret_blink;
mod charts;
mod chip;
mod coach_mark;
mod code_block;
mod code_diff;
mod code_editor;
mod code_syntax;
mod collapse;
mod collapsible;
mod components;
mod composer;
mod context_menu;
mod date_picker;
mod date_time_picker;
mod dropdown_menu;
mod element;
mod empty;
mod file_icons;
mod find_bar;
mod form_field;
mod geometry;
mod image_viewer;
mod list_row;
mod markdown;
mod markdown_cache;
mod markdown_parser;
mod marker;
mod menu;
mod menu_bar;
mod menu_header;
mod modal;
mod motion;
mod notice;
mod output_block;
mod theme;
mod page;
mod pager;
mod panel_header;
mod popover_panel;
mod preferences;
mod progress;
mod raster_image;
mod scheme;
mod scroll_area;
mod select;
mod selector_sizing;
mod separator;
mod shortcut;
mod skeleton;
mod spinner;
mod status_bar;
mod stepper;
mod suggestion_list;
mod tab_strip;
mod text;
mod text_shimmer;
mod text_width_cache;
mod ticker;
mod time_picker;
mod toast;
mod tokens;
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
pub use coach_mark::{CoachMark, Tour, TourStep};
pub use attachment::{Attachment, AttachmentOrientation};
pub use avatar::Avatar;
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use button_group::{ButtonGroup, ButtonGroupSizing};
pub use card::{Card, CardDensity};
pub use charts::{
    BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ColumnChart, HeatmapCell, HeatmapChart, HeatmapScale,
};
pub use chip::Chip;
pub use code_block::{CodeBlock, CodeBlockSizing};
pub use code_diff::{CodeDiff, CodeDiffLine, CodeDiffLineKind};
pub use code_editor::{CodeBreakpoint, CodeBreakpointState, CodeDiagnostic, CodeEditor, CodeExecutionLine, CodeFoldCandidate, CodeHighlight, CodeReviewHunk, CodeReviewHunkActions, CodeReviewPosition, CodeStickyScope, EditorController, VimMode};
pub use code_syntax::CodeLanguage;
pub use collapse::Collapse;
pub use collapsible::{collapse_footer, Collapsible};
pub use components::{
    ActiveHover, Button, ButtonContent, ButtonSize, ButtonSizing, ButtonVariant, Component,
    ComponentPalette, ControlSize, ControlState, DropdownList, DropdownOption, IconButton, Kbd, Link,
    Popover, Search, SearchField, SegmentedControl, Switch, Tab, Tabs, TextArea, TextField, Tooltip,
    TooltipSpec,
};
pub use composer::Composer;
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem};
pub use date_picker::DatePicker;
pub use date_time_picker::DateTimePicker;
pub use dropdown_menu::{
    DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownMenuSection,
};
pub use element::{
    Border, BoxShadow, CoachAlign, CoachSide, CornerRadii, Element, ElementKind, FontFamily, Icon, PopoverPlacement,
    SpinnerVariant, Style, TabSelection, TextHighlight, TextSpan, TooltipItem, TooltipPlacement, TooltipSection,
    TooltipStyle, UniformList, DEFAULT_FONT_SIZE, ICON_SIZE,
};
pub use empty::Empty;
pub use form_field::{FormField, FormFieldSizing};
pub use find_bar::{FindBar, FindIds, FindLabels, FindOptions};
pub use file_icons::{file_icon, folder_icon, language_icon, FileIcon};
pub use geometry::{Color, Point, Rect, Size};
pub use image_viewer::ImageViewer;
pub use list_row::{DiffStat, ListRow, RowSizing};
pub use markdown::{Markdown, MarkdownColors, MarkdownLabels, MarkdownLimits, MarkdownPreview, MarkdownSourcePage};
pub use markdown_cache::MarkdownFrame;
pub use marker::{thinking_highlight, Marker, MarkerVariant};
pub use menu::{MenuItem, MenuSeparator};
pub use menu_bar::{MenuBar, MenuBarMenu};
pub use menu_header::MenuHeader;
pub use modal::{DismissButton, Modal};
pub use motion::{
    Easing, MenuMotion, ScalarSpring, ScalarTransition, ScrollMotion, SpringMotion,
};
pub use notice::{Notice, NoticeSizing};
pub use output_block::OutputBlock;
pub use page::{CardGrid, PageColumn, PageHeader, PageHeaderSizing, SectionHeader, SectionSizing};
pub use pager::{Pager, PagerSizing};
pub use panel_header::{HeaderSizing, PanelHeader, SidebarPanel};
pub use popover_panel::{PopoverPanel, PopoverPanelSizing};
pub use preferences::{ActionRow, ActionRowSizing, PreferencesGroup};
pub use progress::Progress;
pub use raster_image::{RasterImage, RasterImageFormat};
pub use scheme::{ControlColors, Scheme, Surface};
pub use scroll_area::{ScrollArea, ScrollbarAppearance, ScrollbarPlacement};
pub use select::Select;
pub use selector_sizing::SelectorSizing;
pub use separator::Separator;
pub use skeleton::{Skeleton, SkeletonGroup};
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusBarItem, StatusBarItemSizing};
pub use stepper::{Step, Stepper};
pub use suggestion_list::{SuggestionItem, SuggestionList};
pub use tab_strip::{TabItem, TabStrip, TabStripHeader};
pub use text::Text;
pub use text_shimmer::TextShimmer;
pub use ticker::Ticker;
pub use time_picker::TimePicker;
pub use toast::Toast;
pub use tokens::{set_tokens, tokens, Motion, Radius, RadiusRole, Tokens};
pub use theme::Theme;
pub use tree::{
    DrawCommand, FontMetrics, Frame, HitKind, HitRegion, Key, TextLayoutBlock, TextLayoutRun,
    UiAction, UiEvent, UiTree,
};
pub use tree_view::{guides_for_depths, TreeGuides, TreeRow};
pub use typography::{truncate_measured, wrap_measured, TextRole};
