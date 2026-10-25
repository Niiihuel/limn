//! A small, original UI engine for desktop applications.
//!
//! Application code constructs an [`Element`] tree. [`UiTree`] lays it out,
//! retains interaction state by stable IDs, and emits backend-neutral drawing
//! commands. A host such as eframe translates pointer and keyboard events to
//! [`UiEvent`] and paints the resulting [`Frame`].
#![warn(missing_docs)]

#[cfg(test)]
mod captured_paste_tests;

mod accordion;
mod alert;
mod alert_dialog;
mod attachment;
mod avatar;
mod avatar_group;
mod badge;
mod breadcrumb;
mod button_group;
mod bottom_sheet;
mod card;
mod carousel;
mod caret_blink;
mod charts;
mod chat_minimap;
mod checkbox;
mod chip;
mod clipboard;
mod coach_mark;
mod code_block;
mod code_diff;
mod code_editor;
mod code_syntax;
mod collapse;
mod collapsible;
mod color_picker;
mod components;
mod composer;
mod context_window;
mod context_menu;
mod date_picker;
mod date_time_picker;
mod description_list;
mod diagram;
mod dictation;
mod dropdown_menu;
mod element;
mod empty;
mod file_icons;
mod find_bar;
mod flowchart;
mod form_field;
mod framed_card;
mod geometry;
mod diagram_viewer;
mod image_viewer;
mod list_row;
mod markdown;
mod markdown_cache;
mod markdown_parser;
mod marker;
mod media_card;
mod media_tile;
mod menu;
mod menu_bar;
mod fonts;
mod text_layout;
mod text_layout_cache;
mod window_controls;
mod menu_header;
mod modal;
mod navigation_bar;
mod motion;
mod notice;
mod number_input;
mod output_block;
mod theme;
mod page;
mod message_actions;
mod pager;
mod pagination;
mod panel_header;
mod peek_panel;
mod popover_panel;
mod preferences;
mod progress;
mod radio;
mod raster_image;
mod scheme;
mod scroll_area;
mod dismiss_drag;
mod select;
mod selector_sizing;
mod resize_handle;
mod separator;
mod sequence;
mod shortcut;
mod side_rail;
mod skeleton;
mod slider;
mod spinner;
mod status_bar;
mod stepper;
mod suggestion_list;
mod swatch_picker;
mod tab_strip;
mod table;
mod text;
mod text_shimmer;
mod text_width_cache;
mod ticker;
mod time_picker;
mod toast;
mod toggle;
mod toolbar;
mod top_bar;
mod access;
pub mod hot;
mod labels;
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

pub use accordion::{Accordion, AccordionItem};
pub use alert::{Alert, AlertColors, AlertTone};
pub use alert_dialog::AlertDialog;
pub use bottom_sheet::{BottomSheet, BottomSheetDrag};
pub use coach_mark::{CoachMark, Tour, TourStep};
pub use attachment::{Attachment, AttachmentOrientation};
pub use avatar::Avatar;
pub use avatar_group::AvatarGroup;
pub use badge::{Badge, BadgePlacement};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use button_group::{ButtonGroup, ButtonGroupSizing};
pub use card::{Card, CardDensity};
pub use carousel::Carousel;
pub use charts::{
    AreaChart, BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ChartSeries, ColumnChart, HeatmapCell, HeatmapChart,
    HeatmapScale, LineChart, PieChart, RadarChart,
};
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use code_block::{CodeBlock, CodeBlockSizing};
pub use code_diff::{CodeDiff, CodeDiffLine, CodeDiffLineKind};
pub use code_editor::{CaretStyle, CodeBreakpoint, CodeBreakpointState, CodeDiagnostic, CodeEditor, CodeExecutionLine, CodeFoldCandidate, CodeHighlight, CodeReviewHunk, CodeReviewHunkActions, CodeReviewPosition, CodeStickyScope, EditorController, VimMode};
pub use code_syntax::CodeLanguage;
pub use collapse::Collapse;
pub use collapsible::{collapse_footer, Collapsible};
pub use color_picker::{ColorPicker, Hsv};
pub use components::{
    ActiveHover, Button, ButtonContent, ButtonSize, ButtonSizing, ButtonVariant, Component,
    ComponentPalette, ControlSize, ControlState, DropdownList, DropdownOption, IconButton, Kbd, Link,
    Popover, Search, SearchField, SegmentedControl, Switch, SwitchVariant, Tab, Tabs, TabsVariant, TextArea, TextField,
    Tooltip,
    TooltipSpec,
};
pub use composer::Composer;
pub use context_window::{
    ContextItem, ContextShare, ContextShareAction, ContextTones, ContextWindow,
};
pub use framed_card::FramedCard;
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem};
pub use date_picker::DatePicker;
pub use date_time_picker::DateTimePicker;
pub use description_list::{DescriptionItem, DescriptionList};
pub use dictation::{DictationButton, DictationState, DictationWaveform};
pub use dropdown_menu::{
    DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownMenuSection,
};
pub use element::{
    Cursor,
    Border, BoxShadow, CoachAlign, CoachSide, BarPlot, BarSeries, BarValue, CornerRadii, Drawing, Element, ElementKind, FontFamily, HeatCell, HeatPlot, Icon, LevelBars,
    LinePlot,
    PiePlot, PieSlice, PlotSeries, RadarPlot,
    PopoverPlacement,
    SpinnerVariant, Style, TabSelection, TextHighlight, TextHint, TextSpan, InlineImage, INLINE_IMAGE_SCALE, TooltipItem, TooltipPlacement, TooltipSection,
    TooltipOptions, TooltipStyle, UniformList, MeasuredRows, UnitGrid, UnitGridPart, MarkRail, RailMark, DEFAULT_FONT_SIZE, ICON_SIZE, SMALL_ICON_SIZE,
};
pub use empty::Empty;
pub use form_field::{FormField, FormFieldSizing};
pub use diagram::{Diagram, DiagramError};
pub use flowchart::{
    FlowDirection, FlowEdge, FlowEdgeLayout, FlowEdgeLine, FlowGroup, FlowGroupLayout, FlowNode,
    FlowNodeLayout, FlowNodeShape, Flowchart, FlowchartError, FlowchartGraph, FlowchartLayout,
};
pub use find_bar::{FindBar, FindIds, FindLabels, FindOptions};
pub use file_icons::{file_icon, folder_icon, language_icon, FileIcon};
pub use geometry::{Color, Point, Rect, Size};
pub use diagram_viewer::DiagramViewer;
pub use image_viewer::ImageViewer;
pub use list_row::{DiffStat, ListRow, ListRowSizing};
pub use markdown::{Markdown, MarkdownColors, MarkdownLabels, MarkdownLimits, MarkdownPreview, MarkdownSourcePage};
pub use markdown_cache::MarkdownFrame;
pub use marker::{thinking_highlight, Marker, MarkerVariant};
pub use media_card::MediaCard;
pub use media_tile::MediaTile;
pub use menu::{MenuItem, MenuSeparator};
pub use menu_bar::{MenuBar, MenuBarMenu};
pub use fonts::{fonts, set_fonts, Font, FontError, Fonts};
pub use text_layout::{Glyph, GlyphRun, Leading, TextLayout, TextLine, TextStyle};
pub use window_controls::WindowControls;
pub use menu_header::MenuHeader;
pub use modal::{DismissButton, Modal};
pub use motion::{
    Easing, MenuMotion, ScalarSpring, ScalarTransition, ScrollMotion, SpringMotion,
};
pub use notice::{Notice, NoticeSizing};
pub use number_input::{NumberInput, NumberInputButtonPlacement};
pub use navigation_bar::{NavigationAction, NavigationBar, NavigationWorkspace};
pub use output_block::OutputBlock;
pub use page::{CardGrid, PageColumn, PageHeader, PageHeaderSizing, SectionHeader, SectionHeaderSizing};
pub use message_actions::{
    MessageAction, MessageActionDone, MessageActions, MessageActionsVisibility,
};
pub use pager::{Pager, PagerSizing};
pub use pagination::Pagination;
pub use panel_header::{PanelHeaderSizing, PanelHeader, SidebarPanel};
pub use peek_panel::{PeekPanel, Side};
pub use popover_panel::{PopoverPanel, PopoverPanelSizing};
pub use preferences::{ActionRow, ActionRowSizing, PreferencesGroup};
pub use progress::Progress;
pub use radio::{RadioGroup, RadioOption};
pub use raster_image::{RasterImage, RasterImageFormat};
pub use scheme::{ControlColors, Scheme, Surface};
pub use scroll_area::{ScrollArea, ScrollbarAppearance, ScrollbarPlacement, ScrollbarVisibility};
pub use dismiss_drag::DismissDrag;
pub use select::{Select, SelectVariant};
pub use selector_sizing::SelectorSizing;
pub use resize_handle::ResizeHandle;
pub use separator::Separator;
pub use side_rail::{SideRail, SideRailItem};
pub use sequence::{
    SequenceArrow, SequenceBlockKind, SequenceBlockLayout, SequenceDiagram, SequenceEvent, SequenceGraph,
    SequenceLayout, SequenceMessage, SequenceMessageLayout, SequenceNoteLayout, SequenceNotePlace,
    SequenceParticipant, SequenceParticipantLayout,
};
pub use skeleton::{Skeleton, SkeletonGroup};
pub use slider::Slider;
pub use spinner::Spinner;
pub use status_bar::{StatusBar, StatusBarItem, StatusBarItemSizing};
pub use chat_minimap::{ChatMinimap, ChatMinimapMark};
pub use clipboard::{Clipboard, ClipboardCopied};
pub use stepper::{Step, Stepper};
pub use suggestion_list::{SuggestionItem, SuggestionList};
pub use swatch_picker::{Swatch, SwatchPicker};
pub use tab_strip::{TabItem, TabStrip, TabStripHeader};
pub use table::{Table, TableColumn, TableRow, TableSizing, TableSort};
pub use text::Text;
pub use text_shimmer::TextShimmer;
pub use ticker::Ticker;
pub use time_picker::TimePicker;
pub use toast::Toast;
pub use toggle::{Toggle, ToggleGroup, ToggleItem, ToggleVariant};
pub use toolbar::{Toolbar, ToolbarItem};
pub use top_bar::TopBar;
pub use access::{Access, AccessNode, Role};
pub use labels::{labels, set_labels, Labels};
pub use tokens::{set_tokens, tokens, Motion, Radius, RadiusRole, ShortcutStyle, Tokens};
pub use theme::Theme;
pub use tree::{
    DrawCommand, FontMetrics, Frame, HitKind, HitRegion, ImeArea, Key, PasteTarget, TextLayoutBlock,
    TextLayoutRun, UiAction, UiEvent, UiTree, RECORD_REACH, RECORD_SHARE,
};
pub use tree_view::{guides_for_depths, TreeGuides, TreeRow};
pub use typography::{truncate_measured, wrap_measured, TextRole};

// Names these types had before each was called after its component.
#[allow(deprecated)]
pub use list_row::RowSizing;
#[allow(deprecated)]
pub use page::SectionSizing;
#[allow(deprecated)]
pub use panel_header::HeaderSizing;
