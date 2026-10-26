//! Bounded Markdown presentation for prose that arrives as text.
//!
//! The renderer builds an ordinary element tree. It deliberately supports a
//! useful subset of Markdown instead of treating its source as trusted HTML.
//! WarpUI's MIT formatted-text element informed the separation between
//! parsing, line layout, and painting; this is an original renderer. The
//! engine flows inline spans and decorative link icons with the host font
//! measurer. HTTP(S) links retain their destinations as actions the host
//! opens; nothing here fetches, resolves a file or runs a scheme.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Range,
};

use crate::markdown_cache::{self, MarkdownCacheEntry, MarkdownFrame, MARKDOWN_CACHES};
use crate::markdown_parser::{
    begins_web_url, bounded_source, link_icon, parse_bounded_mode, parse_inline_with_tail,
    pending_fence_line, web_links, web_url, Block, InlineKind, InlineRun, ListMarker, MAX_BLOCKS,
    MAX_SOURCE_CHARS,
};
use crate::{
    AlertColors, AlertTone, CodeBlock, CodeLanguage, CodeSyntaxColors, Color, Component, ComponentPalette, Diagram,
    Element, ElementKind, FontFamily, Icon, RadiusRole, TextRole, TextSpan,
};

const MAX_SOURCE_ROW_CHARS: usize = 512;
const BODY_SIZE: f32 = 14.0;
const BODY_LINE_HEIGHT: f32 = 22.0;
const MARKDOWN_RENDER_VERSION: u16 = 8;
/// Subtext against the body, as chat clients size `-# ` lines.
const SUBTEXT_SCALE: f32 = 0.8125;

/// The ten colors of rendered Markdown. [`MarkdownColors::for_palette`] is
/// the default; a host with its own inks passes them with [`Markdown::colors`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkdownColors {
    /// Body text and headings.
    pub text: Color,
    /// `**strong**` runs.
    pub strong: Color,
    /// `*emphasis*` runs.
    pub emphasis: Color,
    /// Links, and a web address still being written.
    pub link: Color,
    /// Inline `code`.
    pub code_ink: Color,
    /// The fill behind inline `code`.
    pub code_fill: Color,
    /// List markers, the quote bar and the note under a limited preview.
    pub muted: Color,
    /// The mark of a checked list item.
    pub success: Color,
    /// The fill of a quote.
    pub panel: Color,
    /// A horizontal rule.
    pub border: Color,
}

impl MarkdownColors {
    /// Inks read from the palette; the link and the checked mark take the
    /// info and success accents of an [`crate::Alert`] on the same surface.
    pub fn for_palette(palette: ComponentPalette) -> Self {
        Self {
            text: palette.text,
            strong: palette.text,
            emphasis: palette.muted,
            link: AlertColors::for_tone(AlertTone::Info, palette).accent,
            code_ink: palette.text,
            code_fill: palette.selected,
            muted: palette.muted,
            success: AlertColors::for_tone(AlertTone::Success, palette).accent,
            panel: palette.raised,
            border: palette.border,
        }
    }
}

/// The two strings the renderer writes itself, in the host's language.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkdownLabels {
    /// The label of a code block whose fence names no language.
    pub code_fallback: &'static str,
    /// The muted line under a preview that was cut to its limits.
    pub truncated_note: &'static str,
    /// The label of a fence drawn as a diagram.
    pub diagram: &'static str,
    /// The action on a diagram that shows the source it was drawn from.
    pub show_source: &'static str,
    /// The action on that source that shows the diagram again.
    pub show_diagram: &'static str,
    /// The action on a diagram that asks the host to show it over the window.
    pub expand: &'static str,
    /// The hint of a code block's copy button.
    pub copy_code: &'static str,
}

impl MarkdownLabels {
    /// English labels, whatever [`crate::set_labels`] was given. A `Markdown`
    /// starts from `MarkdownLabels::default()`, which follows it.
    pub const DEFAULT: Self = Self {
        code_fallback: "CODE",
        truncated_note: "Long text: only the first blocks are shown.",
        diagram: "DIAGRAM",
        show_source: "Source",
        show_diagram: "Diagram",
        expand: "Expand",
        copy_code: "Copy",
    };
}

impl Default for MarkdownLabels {
    /// The words of [`crate::labels`].
    fn default() -> Self {
        let labels = crate::labels();
        Self {
            code_fallback: labels.code,
            truncated_note: labels.long_text,
            diagram: labels.diagram_heading,
            show_source: labels.show_source,
            show_diagram: labels.show_diagram,
            expand: labels.expand,
            copy_code: labels.copy,
        }
    }
}

/// How much of a source is parsed and laid out; the rest is left to
/// [`Markdown::source_page`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkdownLimits {
    /// Unicode scalar values read from the start of the source.
    pub source_chars: usize,
    /// Blocks (paragraphs, list items, code blocks and so on) rendered.
    pub blocks: usize,
}

impl MarkdownLimits {
    /// 20,000 characters and 160 blocks.
    pub const DEFAULT: Self = Self {
        source_chars: MAX_SOURCE_CHARS,
        blocks: MAX_BLOCKS,
    };
}

/// Everything a rendered tree depends on besides its source: the cache key.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct MarkdownPresentation {
    width_bits: u32,
    renderer_version: u16,
    colors: MarkdownColors,
    palette: ComponentPalette,
    labels: MarkdownLabels,
    link_action_prefix: &'static str,
    diagram_action_prefix: &'static str,
    // A digest of the diagrams shown as source, which change the tree.
    diagram_sources: u64,
    limits: MarkdownLimits,
    // Quotes and code blocks bake token radii into the tree; as bits to stay `Eq`.
    radius_bits: [u32; 4],
    // The body size every text and line height is scaled from, as bits.
    font_size_bits: u32,
    code_font_size_bits: Option<u32>,
    code_word_wrap: bool,
    code_label: bool,
    code_fit_content: bool,
    code_copy_action_prefix: &'static str,
    code_syntax_colors: Option<CodeSyntaxColors>,
    // Inline code's size at the body size, as bits; it keeps that proportion elsewhere.
    inline_code_size_bits: Option<u32>,
    streaming_fade: bool,
}

impl MarkdownPresentation {
    fn width(&self) -> f32 {
        f32::from_bits(self.width_bits)
    }

    fn font_size(&self) -> f32 {
        f32::from_bits(self.font_size_bits)
    }

    /// How much larger than the 14 px body everything set in proportion to it is.
    fn scale(&self) -> f32 {
        self.font_size() / BODY_SIZE
    }

    #[cfg(test)]
    pub(crate) fn at_width(width: f32) -> Self {
        Markdown::new("", ComponentPalette::neutral_dark()).width(width).presentation
    }
}

/// A bounded Markdown preview. Its tree is passive apart from link spans,
/// whose action is the link prefix, a hash of the block, the run's index and
/// the destination, separated by colons; a host that shows the same source
/// twice rewrites them to keep focus targets distinct. `limited` says the
/// source or its blocks were cut.
pub struct MarkdownPreview {
    /// The preview's tree, for the host to place.
    pub element: Element,
    /// Whether the source or its blocks were cut to fit the preview's limits.
    pub limited: bool,
}

/// Markdown source as wrapped prose: headings, paragraphs, lists with
/// checkboxes, quotes, fenced code and rules, with strong, emphasis, inline
/// code and web links. The host owns the source and opens the links; code
/// blocks are [`CodeBlock`]s on `palette`. Their comments, strings, keywords
/// and numbers are colored, in inks for a dark or a light palette, when the
/// fence names a language [`CodeLanguage::from_code_fence`] knows and the code
/// is within 64 KiB and 2,000 lines; other code stays plain.
pub struct Markdown<'a> {
    source: &'a str,
    presentation: MarkdownPresentation,
    streaming: bool,
    plain_line_height: Option<f32>,
    diagram_sources: &'a [u64],
    id: Option<String>,
}

impl<'a> Markdown<'a> {
    /// `source` at a reading width of 600, complete, in the palette's colors.
    pub fn new(source: &'a str, palette: ComponentPalette) -> Self {
        Self {
            source,
            presentation: MarkdownPresentation {
                width_bits: normalized_width(f32::NAN).to_bits(),
                renderer_version: MARKDOWN_RENDER_VERSION,
                colors: MarkdownColors::for_palette(palette),
                palette,
                labels: MarkdownLabels::default(),
                link_action_prefix: "markdown_link:",
                diagram_action_prefix: "",
                diagram_sources: 0,
                limits: MarkdownLimits::DEFAULT,
                radius_bits: {
                    let radius = crate::tokens().radius;
                    [radius.control, radius.surface, radius.card, radius.panel].map(f32::to_bits)
                },
                font_size_bits: BODY_SIZE.to_bits(),
                code_font_size_bits: None,
                code_word_wrap: true,
                code_label: true,
                code_fit_content: false,
                code_copy_action_prefix: "",
                code_syntax_colors: None,
                inline_code_size_bits: None,
                streaming_fade: false,
            },
            streaming: false,
            plain_line_height: None,
            diagram_sources: &[],
            id: None,
        }
    }

    /// Stable identity of this preview's code viewports. Set it when rebuilding
    /// a preview each frame to retain horizontal scroll offsets. Source trees
    /// remain shareable in the cache; IDs are applied to the returned instance.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The reading column in logical pixels, kept between 48 and 1,200. A
    /// width that is not finite reads as 600.
    pub fn width(mut self, width: f32) -> Self {
        self.presentation.width_bits = normalized_width(width).to_bits();
        self
    }

    /// The size of body text in logical pixels, 14 unless set. Headings,
    /// inline and fenced code, list markers, line heights and the space
    /// between blocks keep their proportion to it. A size that is not finite
    /// and positive is ignored; the rest is kept between 8 and 96.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.presentation.font_size_bits = size.clamp(8.0, 96.0).to_bits();
        }
        self
    }

    /// Set the absolute size of fenced code, independently of body text.
    /// Without an override it stays proportional to the body. Nonpositive or
    /// nonfinite values are ignored; valid sizes stay within 1..=96.
    pub fn code_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.presentation.code_font_size_bits = Some(size.clamp(1.0, 96.0).to_bits());
        }
        self
    }

    /// Wrap fenced code to the reading width (the default). Disabled code keeps
    /// source lines in a horizontal viewport; prose continues to wrap normally.
    pub fn code_word_wrap(mut self, enabled: bool) -> Self {
        self.presentation.code_word_wrap = enabled;
        self
    }

    /// Write the fence's language, or [`MarkdownLabels::code_fallback`], above
    /// fenced code (the default). Without it the block holds only the code.
    pub fn code_label(mut self, shown: bool) -> Self {
        self.presentation.code_label = shown;
        self
    }

    /// Make each code block as wide as its code and padding, up to the
    /// reading width, instead of the whole width (the default).
    pub fn code_fit_content(mut self, fit: bool) -> Self {
        self.presentation.code_fit_content = fit;
        self
    }

    /// Give every code block a copy icon at its end, hinted
    /// [`MarkdownLabels::copy_code`], whose action is `prefix` followed by the
    /// code exactly: the host copies what follows the prefix. Two blocks with
    /// the same code in one tree share the action. Without a prefix, the
    /// default, a block has no button.
    pub fn code_copy_action_prefix(mut self, prefix: &'static str) -> Self {
        self.presentation.code_copy_action_prefix = prefix;
        self
    }

    /// The inks of fenced code's highlights and text, in place of
    /// [`CodeSyntaxColors::for_palette`]. Setting `function` or
    /// `interpolation` also tells those apart.
    pub fn code_syntax_colors(mut self, colors: CodeSyntaxColors) -> Self {
        self.presentation.code_syntax_colors = Some(colors);
        self
    }

    /// The size of inline `code` in body text, in logical pixels; the body
    /// size unless set. In a heading or a subtext line it keeps the same
    /// proportion to that text. A size that is not finite and positive is
    /// ignored; the rest is kept between 1 and 96.
    pub fn inline_code_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.presentation.inline_code_size_bits = Some(size.clamp(1.0, 96.0).to_bits());
        }
        self
    }

    /// While [`streaming`](Self::streaming), fade the last characters of the
    /// block being written, the newest faintest, so new text comes in out of
    /// a fade as a chat client reveals it. Off by default.
    pub fn streaming_fade(mut self, enabled: bool) -> Self {
        self.presentation.streaming_fade = enabled;
        self
    }

    /// Replace the colors derived from the palette.
    pub fn colors(mut self, colors: MarkdownColors) -> Self {
        self.presentation.colors = colors;
        self
    }

    /// The source is still being written: an unclosed delimiter at its end
    /// styles its text, a web address there has no action until it ends, and
    /// the blocks above the last one are reused from the previous render.
    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// The start of every link action, `markdown_link:` unless set.
    pub fn link_action_prefix(mut self, prefix: &'static str) -> Self {
        self.presentation.link_action_prefix = prefix;
        self
    }

    /// Give every diagram an action that switches between the drawing and
    /// its source: `prefix` followed by the diagram's id as sixteen
    /// hexadecimal digits. The host keeps the ids it was asked to show as
    /// source and passes them to [`Markdown::diagram_sources`]. Without a
    /// prefix a diagram has no such action.
    pub fn diagram_action_prefix(mut self, prefix: &'static str) -> Self {
        self.presentation.diagram_action_prefix = prefix;
        self
    }

    /// The source of the diagram whose expand action, `prefix`, `expand:`
    /// and sixteen hexadecimal digits, carries `id`: what a host parses
    /// with [`Diagram::parse`] and draws in a [`crate::DiagramViewer`].
    pub fn diagram_source(id: u64) -> Option<std::sync::Arc<str>> {
        DIAGRAMS.with_borrow(|diagrams| {
            diagrams.iter().find(|(known, ..)| *known == id).map(|(_, source, _)| source.clone())
        })
    }

    /// The source as what a `mermaid` fence holds: the card a closed fence
    /// is drawn as, with the labels, the action prefix and the diagrams
    /// shown as source given here, or `None` when it is not a diagram the
    /// engine draws. For a host that renders its documents itself.
    pub fn diagram(self) -> Option<Element> {
        diagram_block(self.source, &self.presentation, self.diagram_sources)
    }

    /// The diagrams, by the id in their action, to show as source.
    pub fn diagram_sources(mut self, ids: &'a [u64]) -> Self {
        self.diagram_sources = ids;
        self.presentation.diagram_sources = if ids.is_empty() {
            0
        } else {
            let mut hash = DefaultHasher::new();
            ids.hash(&mut hash);
            hash.finish()
        };
        self
    }

    /// The renderer's own strings, in the host's language.
    pub fn labels(mut self, labels: MarkdownLabels) -> Self {
        self.presentation.labels = labels;
        self
    }

    /// How much of the source is rendered. A limit of zero reads as one.
    pub fn limits(mut self, limits: MarkdownLimits) -> Self {
        self.presentation.limits = MarkdownLimits {
            source_chars: limits.source_chars.max(1),
            blocks: limits.blocks.max(1),
        };
        self
    }

    /// Keep the source literal, as one paragraph with this line height:
    /// only complete HTTP(S) addresses are styled and become links. Nothing
    /// is cached or cut. A height that is not positive and finite is ignored.
    pub fn plain_with_autolinks(mut self, line_height: f32) -> Self {
        if line_height.is_finite() && line_height > 0.0 {
            self.plain_line_height = Some(line_height);
        }
        self
    }

    /// Build the tree. A complete source reuses a bounded tree until its
    /// displayed content, width or presentation changes; the host still
    /// performs layout.
    pub fn render(self) -> MarkdownPreview {
        let mut preview = match self.plain_line_height {
            Some(line_height) => MarkdownPreview {
                element: render_plain(self.source, &self.presentation, line_height),
                limited: false,
            },
            None => render_cached_markdown(
                self.source,
                self.presentation,
                self.streaming,
                self.diagram_sources,
            ),
        };
        if !self.presentation.code_word_wrap {
            let scope = self.id.unwrap_or_else(crate::code_block::anonymous_code_scroll_id);
            scope_code_scrolls(&mut preview.element, &scope, &mut 0);
        }
        preview
    }

    /// Hold the returned guard while building one frame of a transcript:
    /// until it drops, a tree that misses the cache cannot evict the trees
    /// of the rows still to come. A new `scope` or `width` empties the cache.
    #[must_use]
    pub fn begin_frame(scope: Option<&str>, width: f32) -> MarkdownFrame {
        markdown_cache::begin_frame(scope, normalized_width(width).to_bits())
    }

    /// The complete HTTP(S) addresses written in `source`, each with its
    /// range in characters and its normalized form. At most 64, read from
    /// the first 20,000 characters; no address is fetched.
    pub fn web_links(source: &str) -> Vec<(Range<usize>, String)> {
        web_links(source)
            .into_iter()
            .map(|(range, url)| (range, url.into()))
            .collect()
    }

    /// `value` as a normalized HTTP(S) address, or `None` when it is not one
    /// this renderer would open: another scheme, credentials, whitespace or
    /// more than 2,048 bytes. For a host that receives a link's action.
    pub fn web_link(value: &str) -> Option<String> {
        web_url(value).map(Into::into)
    }

    /// The glyph a link to `address` wears: the GitHub mark for github.com,
    /// a globe for everything else.
    pub fn link_icon(address: &str) -> Icon {
        web_url(address).as_ref().map_or(Icon::Globe, link_icon)
    }

    /// Return at most 20,000 Unicode scalar values. Invalid or stale offsets reset
    /// to the beginning instead of slicing through a UTF-8 character. Each cursor
    /// calculation scans at most one page, even for a very large message.
    pub fn source_page(source: &str, start_byte: usize) -> MarkdownSourcePage<'_> {
        let start_byte = if source.get(start_byte..).is_some() {
            start_byte
        } else {
            0
        };
        let (text, _) = bounded_source(&source[start_byte..], MAX_SOURCE_CHARS);
        let end_byte = start_byte + text.len();
        let previous_byte = (start_byte > 0).then(|| {
            source[..start_byte]
                .char_indices()
                .rev()
                .nth(MAX_SOURCE_CHARS - 1)
                .map_or(0, |(offset, _)| offset)
        });
        MarkdownSourcePage {
            text,
            start_byte,
            end_byte,
            previous_byte,
            next_byte: (end_byte < source.len()).then_some(end_byte),
        }
    }
}

fn source_hash(source: &str) -> u64 {
    let mut hash = DefaultHasher::new();
    source.hash(&mut hash);
    hash.finish()
}

fn link_action(prefix: &str, scope: u64, offset: usize, destination: &str) -> String {
    format!("{prefix}{scope:x}:{offset}:{destination}")
}

/// Literal source with link styling on complete HTTP(S) destinations only;
/// file paths, code and Markdown syntax remain unchanged.
fn render_plain(source: &str, presentation: &MarkdownPresentation, line_height: f32) -> Element {
    let colors = presentation.colors;
    let size = presentation.font_size();
    let mut spans = Vec::new();
    let mut byte = 0;
    let mut character = 0;
    let hash = source_hash(source);
    for (index, (range, url)) in web_links(source).into_iter().enumerate() {
        let before = source[byte..]
            .char_indices()
            .nth(range.start - character)
            .map_or(source.len(), |(offset, _)| byte + offset);
        let end = source[before..]
            .char_indices()
            .nth(range.end - range.start)
            .map_or(source.len(), |(offset, _)| before + offset);
        if before > byte {
            spans.push(TextSpan::new(
                &source[byte..before],
                colors.text,
                size,
                FontFamily::Sans,
            ));
        }
        spans.push(
            TextSpan::new(&source[before..end], colors.link, size, FontFamily::Sans)
                .icon(link_icon(&url))
                .action(link_action(
                    presentation.link_action_prefix,
                    hash,
                    index,
                    url.as_str(),
                )),
        );
        byte = end;
        character = range.end;
    }
    if byte < source.len() {
        spans.push(TextSpan::new(
            &source[byte..],
            colors.text,
            size,
            FontFamily::Sans,
        ));
    }
    Element::formatted_text(spans).line_height(line_height)
}

fn span_element(run: &InlineRun, size: f32, colors: MarkdownColors, code_scale: f32) -> TextSpan {
    let (color, family) = match run.kind {
        InlineKind::Normal => (colors.text, FontFamily::Sans),
        InlineKind::Strong => (colors.strong, FontFamily::SansBold),
        InlineKind::StrongEmphasis => (colors.strong, FontFamily::SansBoldItalic),
        InlineKind::Link => (colors.link, FontFamily::Sans),
        InlineKind::Emphasis => (colors.emphasis, FontFamily::SansItalic),
        InlineKind::Code => (colors.code_ink, FontFamily::Monospace),
    };
    let size = if run.kind == InlineKind::Code { size * code_scale } else { size };
    let mut span = TextSpan::new(&run.text, color, size, family);
    if run.kind == InlineKind::Code {
        span = span.background(colors.code_fill);
    }
    span.underline = run.underline;
    span.strikethrough = run.strikethrough;
    span
}

fn render_inline_mode(
    source: &str,
    width: f32,
    size: f32,
    incomplete: bool,
    url_tail_complete: bool,
    presentation: &MarkdownPresentation,
) -> Element {
    let hash = source_hash(source);
    let code_scale = presentation
        .inline_code_size_bits
        .map_or(1.0, |bits| f32::from_bits(bits) / presentation.font_size());
    let spans = parse_inline_with_tail(source, incomplete, url_tail_complete)
        .iter()
        .enumerate()
        .map(|(index, run)| {
            let span = span_element(run, size, presentation.colors, code_scale);
            match run.destination.as_deref().and_then(web_url) {
                Some(url) => span.icon(link_icon(&url)).action(link_action(
                    presentation.link_action_prefix,
                    hash,
                    index,
                    url.as_str(),
                )),
                None => span,
            }
        })
        .collect();
    let spans = if incomplete && presentation.streaming_fade { fade_tail(spans) } else { spans };
    Element::formatted_text(spans)
        .font_family(FontFamily::Sans)
        .font_size(size)
        .line_height(if size == presentation.font_size() {
            BODY_LINE_HEIGHT * presentation.scale()
        } else {
            size * 1.4
        })
        .size(Some(width.max(0.0)), None)
}

/// Characters a streaming fade spans, and how many steps it takes over them.
const FADE_CHARS: usize = 24;
const FADE_STEPS: usize = 6;

/// The spans with their last `FADE_CHARS` characters in steps of fading ink,
/// the last step at a fifth of it. A span with an action or an icon is not
/// split, so its one target stays whole: it takes the step of its last
/// character.
fn fade_tail(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    let step_chars = FADE_CHARS / FADE_STEPS;
    // Characters from the end of the text to the end of each span.
    let mut after = 0;
    let mut out: Vec<TextSpan> = Vec::with_capacity(spans.len() + FADE_STEPS);
    for span in spans.into_iter().rev() {
        let count = span.text.chars().count();
        let alpha = |from_end: usize| {
            let step = from_end / step_chars;
            if step >= FADE_STEPS {
                1.0
            } else {
                0.2 + 0.8 * step as f32 / FADE_STEPS as f32
            }
        };
        let faded = |mut span: TextSpan, alpha: f32| {
            span.color.a = (f32::from(span.color.a) * alpha).round() as u8;
            span
        };
        if after >= FADE_CHARS {
            out.push(span);
        } else if span.action.is_some() || span.icon.is_some() {
            out.push(faded(span, alpha(after)));
        } else {
            // Cut the span where the steps change, last piece first.
            let chars: Vec<char> = span.text.chars().collect();
            let mut end = count;
            while end > 0 {
                let from_end = after + (count - end);
                let step_left = if from_end >= FADE_CHARS { end } else { step_chars - from_end % step_chars };
                let start = end.saturating_sub(step_left);
                let piece = TextSpan { text: chars[start..end].iter().collect(), ..span.clone() };
                out.push(faded(piece, alpha(from_end)));
                end = start;
            }
        }
        after += count;
    }
    out.reverse();
    out
}

fn code_block(language: &str, content: &str, presentation: &MarkdownPresentation) -> Element {
    let mut block = CodeBlock::new(content, presentation.palette)
        .language(CodeLanguage::from_code_fence(language))
        .word_wrap(presentation.code_word_wrap)
        .font_scale(presentation.scale())
        .fit_content(presentation.code_fit_content)
        .syntax_colors(presentation.code_syntax_colors);
    if presentation.code_label {
        block = block.label(if language.is_empty() {
            presentation.labels.code_fallback.to_owned()
        } else {
            language.to_uppercase()
        });
    }
    if let Some(size) = presentation.code_font_size_bits {
        block = block.font_size(f32::from_bits(size));
    }
    let prefix = presentation.code_copy_action_prefix;
    if !prefix.is_empty() {
        block = block.copy(format!("{prefix}{content}"), presentation.labels.copy_code.to_owned());
    }
    block.render()
}

fn scope_code_scrolls(element: &mut Element, scope: &str, ordinal: &mut usize) {
    match &mut element.kind {
        ElementKind::Column(children) | ElementKind::Row(children) => {
            for child in children { scope_code_scrolls(child, scope, ordinal); }
        }
        ElementKind::Panel(child) => scope_code_scrolls(child, scope, ordinal),
        ElementKind::HorizontalScroll { id, child, .. } => {
            if id.starts_with("code_block:auto:") {
                *id = format!("markdown:{scope}:code:{ordinal}");
                *ordinal += 1;
            }
            scope_code_scrolls(child, scope, ordinal);
        }
        _ => {}
    }
}

/// Multiply every text size and line height in a card that sets its text by
/// role, such as a [`CodeBlock`], so it keeps its proportion to the body.
fn scale_text(element: &mut Element, scale: f32) {
    if scale == 1.0 {
        return;
    }
    element.style.font_size *= scale;
    if let Some(height) = element.style.line_height.as_mut() {
        *height *= scale;
    }
    match &mut element.kind {
        ElementKind::FormattedText(spans) => {
            for span in spans {
                span.font_size *= scale;
            }
        }
        ElementKind::Column(children) | ElementKind::Row(children) => {
            for child in children {
                scale_text(child, scale);
            }
        }
        ElementKind::Panel(child) | ElementKind::HorizontalScroll { child, .. } => scale_text(child, scale),
        _ => {}
    }
}

thread_local! {
    // shortcut: the last 64 diagrams drawn, so a host can open one from the
    // id in its action; an older one no longer expands until it is drawn
    // again, and is parsed again by a host that builds its tree every
    // frame. Keep the source with the cached tree if that is ever met.
    static DIAGRAMS: std::cell::RefCell<Vec<(u64, std::sync::Arc<str>, Diagram)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The diagram `source` draws, parsed once: a host that builds its tree
/// again every frame gets the same chart, and with it the same layout.
fn drawn_diagram(id: u64, source: &str, palette: ComponentPalette) -> Option<Diagram> {
    DIAGRAMS.with_borrow_mut(|diagrams| {
        let known = diagrams
            .iter()
            .find(|(known, text, diagram)| *known == id && **text == *source && diagram.palette() == palette);
        if let Some((.., diagram)) = known {
            return Some(diagram.clone());
        }
        let diagram = Diagram::parse(source, palette).ok()?;
        diagrams.retain(|(known, ..)| *known != id);
        if diagrams.len() == 64 {
            diagrams.remove(0);
        }
        diagrams.push((id, source.into(), diagram.clone()));
        Some(diagram)
    })
}

/// A closed `mermaid` fence that is a flowchart or a sequence diagram,
/// drawn; `None` leaves it a code block. The card matches [`CodeBlock`],
/// with one quiet action in its label row that switches between the
/// drawing and the source.
fn diagram_block(
    content: &str,
    presentation: &MarkdownPresentation,
    as_source: &[u64],
) -> Option<Element> {
    // Before anything reads it: a host may hand over a fence of any size,
    // on every frame.
    if content.len() > crate::diagram::MAX_SOURCE_BYTES {
        return None;
    }
    let palette = presentation.palette;
    let id = source_hash(content);
    let chart = drawn_diagram(id, content, palette)?;
    let shows_source = as_source.contains(&id);
    let labels = presentation.labels;
    let mut header = vec![Element::text(if shows_source { "MERMAID" } else { labels.diagram })
        .font_family(FontFamily::Monospace)
        .font_size(TextRole::Micro.size())
        .foreground(palette.muted)];
    if !presentation.diagram_action_prefix.is_empty() {
        let prefix = presentation.diagram_action_prefix;
        let action = |label: &str, action: String| {
            Element::formatted_text(vec![TextSpan::new(
                label,
                palette.muted,
                TextRole::Small.size(),
                FontFamily::Sans,
            )
            .action(action)])
            .font_family(FontFamily::Sans)
            .font_size(TextRole::Small.size())
        };
        header.push(Element::spacer().flex(1.0));
        if !shows_source {
            header.push(action(labels.expand, format!("{prefix}expand:{id:016x}")));
        }
        header.push(action(
            if shows_source { labels.show_diagram } else { labels.show_source },
            format!("{prefix}{id:016x}"),
        ));
    }
    let body = if shows_source {
        Element::text(content)
            .font_family(FontFamily::Monospace)
            .font_size(TextRole::Caption.size())
            .foreground(palette.text)
    } else {
        chart.scrollable(format!("markdown_diagram:{id:016x}"))
    };
    let mut card =
        Element::panel(Element::column(vec![Element::row(header).gap(14.0), body]).gap(8.0))
            .padding(12.0)
            .background(palette.raised)
            .border(palette.border, 1.0)
            .radius(RadiusRole::Card.px());
    // The drawing keeps its own size; only the card's words follow the body.
    scale_text(&mut card, presentation.scale());
    Some(card)
}

/// One exact slice of the original Markdown source. Pagination limits only
/// the presentation and never modifies, drops, or parses the retained source.
/// Cursors are UTF-8 byte offsets into the original message, not character
/// counts or offsets into an independently copied page.
pub struct MarkdownSourcePage<'a> {
    /// The page: `source[start_byte..end_byte]`.
    pub text: &'a str,
    /// Where the page starts in the source, as a UTF-8 byte offset.
    pub start_byte: usize,
    /// Where the page ends in the source, as a UTF-8 byte offset; the byte at it
    /// is not part of the page.
    pub end_byte: usize,
    /// Where the page before this one starts, unless this is the first.
    pub previous_byte: Option<usize>,
    /// Where the page after this one starts, unless this is the last.
    pub next_byte: Option<usize>,
}

impl MarkdownSourcePage<'_> {
    /// Present source literally with wrapping and no Markdown block limit. Split
    /// each bounded page into small passive rows so a single long paragraph or
    /// code line cannot create a 20,000-character layout block. The modal host
    /// supplies the scroll viewport; nothing here retains the full source. The
    /// rows are `width` wide, in the palette's text color.
    pub fn render(&self, width: f32, palette: ComponentPalette) -> Element {
        let page = self;
        let width = normalized_width(width);
        let mut rows = Vec::new();
        let (mut remaining, _) = bounded_source(page.text, MAX_SOURCE_CHARS);
        while !remaining.is_empty() {
            let end = remaining
                .char_indices()
                .nth(MAX_SOURCE_ROW_CHARS)
                .map_or(remaining.len(), |(offset, _)| offset);
            let mut text = &remaining[..end];
            remaining = &remaining[end..];
            // The boundary between two rows already supplies a visual line break.
            // Avoid displaying a second empty line when a chunk ends at a newline.
            // The exact source, including that newline, remains in `page.text`.
            if !remaining.is_empty() {
                text = text
                    .strip_suffix("\r\n")
                    .or_else(|| text.strip_suffix('\n'))
                    .unwrap_or(text);
            }
            rows.push(
                Element::text(text)
                    .font_family(FontFamily::Monospace)
                    .font_size(12.0)
                    .foreground(palette.text)
                    .size(Some(width), None),
            );
        }
        Element::column(rows).gap(0.0).size(Some(width), None)
    }
}

fn render_cached_markdown(
    source: &str,
    presentation: MarkdownPresentation,
    streaming: bool,
    diagram_sources: &[u64],
) -> MarkdownPreview {
    let (source, truncated) = bounded_source(source, presentation.limits.source_chars);
    let cached = MARKDOWN_CACHES.with(|caches| {
        caches
            .borrow_mut()
            .get(source, truncated, presentation, streaming)
    });
    if let Some(element) = cached {
        return element;
    }
    let (blocks, limited) = parse_bounded_mode(source, truncated, streaming, presentation.limits.blocks);
    let pending_fence =
        !source.ends_with('\n') && source.lines().next_back().is_some_and(pending_fence_line);
    let incomplete_block =
        (streaming && !limited && !pending_fence && !ends_with_blank_line(source))
            .then(|| blocks.len().checked_sub(1))
            .flatten()
            .filter(|index| {
                !matches!(blocks[*index], Block::Code(..) | Block::OpenCode(..) | Block::Rule)
            });
    let mut preview = MARKDOWN_CACHES.with(|caches| {
        let caches = caches.borrow();
        let previous = streaming
            .then(|| {
                caches.streaming.entries.iter().rev().find(|entry| {
                    entry.presentation == presentation && source.starts_with(&entry.source)
                })
            })
            .flatten();
        render_markdown_blocks(
            &blocks,
            &presentation,
            limited,
            incomplete_block,
            source.ends_with(char::is_whitespace),
            previous,
            diagram_sources,
        )
    });
    if truncated && !source.ends_with(char::is_whitespace) {
        suspend_truncated_tail_link(&mut preview.element, source);
    }
    // Completed rows already have an exact-source tree; only live rows
    // need block metadata to reconcile the next append.
    let retained_blocks = if streaming { blocks.as_slice() } else { &[] };
    if let Some(entry) = MarkdownCacheEntry::new(
        source,
        truncated,
        presentation,
        &preview,
        retained_blocks,
        incomplete_block,
    ) {
        MARKDOWN_CACHES.with(|caches| caches.borrow_mut().insert(entry, streaming));
    }
    preview
}

/// A presentation limit must never turn a URL prefix into a different target.
/// Keep the bounded text readable, but wait for the full-source view to open it.
fn suspend_truncated_tail_link(element: &mut Element, source: &str) {
    match &mut element.kind {
        ElementKind::Column(children) | ElementKind::Row(children) => {
            for child in children {
                suspend_truncated_tail_link(child, source);
            }
        }
        ElementKind::Panel(child) => suspend_truncated_tail_link(child, source),
        ElementKind::FormattedText(spans) => {
            let mut suffix = String::new();
            for span in spans.iter_mut().rev() {
                suffix.insert_str(0, &span.text);
                if begins_web_url(&span.text) && span.action.is_some() {
                    if source.ends_with(&suffix) {
                        span.action = None;
                        span.icon = None;
                    }
                    break;
                }
                if !span.text.chars().all(|ch| ch.is_ascii_punctuation()) {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn normalized_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(48.0, 1_200.0)
    } else {
        600.0
    }
}

fn ends_with_blank_line(source: &str) -> bool {
    let mut lines = source.lines().rev();
    lines.next().is_some_and(|line| line.trim().is_empty())
}

fn block_spacing(previous: Option<&Block>, current: &Block) -> f32 {
    match (previous, current) {
        (None, _) => 0.0,
        (_, Block::Heading(..)) => 20.0,
        (Some(Block::Heading(..)), _) => 8.0,
        (Some(Block::ListItem(..)), Block::ListItem(..)) => 4.0,
        _ => 14.0,
    }
}

fn render_markdown_blocks(
    blocks: &[Block],
    presentation: &MarkdownPresentation,
    truncated: bool,
    incomplete_block: Option<usize>,
    url_tail_complete: bool,
    previous: Option<&MarkdownCacheEntry>,
    diagram_sources: &[u64],
) -> MarkdownPreview {
    let width = presentation.width();
    let colors = presentation.colors;
    let (body, scale) = (presentation.font_size(), presentation.scale());
    let mut elements = Vec::with_capacity(blocks.len() + usize::from(truncated));
    for (index, block) in blocks.iter().enumerate() {
        let incomplete = incomplete_block == Some(index);
        let spacing = block_spacing(index.checked_sub(1).and_then(|i| blocks.get(i)), block);
        // The owned UI tree still participates in native layout. Reusing each
        // unchanged block avoids reparsing inline markup above the stream tail.
        let reused = previous.and_then(|entry| {
            if entry.blocks.get(index) != Some(block)
                || (entry.incomplete_block == Some(index)) != incomplete
                || (incomplete && entry.source.ends_with(char::is_whitespace) != url_tail_complete)
                || block_spacing(
                    index.checked_sub(1).and_then(|i| entry.blocks.get(i)),
                    block,
                ) != spacing
            {
                return None;
            }
            let ElementKind::Column(children) = &entry.element.kind else {
                return None;
            };
            children.get(index).cloned()
        });
        if let Some(element) = reused {
            elements.push(element);
            continue;
        }
        let element = match block {
            Block::Heading(level, text) => {
                let size = scale
                    * match level {
                        1 => 22.0,
                        2 => 18.0,
                        _ => 16.0,
                    };
                render_inline_mode(text, width, size, incomplete, url_tail_complete, presentation)
            }
            Block::Paragraph(text) => {
                render_inline_mode(
                    text,
                    width,
                    body,
                    incomplete,
                    url_tail_complete,
                    presentation,
                )
            }
            Block::ListItem(depth, marker, text) => {
                let (label, color) = match marker {
                    ListMarker::Bullet => ("•".to_owned(), colors.muted),
                    ListMarker::Number(number) => (format!("{number}."), colors.muted),
                    ListMarker::Done => ("✓".to_owned(), colors.success),
                    ListMarker::Todo => ("○".to_owned(), colors.muted),
                };
                // The text keeps room for a few characters, then the marker
                // column, then the nesting: the item never starts past the
                // edge. At 14 px this is the old 22 px column and 48 px floor.
                let marker = (22.0 * scale).min(width - 26.0);
                let indent = (*depth as f32 * 14.0 * scale)
                    .min((width - marker - 3.0 - 23.0 * scale).max(0.0));
                Element::row(vec![
                    Element::spacer().size(Some(indent), Some(1.0)),
                    Element::formatted_text(vec![TextSpan::new(
                        label,
                        color,
                        body,
                        FontFamily::Sans,
                    )])
                    .font_family(FontFamily::Sans)
                    .font_size(body)
                    .line_height(BODY_LINE_HEIGHT * scale)
                    .foreground(color)
                    .size(Some(marker), None),
                    render_inline_mode(
                        text,
                        width - indent - marker - 6.0,
                        body,
                        incomplete,
                        url_tail_complete,
                        presentation,
                    )
                    .size(None, None)
                    .flex(1.0),
                ])
                .gap(3.0)
            }
            Block::Subtext(text) => {
                let quiet = MarkdownPresentation {
                    colors: MarkdownColors {
                        text: colors.muted,
                        strong: colors.muted,
                        emphasis: colors.muted,
                        ..colors
                    },
                    ..*presentation
                };
                render_inline_mode(
                    text,
                    width,
                    body * SUBTEXT_SCALE,
                    incomplete,
                    url_tail_complete,
                    &quiet,
                )
            }
            Block::Quote(text) => Element::panel(
                Element::row(vec![
                    Element::spacer()
                        .size(Some(2.0), None)
                        .background(colors.muted),
                    render_inline_mode(
                        text,
                        width - 28.0,
                        body,
                        incomplete,
                        url_tail_complete,
                        presentation,
                    )
                    .size(None, None)
                    .flex(1.0),
                ])
                .gap(10.0),
            )
            .padding(8.0)
            .background(colors.panel)
            .radius(RadiusRole::Surface.px()),
            // Only a closed fence is drawn: a diagram still being written
            // would redraw, or stop parsing, with every line that arrives.
            Block::Code(language, content) if language.eq_ignore_ascii_case("mermaid") => {
                diagram_block(content, presentation, diagram_sources)
                    .unwrap_or_else(|| code_block(language, content, presentation))
            }
            Block::Code(language, content) | Block::OpenCode(language, content) => {
                code_block(language, content, presentation)
            }
            Block::Rule => Element::spacer()
                .size(None, Some(1.0))
                .background(colors.border),
        };
        // One root per source block keeps streaming reconciliation stable.
        // List rows stay compact while prose and headings have their own rhythm.
        elements.push(Element::column(vec![
            Element::spacer().size(None, Some(spacing * scale)),
            element,
        ]));
    }
    if truncated {
        elements.push(
            Element::text(presentation.labels.truncated_note)
                .font_family(FontFamily::Sans)
                .font_size(11.0 * scale)
                .foreground(colors.muted),
        );
    }
    MarkdownPreview {
        element: Element::column(elements),
        limited: truncated,
    }
}

#[cfg(test)]
mod tests {
    use unicode_segmentation::UnicodeSegmentation;

    use super::*;
    use crate::markdown_cache::{
        passive_heap_bytes, MarkdownCaches, MAX_CACHED_BYTES, MAX_STREAMING_CACHE_BYTES,
    };
    use crate::markdown_parser::{
        parse, parse_inline, parse_inline_mode, safe_url, url_token_len, INLINE_PARSE_COUNT,
    };
    use crate::{DrawCommand, Icon, Size, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn render_cached_markdown(source: &str, width: f32, streaming: bool) -> MarkdownPreview {
        Markdown::new(source, palette()).width(width).streaming(streaming).render()
    }

    fn render_markdown_preview_at_width(source: &str, width: f32) -> MarkdownPreview {
        render_cached_markdown(source, width, false)
    }

    fn render_streaming_markdown_preview_at_width(source: &str, width: f32) -> MarkdownPreview {
        render_cached_markdown(source, width, true)
    }

    fn render_markdown_at_width(source: &str, width: f32) -> Element {
        render_cached_markdown(source, width, false).element
    }

    fn render_streaming_markdown_at_width(source: &str, width: f32) -> Element {
        render_cached_markdown(source, width, true).element
    }

    fn render_inline(source: &str, width: f32, size: f32) -> Element {
        render_inline_mode(source, width, size, false, false, &MarkdownPresentation::at_width(width))
    }

    fn markdown_source_page(source: &str, start_byte: usize) -> MarkdownSourcePage<'_> {
        Markdown::source_page(source, start_byte)
    }

    fn render_markdown_source_page_at_width(page: &MarkdownSourcePage<'_>, width: f32) -> Element {
        page.render(width, palette())
    }

    fn web_link_url_from_action(action: &str) -> Option<String> {
        let mut parts = action.strip_prefix("markdown_link:")?.splitn(3, ':');
        u64::from_str_radix(parts.next()?, 16).ok()?;
        parts.next()?.parse::<usize>().ok()?;
        Some(web_url(parts.next()?)?.to_string())
    }

    fn actions(element: Element) -> Vec<String> {
        UiTree::new(element)
            .layout(Size::new(640.0, 2000.0))
            .regions
            .into_iter()
            .filter_map(|region| region.id.starts_with("markdown_link:").then_some(region.id))
            .collect()
    }

    #[test]
    fn a_streaming_fade_dims_the_newest_characters_in_steps_and_keeps_the_text() {
        let ink = Color::rgb(10, 20, 30);
        let spans = vec![
            TextSpan::new("The subagent that lists ", ink, 14.0, FontFamily::Sans),
            TextSpan::new("the files", ink, 14.0, FontFamily::Sans).action("link"),
            TextSpan::new(" in /work", ink, 14.0, FontFamily::Sans),
        ];
        let faded = fade_tail(spans.clone());
        let text: String = faded.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, spans.iter().map(|s| s.text.as_str()).collect::<String>());
        // Older text keeps its ink; the last characters are the faintest; the link stays one piece.
        assert_eq!(faded[0].color.a, 255);
        assert_eq!(faded.last().unwrap().color.a, (255.0_f32 * 0.2).round() as u8);
        assert_eq!(faded.iter().filter(|s| s.action.is_some()).count(), 1);
        let alphas: Vec<u8> = faded.iter().map(|s| s.color.a).collect();
        assert!(alphas.windows(2).all(|w| w[0] >= w[1]), "{alphas:?}");
    }

    #[test]
    fn parses_blocks_in_order_and_keeps_number_and_checklist_state() {
        let (blocks, truncated) = parse(
            "# Plan\n\nIntro\n- Uno\n  - [x] Dos\n3. Tres\n> Nota\n```rust\nfn main() {}\n```\n---",
        );
        assert!(!truncated);
        assert!(matches!(&blocks[0], Block::Heading(1, value) if value == "Plan"));
        assert!(matches!(&blocks[1], Block::Paragraph(value) if value == "Intro"));
        assert!(
            matches!(&blocks[2], Block::ListItem(0, ListMarker::Bullet, value) if value == "Uno")
        );
        assert!(
            matches!(&blocks[3], Block::ListItem(1, ListMarker::Done, value) if value == "Dos")
        );
        assert!(
            matches!(&blocks[4], Block::ListItem(0, ListMarker::Number(number), value) if number == "3" && value == "Tres")
        );
        assert!(matches!(&blocks[5], Block::Quote(value) if value == "Nota"));
        assert!(
            matches!(&blocks[6], Block::Code(language, value) if language == "rust" && value == "fn main() {}")
        );
        assert_eq!(blocks[7], Block::Rule);
    }

    #[test]
    fn a_list_item_wrapped_by_hand_stays_one_item_until_a_blank_line() {
        let (blocks, _) = parse("- On the desktop, cards with a play button\n  that rises.\n- Another\nlazy too\n\nAfter.");
        assert_eq!(blocks, [
            Block::ListItem(0, ListMarker::Bullet, "On the desktop, cards with a play button that rises.".into()),
            Block::ListItem(0, ListMarker::Bullet, "Another lazy too".into()),
            Block::Paragraph("After.".into()),
        ]);
    }

    #[test]
    fn inline_emphasis_code_and_https_links_are_readable() {
        let runs =
            parse_inline("**Listo**: *revisa* `src/main.rs` y [docs](https://example.com/a).");
        assert_eq!(runs[0].kind, InlineKind::Strong);
        assert_eq!(runs[0].text, "Listo");
        assert!(runs
            .iter()
            .any(|run| run.kind == InlineKind::Emphasis && run.text == "revisa"));
        assert!(runs
            .iter()
            .any(|run| run.kind == InlineKind::Code && run.text == "src/main.rs"));
        let visible: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(visible, "Listo: revisa src/main.rs y docs.");
    }

    #[test]
    fn chat_marks_underline_strike_and_nest() {
        let marks = |source: &str| -> Vec<(InlineKind, String, bool, bool)> {
            parse_inline(source)
                .into_iter()
                .map(|run| (run.kind, run.text, run.underline, run.strikethrough))
                .collect()
        };
        use InlineKind::{Normal, Strong, StrongEmphasis};
        let run = |kind, text: &str, underline, strikethrough| {
            (kind, text.to_owned(), underline, strikethrough)
        };
        assert_eq!(
            marks("__sub__ ~~fuera~~ ***ambas***"),
            [
                run(Normal, "sub", true, false),
                run(Normal, " ", false, false),
                run(Normal, "fuera", false, true),
                run(Normal, " ", false, false),
                run(StrongEmphasis, "ambas", false, false),
            ]
        );
        // Either way around, a mark inside another takes both.
        assert_eq!(marks("__**uno**__"), [run(Strong, "uno", true, false)]);
        assert_eq!(marks("**__dos__**"), [run(Strong, "dos", true, false)]);
        assert_eq!(
            marks("~~**a** *b*~~"),
            [
                run(Strong, "a", false, true),
                run(Normal, " ", false, true),
                run(InlineKind::Emphasis, "b", false, true),
            ]
        );
        assert_eq!(
            marks("**a *b* c**"),
            [
                run(Strong, "a ", false, false),
                run(StrongEmphasis, "b", false, false),
                run(Strong, " c", false, false),
            ]
        );
        // Escaped, unclosed or empty, a mark is text.
        for literal in ["\\~~no~~", "__abierto", "~~ ~", "____"] {
            let text: String = marks(literal).into_iter().map(|run| run.1).collect();
            assert_eq!(text, literal.replace('\\', ""), "{literal}");
        }
        // Each mark once, all around one word.
        assert_eq!(marks("**__~~*x*~~__**"), [run(StrongEmphasis, "x", true, true)]);
        // While written, an open mark already styles its text.
        assert_eq!(
            parse_inline_mode("hola __mun_", true).last().map(|run| (run.underline, run.text.as_str())),
            Some((true, "mun"))
        );
    }

    #[test]
    fn marks_neither_open_nor_close_inside_an_angle_token() {
        let marks = |source: &str| -> Vec<(String, bool)> {
            parse_inline(source)
                .into_iter()
                .map(|run| (run.text, run.underline))
                .collect()
        };
        let run = |text: &str, underline| (text.to_owned(), underline);
        // A chat client's emoji keeps its name whole, and the mark after it
        // still underlines.
        assert_eq!(
            marks("<:pepe__love:1> es __esto__"),
            [run("<:pepe__love:1> es ", false), run("esto", true)]
        );
        assert_eq!(
            marks("<:a__b:1> <:c__d:2>"),
            [run("<:a__b:1> <:c__d:2>", false)]
        );
        // Around one, a mark holds it whole.
        assert_eq!(marks("__a <:b__c:1>__"), [run("a <:b__c:1>", true)]);
        // Spaces, or no closing bracket, make no token.
        assert_eq!(
            marks("<a __b__ c>"),
            [run("<a ", false), run("b", true), run(" c>", false)]
        );
        assert_eq!(marks("<x__y__"), [run("<x", false), run("y", true)]);
        // An autolink is still a link.
        let runs = parse_inline("<https://example.com/a__b>");
        let link = runs.iter().find(|run| run.kind == InlineKind::Link).unwrap();
        assert_eq!(link.destination.as_deref(), Some("https://example.com/a__b"));
    }

    #[test]
    fn subtext_lines_are_small_quiet_blocks() {
        let (blocks, _) = parse("Hola\n-# al pie, **fuerte**\n> cita");
        assert_eq!(
            blocks,
            [
                Block::Paragraph("Hola".into()),
                Block::Subtext("al pie, **fuerte**".into()),
                Block::Quote("cita".into()),
            ]
        );
        assert_eq!(parse("-#sin espacio").0, [Block::Paragraph("-#sin espacio".into())]);
        let element = render_markdown_at_width("-# al pie, **fuerte**", 640.0);
        let mut spans = Vec::new();
        collect_spans(&element, &mut spans);
        let colors = MarkdownColors::for_palette(palette());
        assert!(spans
            .iter()
            .all(|span| span.color == colors.muted && span.font_size == BODY_SIZE * SUBTEXT_SCALE));
        assert_eq!(spans.last().unwrap().font_family, FontFamily::SansBold);
    }

    #[test]
    fn marks_choose_faces_and_lines() {
        let element = render_markdown_at_width("**b** *i* ***bi*** __u__ ~~s~~ `c`", 640.0);
        let mut spans = Vec::new();
        collect_spans(&element, &mut spans);
        let span = |text: &str| spans.iter().find(|span| span.text == text).unwrap();
        assert_eq!(span("b").font_family, FontFamily::SansBold);
        assert_eq!(span("i").font_family, FontFamily::SansItalic);
        assert_eq!(span("bi").font_family, FontFamily::SansBoldItalic);
        assert!(span("u").underline && !span("u").strikethrough);
        assert!(span("s").strikethrough && !span("s").underline);
        assert_eq!(span("c").font_family, FontFamily::Monospace);
    }

    #[test]
    fn a_web_address_stops_before_the_pipes_that_close_a_spoiler() {
        let runs = parse_inline("||https://example.com/a||");
        let link = runs.iter().find(|run| run.kind == InlineKind::Link).unwrap();
        assert_eq!(link.destination.as_deref(), Some("https://example.com/a"));
        let text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(text, "||https://example.com/a||");
    }

    fn collect_spans(element: &Element, out: &mut Vec<TextSpan>) {
        match &element.kind {
            ElementKind::FormattedText(spans) => out.extend(spans.iter().cloned()),
            ElementKind::Column(children) | ElementKind::Row(children) => {
                children.iter().for_each(|child| collect_spans(child, out))
            }
            ElementKind::Panel(child) => collect_spans(child, out),
            _ => {}
        }
    }

    #[test]
    fn unsafe_links_remain_literal_and_escapes_keep_unicode() {
        let source = "[click](javascript:alert(1)) \\*literal\\* áé 🧪";
        let visible: String = parse_inline(source)
            .iter()
            .map(|run| run.text.as_str())
            .collect();
        assert_eq!(visible, "[click](javascript:alert(1)) *literal* áé 🧪");
        assert!(!safe_url("file:///etc/passwd"));
        assert!(!safe_url("https://host/path\nother"));
    }

    #[test]
    fn hard_breaks_and_code_indentation_survive() {
        let (blocks, _) = parse("primera  \nsegunda\n\n```rust\n    let x = 1;\n```");
        assert_eq!(blocks[0], Block::Paragraph("primera\nsegunda".to_owned()));
        assert_eq!(
            blocks[1],
            Block::Code("rust".to_owned(), "    let x = 1;".to_owned())
        );
        let frame = UiTree::new(render_inline("primera\nsegunda", 400.0, BODY_SIZE))
            .layout(Size::new(400.0, 100.0));
        let positions: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, .. }
                    if text == "primera" || text == "segunda" =>
                {
                    Some(position.y)
                }
                _ => None,
            })
            .collect();
        assert_eq!(positions.len(), 2);
        assert!(positions[1] > positions[0]);
    }

    #[test]
    fn narrow_inline_markdown_uses_real_width_and_preserves_unicode() {
        let mut tree = UiTree::new(render_inline("i **WWW** `👩‍💻e\u{301}`", 22.0, BODY_SIZE));
        tree.set_text_measurer(|text, _, family| {
            text.graphemes(true)
                .map(|part| match (part, family) {
                    ("W", _) => 10.0,
                    ("👩‍💻", _) => 14.0,
                    _ => 3.0,
                })
                .sum()
        });
        let frame = tree.layout(Size::new(22.0, 160.0));
        let runs: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    position,
                    text,
                    font_family,
                    ..
                } => Some((position.y, text.as_str(), *font_family)),
                _ => None,
            })
            .collect();
        assert_eq!(
            runs.iter().map(|(_, text, _)| *text).collect::<String>(),
            "i WWW 👩‍💻e\u{301}"
        );
        assert!(runs
            .iter()
            .any(|(_, text, family)| text.contains("👩‍💻") && *family == FontFamily::Monospace));
        assert!(runs
            .iter()
            .any(|(y, text, _)| *y > 0.0 && text.contains('W')));
    }

    #[test]
    fn prose_has_reading_leading_and_distinct_paragraph_list_and_heading_spacing() {
        let source = "First line  \nsecond line\n\nAnother paragraph\n\n- first item\n- second item\n\n## Next section\n\nFinal paragraph";
        for streaming in [false, true] {
            let root = render_cached_markdown(source, 320.0, streaming).element;
            let frame = UiTree::new(root).layout(Size::new(320.0, 800.0));
            let blocks = frame.text_layouts();
            let first = blocks
                .iter()
                .find(|block| block.source.starts_with("First line"))
                .unwrap();
            assert_eq!(first.runs[0].rect.height, BODY_LINE_HEIGHT);
            assert_eq!(
                first.runs[1].rect.y - first.runs[0].rect.y,
                BODY_LINE_HEIGHT
            );
            let top = |source: &str| {
                blocks
                    .iter()
                    .find(|block| block.source == source)
                    .unwrap()
                    .runs[0]
                    .rect
                    .y
            };
            let bottom = |source: &str| {
                let block = blocks.iter().find(|block| block.source == source).unwrap();
                let run = block.runs.last().unwrap();
                run.rect.y + run.rect.height
            };
            assert!(top("Another paragraph") - bottom("First line\nsecond line") >= 14.0);
            assert_eq!(top("second item") - bottom("first item"), 4.0);
            let paint_y = |text: &str| {
                frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text {
                            position,
                            text: value,
                            ..
                        } if value == text => Some(position.y),
                        _ => None,
                    })
                    .unwrap()
            };
            assert_eq!(paint_y("•"), paint_y("first item"));
            assert!(top("Next section") - bottom("second item") >= 20.0);
            assert!(top("Final paragraph") - bottom("Next section") >= 8.0);
        }
    }

    #[test]
    fn resized_list_items_and_quotes_reserve_every_wrapped_line() {
        let markdown = "- Primero: conservar el orden de cada actualización y la respuesta completa con acentos.\n- Segundo: mostrar los archivos y las herramientas sin solapar las líneas del párrafo.\n\n> Tercero: esta cita también debe calcular su altura con el ancho real disponible.\n\n- Cuarto: el siguiente bloque permanece debajo del contenido anterior.";
        // The reading width is an upper bound. Scrollbars, dock animation and
        // parent flex layout can allocate a narrower viewport in this frame.
        let mut tree = UiTree::new(render_markdown_at_width(markdown, 480.0));
        tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 6.5);
        for width in [480.0, 220.0, 360.0, 140.0, 480.0] {
            let frame = tree.layout(Size::new(width, 1600.0));
            let blocks: Vec<_> = frame
                .text_layouts()
                .into_iter()
                .filter(|block| {
                    ["Primero:", "Segundo:", "Tercero:", "Cuarto:"]
                        .iter()
                        .any(|prefix| block.source.starts_with(prefix))
                })
                .collect();
            assert_eq!(blocks.len(), 4);
            for pair in blocks.windows(2) {
                let bottom = pair[0]
                    .runs
                    .iter()
                    .map(|run| run.rect.y + run.rect.height)
                    .fold(0.0_f32, f32::max);
                let next_top = pair[1].runs.first().unwrap().rect.y;
                assert!(
                    next_top >= bottom,
                    "overlapping Markdown blocks at {width}px: {bottom} > {next_top}"
                );
            }
            if width <= 220.0 {
                assert!(blocks[0].runs.len() > 1);
            }
        }
    }

    #[test]
    fn bounded_unicode_response_lays_out_without_markup_delimiters() {
        let markdown = "# Resumen\n\nSe corrigió **la sesión** en `src/main.rs`.\n- [x] Compila\n- [ ] Revisar [guía](https://example.com/docs)";
        let root = render_markdown_at_width(markdown, 360.0);
        let frame = UiTree::new(root).layout(Size::new(360.0, 400.0));
        let text: String = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(text.contains("la sesión"));
        assert!(text.contains("src/main.rs"));
        assert!(text.contains("guía"));
        assert!(!text.contains("https://example.com/docs"));
        assert!(frame
            .regions
            .iter()
            .any(|region| web_link_url_from_action(&region.id).as_deref()
                == Some("https://example.com/docs")));
        assert!(!text.contains("**"));
        assert!(!text.contains("[guía]"));
    }

    #[test]
    fn source_and_block_limits_are_enforced() {
        let (blocks, truncated) = parse(&"x".repeat(MAX_SOURCE_CHARS + 3));
        assert!(truncated);
        assert_eq!(blocks.len(), 1);
        let many = "paragraph\n\n".repeat(MAX_BLOCKS + 5);
        let (blocks, truncated) = parse(&many);
        assert!(truncated);
        assert_eq!(blocks.len(), MAX_BLOCKS);
    }

    #[test]
    fn preview_limits_and_cache_hits_keep_actual_block_limit_separate_from_source_limit() {
        MARKDOWN_CACHES.with(|caches| *caches.borrow_mut() = MarkdownCaches::new());
        let block_limited = "paragraph\n\n".repeat(MAX_BLOCKS + 1);
        assert!(!bounded_source(&block_limited, MAX_SOURCE_CHARS).1);
        let first = render_markdown_preview_at_width(&block_limited, 320.0);
        assert!(first.limited);
        assert!(passive_heap_bytes(&first.element).is_some());
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            assert_eq!(caches.completed.entries.len(), 1);
            let entry = &caches.completed.entries[0];
            assert!(!entry.truncated);
            assert!(entry.limited);
        });
        let cached = render_markdown_preview_at_width(&block_limited, 320.0);
        assert!(cached.limited);
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            assert_eq!(caches.completed.entries.len(), 1);
            assert!(caches.completed.retained_bytes <= MAX_CACHED_BYTES);
        });

        assert!(!render_markdown_preview_at_width("Respuesta breve 🦀", 320.0).limited);
        let exactly_at_limit = "🦀".repeat(MAX_SOURCE_CHARS);
        assert!(!render_markdown_preview_at_width(&exactly_at_limit, 320.0).limited);
        let source_limited = format!("{exactly_at_limit}FIN");
        assert!(render_markdown_preview_at_width(&source_limited, 320.0).limited);
        assert!(render_markdown_preview_at_width(&source_limited, 320.0).limited);
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            let entry = caches.completed.entries.last().unwrap();
            assert!(entry.truncated && entry.limited);
            assert_eq!(entry.source.chars().count(), MAX_SOURCE_CHARS);
            assert!(caches.completed.retained_bytes <= MAX_CACHED_BYTES);
        });
    }

    #[test]
    fn live_preview_cache_keeps_limit_metadata_and_bounded_admission() {
        MARKDOWN_CACHES.with(|caches| *caches.borrow_mut() = MarkdownCaches::new());
        for index in 0..5 {
            let source = format!("{index}{}", "\n\nparagraph".repeat(MAX_BLOCKS + 1));
            assert!(render_streaming_markdown_preview_at_width(&source, 320.0).limited);
            assert!(render_streaming_markdown_preview_at_width(&source, 320.0).limited);
        }
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            // Very large block trees can reach the byte budget before the
            // two-entry limit. They must still retain the newest preview.
            assert!((1..=2).contains(&caches.streaming.entries.len()));
            assert!(caches
                .streaming
                .entries
                .last()
                .unwrap()
                .source
                .starts_with('4'));
            assert!(caches.streaming.entries.iter().all(|entry| entry.limited));
            assert!(caches.streaming.retained_bytes <= MAX_STREAMING_CACHE_BYTES);
            assert!(caches.completed.entries.is_empty());
        });
        let interactive = MarkdownPreview {
            element: Element::button("message-specific-action", "Ver respuesta completa"),
            limited: true,
        };
        assert!(MarkdownCacheEntry::new(
            "source",
            false,
            MarkdownPresentation::at_width(320.0),
            &interactive,
            &[],
            None,
        )
        .is_none());
    }

    #[test]
    fn source_pages_round_trip_unicode_crlf_and_exact_previous_next_offsets() {
        let source = format!(
            "{}\r\n{}{}FIN 🦀",
            "a".repeat(MAX_SOURCE_CHARS - 1),
            "日本語 👩‍💻 e\u{301}\r\n".repeat(3_700),
            "🧪".repeat(MAX_SOURCE_CHARS + 9),
        );
        let mut reconstructed = String::new();
        let mut starts = Vec::new();
        let mut cursor = 0;
        loop {
            let page = markdown_source_page(&source, cursor);
            assert_eq!(page.start_byte, cursor);
            assert!(source.is_char_boundary(page.start_byte));
            assert!(source.is_char_boundary(page.end_byte));
            assert_eq!(page.text, &source[page.start_byte..page.end_byte]);
            assert!(page.text.chars().count() <= MAX_SOURCE_CHARS);
            assert_eq!(page.previous_byte, starts.last().copied());
            if let Some(previous) = page.previous_byte {
                assert_eq!(
                    markdown_source_page(&source, previous).next_byte,
                    Some(cursor)
                );
            }
            if starts.is_empty() {
                assert!(page.text.ends_with('\r'));
                assert!(source[page.next_byte.unwrap()..].starts_with('\n'));
            }
            reconstructed.push_str(page.text);
            starts.push(cursor);
            match page.next_byte {
                Some(next) => {
                    assert_eq!(next, page.end_byte);
                    assert!(next > cursor);
                    cursor = next;
                }
                None => {
                    assert_eq!(page.end_byte, source.len());
                    break;
                }
            }
        }
        assert!(starts.len() > 3);
        assert_eq!(reconstructed, source);
    }

    #[test]
    fn source_page_handles_empty_invalid_utf8_and_stale_cursors() {
        let empty = markdown_source_page("", 0);
        assert_eq!(empty.text, "");
        assert_eq!((empty.start_byte, empty.end_byte), (0, 0));
        assert_eq!((empty.previous_byte, empty.next_byte), (None, None));
        let source = format!("🦀{}", "x".repeat(MAX_SOURCE_CHARS + 3));
        for cursor in [1, 2, 3, source.len() + 1, usize::MAX] {
            let reset = markdown_source_page(&source, cursor);
            assert_eq!(reset.start_byte, 0);
            assert_eq!(reset.previous_byte, None);
            assert!(reset.text.starts_with('🦀'));
        }
        let end = markdown_source_page(&source, source.len());
        assert_eq!(end.text, "");
        assert_eq!(end.next_byte, None);
        assert!(end.previous_byte.is_some());
    }

    #[test]
    fn source_render_is_bounded_to_forty_rows_and_bypasses_markdown_block_limit() {
        let source = "🦀".repeat(MAX_SOURCE_CHARS + 1);
        let page = markdown_source_page(&source, 0);
        let rendered = render_markdown_source_page_at_width(&page, 320.0);
        let ElementKind::Column(rows) = &rendered.kind else {
            panic!("source pages render as a passive row column");
        };
        assert_eq!(rows.len(), MAX_SOURCE_CHARS.div_ceil(MAX_SOURCE_ROW_CHARS));
        assert_eq!(rows.len(), 40);
        let mut reconstructed = String::new();
        for row in rows {
            let ElementKind::Text(text) = &row.kind else {
                panic!("source rows remain literal text");
            };
            assert!(text.chars().count() <= MAX_SOURCE_ROW_CHARS);
            reconstructed.push_str(text);
        }
        assert_eq!(reconstructed, page.text);
        assert!(passive_heap_bytes(&rendered).is_some());

        let blocks = (0..MAX_BLOCKS + 9)
            .map(|index| format!("## bloque {index}\n\n"))
            .collect::<String>();
        assert!(render_markdown_preview_at_width(&blocks, 320.0).limited);
        let page = markdown_source_page(&blocks, 0);
        assert_eq!(page.next_byte, None);
        let rendered = render_markdown_source_page_at_width(&page, 320.0);
        let ElementKind::Column(rows) = rendered.kind else {
            panic!("source pages render as a passive row column");
        };
        let text = rows
            .iter()
            .filter_map(|row| match &row.kind {
                ElementKind::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(text.contains(&format!("## bloque {}", MAX_BLOCKS + 8)));
        assert!(!text.contains(MarkdownLabels::DEFAULT.truncated_note));
    }

    fn visible_text(element: Element) -> String {
        UiTree::new(element)
            .layout(Size::new(640.0, 2_000.0))
            .text_layouts()
            .iter()
            .map(|layout| layout.source.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn live_inline_tokens_keep_their_style_and_terminal_render_uses_authoritative_source() {
        for (source, expected, kind) in [
            ("**acción 👩‍💻", "acción 👩‍💻", InlineKind::Strong),
            ("**acción*", "acción", InlineKind::Strong),
            ("*énfasis", "énfasis", InlineKind::Emphasis),
            ("`src/main.rs", "src/main.rs", InlineKind::Code),
            ("[guía](https://exa", "guía", InlineKind::Link),
            ("[guía]", "guía", InlineKind::Link),
        ] {
            let live = parse_inline_mode(source, true);
            assert_eq!(
                live,
                vec![InlineRun::new(kind, expected)],
                "{source}"
            );
            let final_text: String = parse_inline(source)
                .iter()
                .map(|run| run.text.as_str())
                .collect();
            let final_expected = if source == "**acción*" {
                "*acción"
            } else {
                source
            };
            assert_eq!(
                final_text, final_expected,
                "only the live tail may synthesize completion"
            );
            assert_eq!(
                visible_text(render_streaming_markdown_at_width(source, 640.0)),
                expected
            );
            assert_eq!(
                visible_text(render_markdown_at_width(source, 640.0)),
                final_expected
            );
        }
        assert_eq!(
            parse_inline_mode("\\*literal", true),
            vec![InlineRun::new(InlineKind::Normal, "*literal")]
        );
        assert_eq!(parse_inline_mode("2 * 3", true), parse_inline("2 * 3"));
        assert_eq!(
            parse_inline_mode("[x](javascript:bad)", true),
            parse_inline("[x](javascript:bad)")
        );
    }

    #[test]
    fn completed_blocks_do_not_inherit_provisional_formatting_from_the_live_tail() {
        let source = "**literal\n\nAhora *énfasis";
        let live = visible_text(render_streaming_markdown_at_width(source, 640.0));
        assert!(live.starts_with("**literal\n"), "{live}");
        assert!(live.ends_with("Ahora énfasis"), "{live}");
        let closed = visible_text(render_streaming_markdown_at_width("**literal\n\n", 640.0));
        assert_eq!(closed, "**literal");
        assert_eq!(
            visible_text(render_streaming_markdown_at_width("**literal\n\n``", 640.0)),
            "**literal"
        );
        let limited = format!("{}\n\nactual tail", "**literal\n\n".repeat(MAX_BLOCKS));
        let rendered = visible_text(render_streaming_markdown_at_width(&limited, 640.0));
        assert!(rendered
            .lines()
            .filter(|line| line.contains("literal"))
            .all(|line| line == "**literal"));
    }

    #[test]
    fn fences_respect_marker_length_language_and_literal_embedded_fences() {
        let source = "````markdown\n```rust\nlet value = 1;\n```\n````\n\n~~~text\n``` is content\n~~~ trailing text\n~~~\nAfter";
        let (blocks, limited) = parse(source);
        assert!(!limited);
        assert_eq!(
            blocks,
            vec![
                Block::Code("markdown".into(), "```rust\nlet value = 1;\n```".into()),
                Block::Code("text".into(), "``` is content\n~~~ trailing text".into()),
                Block::Paragraph("After".into()),
            ]
        );
        assert_eq!(
            parse("```rust extra-info\n  ñ 🦀\n```").0,
            vec![Block::Code("rust".into(), "  ñ 🦀".into())]
        );
        assert_eq!(
            parse("```not`a`fence").0,
            vec![Block::Paragraph("```not`a`fence".into())]
        );
    }

    #[test]
    fn split_fence_openers_do_not_flash_as_prose_and_never_modify_retained_source() {
        for marker in ["`", "``", "~", "~~"] {
            let source = format!("Introducción\n\n{marker}");
            assert_eq!(
                visible_text(render_streaming_markdown_at_width(&source, 640.0)),
                "Introducción"
            );
            assert!(visible_text(render_markdown_at_width(&source, 640.0)).ends_with(marker));
        }
        let source = "Introducción\n\n```rust\nfn main() {}\n```\n\n**Final 👩‍💻**";
        for end in source
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(source.len()))
        {
            let prefix = &source[..end];
            let _ = visible_text(render_streaming_markdown_at_width(prefix, 360.0));
            assert_eq!(prefix, &source[..end]);
        }
        assert_eq!(
            visible_text(render_streaming_markdown_at_width(source, 640.0)),
            visible_text(render_markdown_at_width(source, 640.0))
        );
    }

    #[test]
    fn live_append_reuses_completed_blocks_and_only_reparses_changed_tail() {
        MARKDOWN_CACHES.with(|caches| *caches.borrow_mut() = MarkdownCaches::new());
        INLINE_PARSE_COUNT.with(|count| count.set(0));
        let source =
            "# Encabezado\n\nPárrafo **estable**\n\n```rust\nfn main() {}\n```\n\nCola **vi";
        let _ = render_streaming_markdown_at_width(source, 640.0);
        assert_eq!(INLINE_PARSE_COUNT.with(|count| count.get()), 3);
        INLINE_PARSE_COUNT.with(|count| count.set(0));
        let appended = format!("{source}va**");
        let appended_text = visible_text(render_streaming_markdown_at_width(&appended, 640.0));
        assert!(appended_text.ends_with("Cola viva"));
        assert_eq!(
            INLINE_PARSE_COUNT.with(|count| count.get()),
            1,
            "unchanged heading and paragraph should not parse again"
        );
        let _ = render_streaming_markdown_at_width(&appended, 640.0);
        assert_eq!(
            INLINE_PARSE_COUNT.with(|count| count.get()),
            1,
            "same-frame hit needs no parsing"
        );
        let _ = render_streaming_markdown_at_width(&appended, 360.0);
        assert_eq!(
            INLINE_PARSE_COUNT.with(|count| count.get()),
            4,
            "width changes rebuild every block"
        );
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            assert!(caches.streaming.retained_bytes <= MAX_STREAMING_CACHE_BYTES);
            assert!(caches.completed.entries.is_empty());
        });
    }

    #[test]
    fn long_unmatched_inline_code_delimiters_remain_literal_and_bounded() {
        let source = format!("Texto {} fin", "`".repeat(MAX_SOURCE_CHARS - 20));
        assert_eq!(
            parse_inline(&source),
            vec![InlineRun::new(InlineKind::Normal, source.clone())]
        );
        assert_eq!(parse_inline_mode(&source, true), parse_inline(&source));
    }

    #[test]
    fn web_destinations_reject_unsafe_schemes_and_spoofed_github_hosts() {
        for url in ["https://github.com/a/b", "HTTPS://GITHUB.COM/a", "https://www.github.com/a"] {
            assert_eq!(link_icon(&web_url(url).unwrap()), Icon::Github);
        }
        for url in [
            "https://github.com.evil.test/a",
            "https://evilgithub.com/a",
            "https://docs.github.com/a",
            "http://localhost:3000/?url=github.com",
        ] {
            assert_eq!(link_icon(&web_url(url).unwrap()), Icon::Globe);
        }
        for url in [
            "file:///tmp/a",
            "javascript:alert(1)",
            "https://github.com@evil.test/a",
            "https://user:password@github.com/a",
            "https://github.com\\@evil.test",
            "src/main.rs",
            "https://example.test/\nnext",
        ] {
            assert!(web_url(url).is_none(), "{url}");
        }
    }

    #[test]
    fn markdown_urls_keep_labels_balanced_parentheses_and_literal_file_paths() {
        let source = "Mirá [repositorio](https://github.com/a/b) y https://example.test/a_(b). `https://code.test` [archivo](src/main.rs)";
        let runs = parse_inline_mode(source, false);
        let links: Vec<_> = runs
            .iter()
            .filter_map(|run| run.destination.as_deref())
            .collect();
        assert_eq!(
            links,
            ["https://github.com/a/b", "https://example.test/a_(b)"]
        );
        let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
        assert!(text.contains("Mirá repositorio y https://example.test/a_(b)."));
        assert!(!text.contains("repositorio (https"));
        assert!(text.contains("[archivo](src/main.rs)"));
        let nested = parse_inline_mode("[manual](https://example.test/a_(b))", false);
        assert_eq!(
            nested[0].destination.as_deref(),
            Some("https://example.test/a_(b)")
        );
        assert_eq!(
            url_token_len(&format!("https://example.test/{}", ")".repeat(30_000))),
            0
        );
    }

    #[test]
    fn partial_stream_urls_become_actionable_only_when_the_target_is_complete() {
        for source in [
            "Ver https://github.",
            "Ver https://github.co",
            "Ver https://github.com/a/b",
        ] {
            assert!(actions(render_streaming_markdown_at_width(source, 420.0)).is_empty());
            assert!(parse_inline_mode(source, true)
                .iter()
                .any(|run| run.kind == InlineKind::Link));
        }
        let source = "Ver https://github.com/a/b ";
        let links = actions(render_streaming_markdown_at_width(source, 420.0));
        assert!(!links.is_empty());
        assert!(links
            .iter()
            .all(|id| web_link_url_from_action(id).as_deref() == Some("https://github.com/a/b")));
        assert!(!actions(render_markdown_at_width(
            "Ver https://github.com/a/b",
            420.0
        ))
        .is_empty());
        assert!(!actions(render_streaming_markdown_at_width(
            "[repo](https://github.com/a/b)",
            420.0
        ))
        .is_empty());
    }

    #[test]
    fn a_host_reads_a_link_and_its_glyph_as_the_renderer_does() {
        assert_eq!(
            Markdown::web_link("https://GitHub.com/a/b").as_deref(),
            Some("https://github.com/a/b")
        );
        for refused in ["file:///etc/passwd", "https://user@example.com", "https://a b", ""] {
            assert_eq!(Markdown::web_link(refused), None);
        }
        assert_eq!(Markdown::link_icon("https://www.github.com/a"), Icon::Github);
        assert_eq!(Markdown::link_icon("https://example.com"), Icon::Globe);
        assert_eq!(Markdown::link_icon("no es una dirección"), Icon::Globe);
    }

    #[test]
    fn bounded_detection_and_markdown_never_open_a_truncated_host() {
        let prefix = "x".repeat(MAX_SOURCE_CHARS - " https://github.co".chars().count());
        let source = format!("{prefix} https://github.com/a/b");
        let draft_links = Markdown::web_links(&source);
        assert_eq!(draft_links.len(), 1);
        assert_eq!(draft_links[0].1, "https://github.com/a/b");
        let preview = render_markdown_preview_at_width(&source, 400.0);
        assert!(preview.limited);
        assert!(actions(preview.element).is_empty());
    }

    fn commands(element: Element, width: f32) -> Vec<DrawCommand> {
        let mut tree = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(width, 2_000.0)).commands
    }

    fn ink(commands: &[DrawCommand], value: &str) -> Color {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text.trim() == value => Some(*color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {value:?}"))
    }

    #[test]
    fn colors_labels_and_the_link_prefix_come_from_the_host() {
        let colors = MarkdownColors {
            text: Color::rgb(1, 1, 1),
            strong: Color::rgb(2, 2, 2),
            emphasis: Color::rgb(3, 3, 3),
            link: Color::rgb(4, 4, 4),
            code_ink: Color::rgb(5, 5, 5),
            code_fill: Color::rgb(6, 6, 6),
            muted: Color::rgb(7, 7, 7),
            success: Color::rgb(8, 8, 8),
            panel: Color::rgb(9, 9, 9),
            border: Color::rgb(10, 10, 10),
        };
        let source = "uno **dos** *tres* `cuatro` [cinco](https://example.test/a)\n\n- [x] seis\n- siete\n\n> ocho\n\n---\n\n```\nnueve\n```";
        let build = || {
            Markdown::new(source, palette())
                .width(400.0)
                .colors(colors)
                .labels(MarkdownLabels {
                    code_fallback: "CÓDIGO",
                    truncated_note: "Cortado.",
                    ..MarkdownLabels::DEFAULT
                })
                .link_action_prefix("open:")
                .limits(MarkdownLimits { source_chars: 20_000, blocks: 8 })
        };
        let preview = build().render();
        assert!(!preview.limited);
        let frame = UiTree::new(preview.element).layout(Size::new(400.0, 2_000.0));
        let painted = frame.commands;
        for (text, color) in [
            ("uno", colors.text),
            ("dos", colors.strong),
            ("tres", colors.emphasis),
            ("cuatro", colors.code_ink),
            ("cinco", colors.link),
            ("✓", colors.success),
            ("•", colors.muted),
            ("CÓDIGO", palette().muted),
            ("nueve", palette().text),
        ] {
            assert_eq!(ink(&painted, text), color, "{text}");
        }
        let fills: Vec<_> = painted
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        for fill in [colors.code_fill, colors.muted, colors.panel, colors.border] {
            assert!(fills.contains(&fill), "{fill:?}");
        }
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id.starts_with("open:") && region.id.ends_with(":https://example.test/a")));

        // Other colors are another cache entry, not a hit on this one.
        let plain = Markdown::new(source, palette()).width(400.0).render().element;
        assert_eq!(ink(&commands(plain, 400.0), "✓"), MarkdownColors::for_palette(palette()).success);

        let cut = build().limits(MarkdownLimits { source_chars: 20_000, blocks: 2 }).render();
        assert!(cut.limited);
        assert_eq!(ink(&commands(cut.element, 400.0), "Cortado."), colors.muted);
    }

    #[test]
    fn code_blocks_drop_the_label_fit_their_code_and_offer_a_copy_icon() {
        let width = 480.0;
        let source = "Antes `saludo.py`:\n\n```python\nprint(\"hola\")\n```";
        let frame = |markdown: Markdown| UiTree::new(markdown.render().element).layout(Size::new(width, 600.0));
        let block = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == palette().raised => Some(*rect),
                    _ => None,
                })
                .unwrap()
        };
        let has_text = |frame: &crate::Frame, value: &str| {
            frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == value))
        };
        let desktop = |source| {
            Markdown::new(source, palette())
                .width(width)
                .code_label(false)
                .code_fit_content(true)
                .code_copy_action_prefix("copy:")
        };

        // Unless asked: the label, the whole width and no button.
        let plain = frame(Markdown::new(source, palette()).width(width));
        assert!(has_text(&plain, "PYTHON"));
        assert_eq!(block(&plain).width, width);
        assert!(plain.regions.iter().all(|region| !region.id.starts_with("copy:")));

        let fitted = frame(desktop(source));
        assert!(!has_text(&fitted, "PYTHON"));
        let rect = block(&fitted);
        assert!(rect.width < width / 2.0, "{rect:?}");
        let copy = fitted.regions.iter().find(|region| region.id == "copy:print(\"hola\")").unwrap();
        // At the end of a one-line block, in its middle.
        assert!((copy.rect.x + copy.rect.width - (rect.x + rect.width - 12.0)).abs() < 0.5, "{copy:?} in {rect:?}");
        assert!((copy.rect.y + copy.rect.height / 2.0 - (rect.y + rect.height / 2.0)).abs() < 0.5, "{copy:?} in {rect:?}");
        assert!(fitted.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::Copy, .. })));
        // On one line: the width handed back is the width measured.
        let top = |value: &str| {
            fitted.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == value => Some(position.y),
                _ => None,
            })
        };
        assert_eq!(top("print("), top(")"));
        // Inline code keeps its own ink and fill.
        let colors = MarkdownColors::for_palette(palette());
        assert_eq!(ink(&fitted.commands, "saludo.py"), colors.code_ink);
        assert!(fitted.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == colors.code_fill)));

        // Code wider than the column still takes only the column, and wraps.
        let long = format!("```\n{}\n```", "palabra ".repeat(120));
        assert_eq!(block(&frame(desktop(&long))).width, width);
        assert!(block(&frame(desktop(&long).code_word_wrap(false))).width <= width);
    }

    #[test]
    fn a_host_sets_the_code_inks_and_tells_functions_and_interpolations_apart() {
        let desktop = CodeSyntaxColors {
            text: Color::rgb(0x19, 0x19, 0x19),
            keyword: Color::rgb(0xB0, 0x5A, 0x00),
            string: Color::rgb(0x1E, 0x9E, 0x3C),
            number: Color::rgb(0x0A, 0x6E, 0x6E),
            comment: Color::rgb(0x70, 0x70, 0x70),
            function: Some(Color::rgb(0x00, 0x73, 0xE6)),
            interpolation: Some(Color::rgb(0xCD, 0x20, 0x54)),
            types: None,
            properties: None,
        };
        let runs = |source: &str, colors: Option<CodeSyntaxColors>| {
            let mut markdown = Markdown::new(source, palette());
            if let Some(colors) = colors {
                markdown = markdown.code_syntax_colors(colors);
            }
            code_spans(&markdown.render().element)
                .unwrap()
                .into_iter()
                .map(|span| (span.text, span.color))
                .collect::<Vec<_>>()
        };
        let d = desktop;
        let (function, braces) = (d.function.unwrap(), d.interpolation.unwrap());
        assert_eq!(
            runs("```python\nprint(f\"Hola, {nombre}! {{x}} Bienvenido.\")  # hi\n```", Some(d)),
            [
                ("print".into(), function),
                ("(f".into(), d.text),
                ("\"Hola, ".into(), d.string),
                ("{".into(), braces),
                ("nombre".into(), d.text),
                ("}".into(), braces),
                ("! {{x}} Bienvenido.\"".into(), d.string),
                (")  ".into(), d.text),
                ("# hi".into(), d.comment),
            ]
        );
        assert_eq!(
            runs("```js\nconst n = len(`a ${b} c`, 2)\n```", Some(d)),
            [
                ("const".into(), d.keyword),
                (" n = ".into(), d.text),
                ("len".into(), function),
                ("(".into(), d.text),
                ("`a ".into(), d.string),
                ("${".into(), braces),
                ("b".into(), d.text),
                ("}".into(), braces),
                (" c`".into(), d.string),
                (", ".into(), d.text),
                ("2".into(), d.number),
                (")".into(), d.text),
            ]
        );
        // A plain string is not an f-string; a call alone is still colored.
        assert_eq!(runs("```py\nx(\"{y}\")\n```", Some(d))[2], ("\"{y}\"".into(), d.string));
        assert_eq!(runs("```py\nrun()\n```", Some(d))[0], ("run".into(), function));
        // Unless set: the palette's inks, with the call and the braces in their run.
        let ours = CodeSyntaxColors::for_palette(palette());
        assert_eq!(
            runs("```python\nprint(f\"{n}\")\n```", None),
            [("print(f".into(), ours.text), ("\"{n}\"".into(), ours.string), (")".into(), ours.text)]
        );
    }

    #[test]
    fn inline_code_takes_its_own_size_in_proportion_to_its_text() {
        let sizes = |markdown: Markdown| {
            let mut spans = Vec::new();
            collect_spans(&markdown.render().element, &mut spans);
            spans.into_iter().filter(|span| span.font_family == FontFamily::Monospace).map(|span| span.font_size).collect::<Vec<_>>()
        };
        let source = "Run `ls`.";
        assert_eq!(sizes(Markdown::new(source, palette()).font_size(16.0)), [16.0]);
        assert_eq!(sizes(Markdown::new(source, palette()).font_size(16.0).inline_code_size(13.0)), [13.0]);
        for bad in [0.0, -1.0, f32::NAN] {
            assert_eq!(sizes(Markdown::new(source, palette()).font_size(16.0).inline_code_size(bad)), [16.0]);
        }
        let heading = sizes(Markdown::new("# A `b`", palette()).font_size(16.0).inline_code_size(13.0));
        let plain_heading = sizes(Markdown::new("# A `b`", palette()).font_size(16.0));
        assert!((heading[0] - plain_heading[0] * 13.0 / 16.0).abs() < 0.001, "{heading:?}");
    }

    #[test]
    fn plain_source_keeps_its_markup_and_links_only_complete_addresses() {
        let source = "ñ 👩‍💻 **no** [repo](https://github.com/a/b), https://example.test/docs y src/main.rs";
        let preview = Markdown::new(source, palette())
            .plain_with_autolinks(21.0)
            .plain_with_autolinks(f32::NAN)
            .plain_with_autolinks(-4.0)
            .render();
        assert!(!preview.limited);
        assert_eq!(preview.element.style.line_height, Some(21.0));
        let frame = UiTree::new(preview.element).layout(Size::new(640.0, 500.0));
        assert_eq!(frame.text_layouts()[0].source, source);
        let links: Vec<_> = frame
            .regions
            .iter()
            .filter_map(|region| web_link_url_from_action(&region.id))
            .collect();
        assert!(links.contains(&"https://github.com/a/b".to_owned()));
        assert!(links.contains(&"https://example.test/docs".to_owned()));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon { icon: Icon::Github, .. }
        )));
    }

    /// The spans of the first code block in a tree; `None` when its code is plain text.
    fn code_spans(element: &Element) -> Option<Vec<TextSpan>> {
        match &element.kind {
            ElementKind::FormattedText(spans) => Some(spans.clone()),
            ElementKind::Column(children) | ElementKind::Row(children) => {
                children.iter().find_map(code_spans)
            }
            ElementKind::Panel(child) | ElementKind::HorizontalScroll { child, .. } => {
                code_spans(child)
            }
            _ => None,
        }
    }

    fn joined(spans: &[TextSpan]) -> String {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn fenced_code_is_colored_by_its_language_and_keeps_its_text() {
        let code = "// note\nfn main() { let s = \"hi\"; }";
        let spans = code_spans(&render_markdown_at_width(
            &format!("```rust\n{code}\n```"),
            480.0,
        ))
        .expect("rust is highlighted");
        assert_eq!(joined(&spans), code);
        let ink = |text: &str| spans.iter().find(|span| span.text == text).unwrap().color;
        let (comment, keyword, string) = (ink("// note"), ink("fn"), ink("\"hi\""));
        assert_eq!(ink(" main() { "), palette().text);
        for (a, b) in [(comment, keyword), (keyword, string), (string, comment)] {
            assert_ne!(a, b);
        }
        assert!([comment, keyword, string]
            .iter()
            .all(|color| *color != palette().text));
        assert!(spans
            .iter()
            .all(|span| span.font_family == FontFamily::Monospace
                && span.font_size == TextRole::Caption.size()));
        let scaled = code_spans(
            &Markdown::new(&format!("```rust\n{code}\n```"), palette())
                .font_size(21.0)
                .render()
                .element,
        )
        .unwrap();
        assert!(scaled
            .iter()
            .all(|span| span.font_size == TextRole::Caption.size() * 1.5));

        // The aliases a fence is written with, in every language the lexer knows.
        for (fence, code) in [
            ("rs", "let a = 1;"),
            ("js", "const a = 1;"),
            ("javascript", "return null;"),
            ("ts", "let a: number = 1;"),
            ("tsx", "export default 1;"),
            ("py", "def f(): pass"),
            ("python", "# note"),
            ("json", "{\"a\": true}"),
            ("toml", "a = 1 # note"),
            ("css", "/* note */ a { color: red; }"),
            ("html", "<a href=\"x\">"),
            ("md", "```rust\nfn f() {}\n```"),
        ] {
            let source = format!("````{fence}\n{code}\n````");
            let spans = code_spans(&render_markdown_at_width(&source, 480.0))
                .unwrap_or_else(|| panic!("{fence} is highlighted"));
            assert_eq!(joined(&spans), code, "{fence}");
            assert!(
                spans.iter().any(|span| span.color != palette().text),
                "{fence}"
            );
        }
    }

    #[test]
    fn unknown_or_missing_languages_and_code_with_nothing_to_color_stay_plain() {
        for source in [
            "```\nfn main() {}\n```",
            "```brainfuck\nfn main() {}\n```",
            "```text\nfn main() {}\n```",
            "```rust\nmain()\n```",
        ] {
            let element = render_markdown_at_width(source, 480.0);
            assert!(code_spans(&element).is_none(), "{source}");
            let code = source.lines().nth(1).unwrap();
            assert!(commands(element, 480.0).iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, color, .. } if text == code && *color == palette().text
            )));
        }
    }

    #[test]
    fn code_past_the_highlight_bounds_stays_plain() {
        use crate::code_syntax::{MAX_HIGHLIGHT_BYTES, MAX_HIGHLIGHT_LINES};
        let limits = MarkdownLimits {
            source_chars: 4 * MAX_HIGHLIGHT_BYTES,
            blocks: MAX_BLOCKS,
        };
        let spans = |code: &str| {
            code_spans(
                &Markdown::new(&format!("```rust\n{code}\n```"), palette())
                    .limits(limits)
                    .render()
                    .element,
            )
        };
        let lines = |count: usize| vec!["let x = 1;"; count].join("\n");
        assert_eq!(
            joined(&spans(&lines(MAX_HIGHLIGHT_LINES)).unwrap()),
            lines(MAX_HIGHLIGHT_LINES)
        );
        assert!(spans(&lines(MAX_HIGHLIGHT_LINES + 1)).is_none());
        // `let x = "` and `";` around the filler.
        let long = |bytes: usize| format!("let x = \"{}\";", "a".repeat(bytes - 11));
        assert_eq!(long(MAX_HIGHLIGHT_BYTES).len(), MAX_HIGHLIGHT_BYTES);
        assert!(spans(&long(MAX_HIGHLIGHT_BYTES)).is_some());
        assert!(spans(&long(MAX_HIGHLIGHT_BYTES + 1)).is_none());
    }

    #[test]
    fn code_inks_read_on_dark_and_light_palettes_and_keep_the_block_height() {
        let light = crate::Theme {
            background: Color::rgb(255, 255, 255),
            foreground: Color::rgb(24, 26, 31),
            accent: Color::rgb(9, 105, 218),
        };
        let code = "// note\nlet s = \"hi\"; 42";
        let mut inks = Vec::new();
        let palettes = [
            ComponentPalette::neutral_dark(),
            crate::Theme::DARK.palette(),
            light.palette(),
            light.scheme().card(),
        ];
        for palette in palettes {
            let source = format!("```rust\n{code}\n```");
            let element = Markdown::new(&source, palette)
                .width(480.0)
                .render()
                .element;
            let colored: Vec<_> = code_spans(&element)
                .unwrap()
                .into_iter()
                .filter(|span| span.color != palette.text)
                .map(|span| span.color)
                .collect();
            assert_eq!(colored.len(), 4, "comment, keyword, string and number");
            for color in &colored {
                assert!(
                    crate::chip::contrast(*color, palette.raised) >= 4.5,
                    "{color:?} on {:?}",
                    palette.raised
                );
            }
            inks.push(colored);
            // Colored code is set on the same lines as plain code.
            let height = |element: Element| {
                commands(element, 480.0)
                    .into_iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, color, .. } if color == palette.raised => {
                            Some(rect.height)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let plain = format!("```\n{code}\n```");
            let plain = Markdown::new(&plain, palette).width(480.0).render().element;
            assert_eq!(height(element), height(plain));
        }
        assert_eq!(inks[0], inks[1]);
        assert_eq!(inks[2], inks[3]);
        assert!(inks[0]
            .iter()
            .zip(&inks[2])
            .all(|(dark, light)| dark != light));
    }

    #[test]
    fn a_closed_mermaid_flowchart_is_drawn_and_anything_else_stays_code() {
        let chart = "```mermaid\nflowchart LR\n  A[Uno] --> B[Dos]\n```\n";
        let lines = |commands: &[DrawCommand]| {
            commands.iter().filter(|command| matches!(command, DrawCommand::Polyline { .. })).count()
        };
        let texts = |commands: &[DrawCommand]| {
            commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("|")
        };
        let drawn = commands(render_markdown_at_width(chart, 480.0), 480.0);
        assert_eq!(lines(&drawn), 1);
        assert_eq!(texts(&drawn), "DIAGRAM|Uno|Dos");

        // Still being written, of another kind, or not Mermaid at all.
        for source in [
            "```mermaid\nflowchart LR\n  A[Uno] --> B[Dos]",
            "```mermaid\nsequenceDiagram\n  A->>B: hola",
            "```mermaid\nstateDiagram-v2\n  [*] --> Activo\n```\n",
            "```text\nflowchart LR\n  A --> B\n```\n",
        ] {
            for streaming in [false, true] {
                let code = commands(render_cached_markdown(source, 480.0, streaming).element, 480.0);
                assert_eq!(lines(&code), 0, "{source}");
                assert!(texts(&code).contains("-->") || texts(&code).contains("->>"), "{source}");
            }
        }
        // Two identical charts share an id and still lay out side by side.
        let twice = commands(render_markdown_at_width(&format!("{chart}\n{chart}"), 480.0), 480.0);
        assert_eq!(lines(&twice), 2);
        // The same tree is reused once the fence closes mid-stream.
        let streamed = commands(render_streaming_markdown_at_width(chart, 480.0), 480.0);
        assert_eq!(lines(&streamed), 1);

        // With a prefix the card carries the action; a listed id shows the source.
        let id = source_hash("flowchart LR\n  A[Uno] --> B[Dos]");
        let action = format!("diagram:{id:016x}");
        let render = |ids: &[u64]| {
            Markdown::new(chart, palette())
                .width(480.0)
                .diagram_action_prefix("diagram:")
                .diagram_sources(ids)
                .render()
                .element
        };
        let actions = |element: Element| {
            let regions = UiTree::new(element).layout(Size::new(640.0, 2000.0)).regions;
            regions
                .into_iter()
                .filter_map(|region| region.id.starts_with("diagram:").then_some(region.id))
                .collect::<Vec<_>>()
        };
        // A drawn diagram can also be opened over the window, from its source.
        assert_eq!(actions(render(&[])), [format!("diagram:expand:{id:016x}"), action.clone()]);
        assert_eq!(Markdown::diagram_source(id).as_deref(), Some("flowchart LR\n  A[Uno] --> B[Dos]"));
        let diagram = commands(render(&[]), 480.0);
        assert_eq!((lines(&diagram), texts(&diagram).as_str()), (1, "DIAGRAM|Expand|Source|Uno|Dos"));
        assert_eq!(actions(render(&[id])), [action]);
        let source = commands(render(&[id]), 480.0);
        assert_eq!(lines(&source), 0);
        assert!(texts(&source).starts_with("MERMAID|Diagram|flowchart LR"));
        // Both views are passive trees the cache keeps.
        MARKDOWN_CACHES.with(|caches| {
            let caches = caches.borrow();
            assert!(caches.completed.entries.iter().filter(|entry| entry.source == chart).count() >= 2);
        });
    }

    #[test]
    fn a_closed_mermaid_sequence_diagram_is_drawn_in_the_same_card_and_alone_for_a_host() {
        let source = "sequenceDiagram\n  A->>B: hola\n  B-->>A: chau";
        let fence = format!("```mermaid\n{source}\n```\n");
        let texts = |element: Element| {
            commands(element, 480.0)
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("|")
        };
        let markdown = |source| Markdown::new(source, palette()).width(480.0).diagram_action_prefix("diagram:");
        let drawn = texts(markdown(&fence).render().element);
        assert_eq!(drawn, "DIAGRAM|Expand|Source|hola|chau|A|A|B|B");
        assert_eq!(Markdown::diagram_source(source_hash(source)).as_deref(), Some(source));
        // What the fence holds gives the same card to a host that reads its documents itself.
        assert_eq!(texts(markdown(source).diagram().expect("a sequence diagram")), drawn);
        let id = [source_hash(source)];
        let as_source = texts(markdown(source).diagram_sources(&id).diagram().unwrap());
        assert!(as_source.starts_with("MERMAID|Diagram|sequenceDiagram"), "{as_source}");
        assert!(texts(markdown("flowchart LR\n  A[Uno] --> B[Dos]").diagram().unwrap()).ends_with("Uno|Dos"));
        for other in ["", "texto", "classDiagram\n  A <|-- B", "sequenceDiagram\n  A->>B"] {
            assert!(markdown(other).diagram().is_none(), "{other:?} was drawn");
        }
    }

    #[test]
    fn bad_widths_and_zero_limits_cannot_reach_layout() {
        for width in [f32::NAN, f32::INFINITY, -80.0, 0.0, 1.0e9] {
            let markdown = Markdown::new("# Título\n\ntexto", palette()).width(width);
            let used = markdown.presentation.width();
            assert!((48.0..=1_200.0).contains(&used), "{width} became {used}");
        }
        let cut = Markdown::new("uno\n\ndos", palette())
            .limits(MarkdownLimits { source_chars: 0, blocks: 0 })
            .render();
        assert!(cut.limited);
        assert_eq!(visible_text(cut.element).lines().next(), Some("u"));
    }

    #[test]
    fn nothing_escapes_a_narrow_column() {
        let source = "# Encabezado largo que debe partirse\n\nUn párrafo con `código`, **énfasis** y un [enlace](https://example.test/una/ruta/larga).\n\n- uno\n  - dos\n    - tres\n      - cuatro, bien adentro\n\n> cita\n\n```rust\nfn main() {}\n```";
        for (width, size) in [48.0, 120.0, 360.0, 900.0]
            .into_iter()
            .flat_map(|width| [(width, 14.0), (width, 28.0), (width, 48.0)])
        {
            let element = Markdown::new(source, palette())
                .width(width)
                .font_size(size)
                .render()
                .element;
            for command in commands(element, width) {
                if let DrawCommand::Text { position, .. } = command {
                    assert!(
                        position.x >= 0.0 && position.x < width,
                        "{width} at {size}: {position:?}"
                    );
                }
            }
        }
        // Where there is room, each level still steps in by its scaled indent.
        let wide = commands(
            Markdown::new(source, palette())
                .width(900.0)
                .font_size(28.0)
                .render()
                .element,
            900.0,
        );
        let x = |value: &str| {
            wide.iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text.trim() == value => {
                        Some(position.x)
                    }
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(x("dos") - x("uno"), 28.0);
        assert_eq!(x("tres") - x("dos"), 28.0);
    }

    fn painted_size(commands: &[DrawCommand], value: &str) -> f32 {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text, font_size, ..
                } if text.trim() == value => Some(*font_size),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {value:?}"))
    }

    #[test]
    fn font_size_scales_the_body_and_everything_set_from_it() {
        let source = "# Uno\n\nDos `tres` [cuatro](https://example.test/a)\n\n- cinco\n\n> seis\n\n```rust\nsiete\n```\n\n```mermaid\nflowchart LR\n  A[Ocho] --> B[Nueve]\n```";
        let build = |size: f32| {
            Markdown::new(source, palette())
                .width(480.0)
                .font_size(size)
        };
        let normal = commands(build(14.0).render().element, 480.0);
        let large = commands(build(21.0).render().element, 480.0);
        // Headings, prose, inline code, links, markers, quotes, code blocks and a diagram's card.
        for text in [
            "Uno", "Dos", "tres", "cuatro", "•", "cinco", "seis", "RUST", "siete", "DIAGRAM",
        ] {
            assert_eq!(
                painted_size(&large, text),
                painted_size(&normal, text) * 1.5,
                "{text}"
            );
        }
        assert_eq!(painted_size(&normal, "Dos"), BODY_SIZE);
        // The drawing keeps its own size.
        assert_eq!(painted_size(&large, "Ocho"), painted_size(&normal, "Ocho"));

        let mut tree = UiTree::new(Element::column(vec![
            build(21.0).render().element,
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(480.0, 2_000.0));
        let blocks = frame.text_layouts();
        let run = |source: &str| {
            blocks
                .iter()
                .find(|block| block.source.starts_with(source))
                .unwrap()
                .runs[0]
                .rect
        };
        assert_eq!(run("Dos").height, BODY_LINE_HEIGHT * 1.5);
        assert_eq!(run("Uno").height, 22.0 * 1.5 * 1.4);
        // The marker still sits on the line of its item, and the space above a block grows too.
        assert_eq!(run("•").y, run("cinco").y);
        assert!(run("Dos").y - (run("Uno").y + run("Uno").height) >= 8.0 * 1.5);
        // The link answers the pointer where it is drawn at the new size.
        let link = frame
            .regions
            .iter()
            .find(|region| region.id.starts_with("markdown_link:"))
            .unwrap();
        let (id, rect) = (link.id.clone(), link.rect);
        assert!(rect.height >= 21.0);
        let center = crate::Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(crate::UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(crate::UiEvent::PointerUp(center)),
            vec![crate::UiAction::Clicked { id }]
        );

        // The literal mode and the note under a cut preview follow it as well.
        let plain = Markdown::new("ver https://example.test/a", palette())
            .font_size(20.0)
            .plain_with_autolinks(28.0)
            .render();
        let plain = commands(plain.element, 480.0);
        assert_eq!(
            (
                painted_size(&plain, "ver"),
                painted_size(&plain, "https://example.test/a")
            ),
            (20.0, 20.0)
        );
        let cut = build(21.0)
            .limits(MarkdownLimits {
                source_chars: 20_000,
                blocks: 1,
            })
            .render()
            .element;
        assert_eq!(
            painted_size(
                &commands(cut, 480.0),
                MarkdownLabels::default().truncated_note
            ),
            11.0 * 1.5
        );
    }

    #[test]
    fn a_larger_list_marker_keeps_to_its_column() {
        let element = Markdown::new("10. diez", palette())
            .width(480.0)
            .font_size(28.0)
            .render()
            .element;
        let mut tree = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]));
        tree.set_text_measurer(|text, size, _| text.chars().count() as f32 * size * 0.5);
        let blocks = tree.layout(Size::new(480.0, 400.0)).text_layouts();
        let runs = |source: &str| {
            blocks
                .iter()
                .find(|block| block.source == source)
                .unwrap()
                .runs
                .clone()
        };
        // At 28 px "10." is 42 wide: more than the 22 px column of a 14 px body.
        let (marker, item) = (runs("10."), runs("diez")[0].rect);
        assert_eq!(marker.len(), 1, "the marker wrapped: {marker:?}");
        assert!(
            marker[0].rect.x + marker[0].rect.width <= item.x,
            "{marker:?} {item:?}"
        );
    }

    #[test]
    fn a_bad_font_size_is_ignored_and_the_rest_is_kept_between_8_and_96() {
        let used = |markdown: Markdown<'_>| markdown.presentation.font_size();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -3.0] {
            assert_eq!(
                used(Markdown::new("", palette()).font_size(bad)),
                BODY_SIZE,
                "{bad}"
            );
            assert_eq!(
                used(Markdown::new("", palette()).font_size(18.0).font_size(bad)),
                18.0,
                "{bad}"
            );
            let element = Markdown::new("# a\n\n- b", palette())
                .font_size(bad)
                .render()
                .element;
            for command in commands(element, 320.0) {
                if let DrawCommand::Text {
                    position,
                    font_size,
                    ..
                } = command
                {
                    assert!(
                        position.x.is_finite() && position.y.is_finite() && font_size.is_finite()
                    );
                }
            }
        }
        assert_eq!(used(Markdown::new("", palette()).font_size(1.0)), 8.0);
        assert_eq!(used(Markdown::new("", palette()).font_size(1.0e9)), 96.0);
        assert_eq!(used(Markdown::new("", palette()).font_size(17.5)), 17.5);
    }

    #[test]
    fn another_font_size_is_another_cache_entry_and_no_stream_reuses_a_block_across_sizes() {
        MARKDOWN_CACHES.with(|caches| *caches.borrow_mut() = MarkdownCaches::new());
        let complete = |size: f32| {
            Markdown::new("uno", palette())
                .width(320.0)
                .font_size(size)
                .render()
                .element
        };
        assert_eq!(painted_size(&commands(complete(14.0), 320.0), "uno"), 14.0);
        assert_eq!(painted_size(&commands(complete(20.0), 320.0), "uno"), 20.0);
        assert_eq!(painted_size(&commands(complete(14.0), 320.0), "uno"), 14.0);
        MARKDOWN_CACHES.with(|caches| assert_eq!(caches.borrow().completed.entries.len(), 2));

        let streaming = |source: &str, size: f32| {
            Markdown::new(source, palette())
                .width(640.0)
                .streaming(true)
                .font_size(size)
                .render()
                .element
        };
        let _ = streaming("Uno\n\nDos", 14.0);
        INLINE_PARSE_COUNT.with(|count| count.set(0));
        let grown = streaming("Uno\n\nDos tres", 20.0);
        assert_eq!(
            INLINE_PARSE_COUNT.with(|count| count.get()),
            2,
            "a block of another size is built again"
        );
        assert_eq!(painted_size(&commands(grown, 640.0), "Uno"), 20.0);
    }

    #[test]
    fn a_frame_guard_keeps_cached_rows_until_the_scope_or_width_changes() {
        MARKDOWN_CACHES.with(|caches| *caches.borrow_mut() = MarkdownCaches::new());
        let entries = || MARKDOWN_CACHES.with(|caches| caches.borrow().completed.entries.len());
        {
            let _frame = Markdown::begin_frame(Some("a"), 320.0);
            let _ = render_markdown_at_width("uno", 320.0);
        }
        assert_eq!(entries(), 1);
        drop(Markdown::begin_frame(Some("a"), 320.0));
        assert_eq!(entries(), 1);
        drop(Markdown::begin_frame(Some("b"), 320.0));
        assert_eq!(entries(), 0);
        let _ = render_markdown_at_width("uno", 320.0);
        drop(Markdown::begin_frame(Some("b"), 480.0));
        assert_eq!(entries(), 0);
    }
}
