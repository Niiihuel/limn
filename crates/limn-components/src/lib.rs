//! The design that ships with limn: every component with a look of its own,
//! built from what [`limn_core`] makes public. An application that wants a
//! design of its own builds on `limn-core` alone and leaves this crate out.
#![warn(missing_docs)]

// Components name the engine's types as `crate::Element`, as they did when
// both lived in one crate.
#[allow(unused_imports)]
use limn_core::*;

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
mod charts;
mod chat_minimap;
mod checkbox;
mod chip;
mod clipboard;
mod coach_mark;
mod code_block;
mod collapse;
mod collapsible;
mod color_picker;
mod composer;
mod context_window;
mod date_picker;
mod date_time_picker;
mod description_list;
mod dictation;
mod dropdown_menu;
mod empty;
mod find_bar;
mod form_field;
mod framed_card;
mod diagram_viewer;
mod image_viewer;
mod link;
mod list_row;
mod markdown;
mod markdown_cache;
mod markdown_parser;
mod marker;
mod media_card;
mod media_tile;
mod menu;
mod menu_bar;
mod window_controls;
mod menu_header;
mod modal;
mod share_sheet;
mod navigation_bar;
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
mod popover;
mod popover_panel;
mod preferences;
mod progress;
mod radio;
mod rolling_number;
mod scheme;
mod search_field;
mod dismiss_drag;
mod segmented_control;
mod select;
mod drag_handle;
mod resize_handle;
mod separator;
mod side_rail;
mod skeleton;
mod slider;
mod split_button;
mod split_view;
mod status_bar;
mod status_dot;
mod stepper;
mod suggestion_list;
mod swatch_picker;
mod switch;
mod tab_strip;
mod table;
mod tabs;
mod terminal_view;
mod text;
mod text_area;
mod text_field;
mod text_shimmer;
mod ticker;
mod time_picker;
mod toast;
mod toggle;
mod toolbar;
mod top_bar;
mod tree_view;
mod video;
#[cfg(test)]
mod code_view_lifecycle_tests;
#[cfg(test)]
mod raw_text_area_tests;
#[cfg(test)]
mod link_presentation_tests;
#[cfg(test)]
mod selection_scope_tests;
#[cfg(test)]
mod text_measurement_tests;
#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod breadcrumb_width_cap_tests;
#[cfg(test)]
mod coach_mark_tests;
#[cfg(test)]
mod dropdown_anchor_tests;
#[cfg(test)]
mod horizontal_scroll_tests;
#[cfg(test)]
mod inline_menu_navigation_tests;
#[cfg(test)]
mod menu_entrance_tests;
#[cfg(test)]
mod modal_wheel_tests;
#[cfg(test)]
mod popover_initial_focus_tests;
#[cfg(test)]
mod popover_secondary_actions_tests;
#[cfg(test)]
mod switch_accent_tests;
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
pub use collapse::Collapse;
pub use collapsible::{collapse_footer, Collapsible};
pub use color_picker::{ColorPicker, Hsv};
pub use composer::Composer;
pub use context_window::{
    ContextItem, ContextShare, ContextShareAction, ContextTones, ContextWindow,
};
pub use framed_card::FramedCard;
pub use date_picker::DatePicker;
pub use date_time_picker::DateTimePicker;
pub use description_list::{DescriptionItem, DescriptionList};
pub use dictation::{DictationButton, DictationState, DictationWaveform};
pub use dropdown_menu::{
    ContextMenuMetrics, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownMenuSection,
};
pub use empty::Empty;
pub use form_field::{FormField, FormFieldSizing};
pub use find_bar::{FindBar, FindIds, FindLabels, FindOptions};
pub use diagram_viewer::DiagramViewer;
pub use image_viewer::{ImageViewer, ViewerLayout};
pub use link::Link;
pub use list_row::{DiffStat, ListRow, ListRowSizing};
pub use markdown::{Markdown, MarkdownColors, MarkdownLabels, MarkdownLimits, MarkdownPreview, MarkdownSourcePage};
pub use markdown_cache::MarkdownFrame;
pub use marker::{thinking_highlight, Marker, MarkerVariant};
pub use media_card::MediaCard;
pub use media_tile::MediaTile;
pub use menu::{MenuItem, MenuSeparator};
pub use menu_bar::{MenuBar, MenuBarMenu};
pub use window_controls::WindowControls;
pub use menu_header::MenuHeader;
pub use modal::{DismissButton, Modal};
pub use share_sheet::{ShareAction, ShareSheet, ShareTarget};
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
pub use popover::Popover;
pub use popover_panel::{PopoverPanel, PopoverPanelSizing};
pub use preferences::{ActionRow, ActionRowSizing, PreferencesGroup};
pub use progress::Progress;
pub use radio::{RadioGroup, RadioMetrics, RadioOption, RadioVariant};
pub use scheme::{ControlColors, Scheme, Surface};
pub use search_field::{Search, SearchField};
pub use dismiss_drag::DismissDrag;
pub use segmented_control::SegmentedControl;
pub use select::{Select, SelectVariant};
pub use drag_handle::DragHandle;
pub use resize_handle::ResizeHandle;
pub use separator::Separator;
pub use side_rail::{SideRail, SideRailItem};
pub use skeleton::{Skeleton, SkeletonGroup};
pub use slider::{MarkStyle, Slider};
pub use split_view::{Pane, SplitView};
pub use split_button::SplitButton;
pub use status_bar::{StatusBar, StatusBarItem, StatusBarItemSizing};
pub use status_dot::StatusDot;
pub use chat_minimap::{ChatMinimap, ChatMinimapMark};
pub use clipboard::{Clipboard, ClipboardCopied};
pub use stepper::{Step, Stepper};
pub use suggestion_list::{SuggestionItem, SuggestionList};
pub use swatch_picker::{Swatch, SwatchPicker};
pub use switch::{Switch, SwitchVariant};
pub use tab_strip::{TabItem, TabStrip, TabStripHeader};
pub use table::{Table, TableColumn, TableRow, TableSizing, TableSort};
pub use tabs::{Tab, Tabs, TabsVariant};
pub use terminal_view::{TerminalCell, TerminalCursor, TerminalCursorShape, TerminalScreen, TerminalView};
pub use text::Text;
pub use text_area::TextArea;
pub use text_field::TextField;
pub use text_shimmer::TextShimmer;
pub use rolling_number::RollingNumber;
pub use ticker::Ticker;
pub use time_picker::TimePicker;
pub use toast::Toast;
pub use toggle::{Toggle, ToggleGroup, ToggleItem, ToggleVariant};
pub use toolbar::{Toolbar, ToolbarItem};
pub use top_bar::TopBar;
pub use theme::Theme;
pub use tree_view::{guides_for_depths, TreeGuides, TreeRow};
pub use video::Video;
// Names these types had before each was called after its component.
#[allow(deprecated)]
pub use list_row::RowSizing;
#[allow(deprecated)]
pub use page::SectionSizing;
#[allow(deprecated)]
pub use panel_header::HeaderSizing;
mod floating_glass_bar;
pub use floating_glass_bar::{FloatingBarMotion, FloatingGlassBar, FloatingGlassTab};
mod glass_menu;
pub use glass_menu::GlassMenu;

mod dropdown_list;
pub use dropdown_list::{DropdownList, DropdownOption};
