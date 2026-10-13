//! Engine cache for [`crate::Markdown`]: bounded presentation trees retained
//! across frames, like the paragraph layouts of `formatted_text_cache.rs`.
//! The component stores nothing; a tree is reused only while its source,
//! width and every color, label and limit are equal.

use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::size_of,
    rc::Rc,
};

use crate::markdown::{MarkdownPresentation, MarkdownPreview};
use crate::markdown_parser::Block;
use crate::{Element, ElementKind, TextSpan};

pub(crate) const MAX_CACHED_MESSAGES: usize = 64;
pub(crate) const MAX_CACHED_BYTES: usize = 4 * 1_024 * 1_024;
// A preview at the block limit must fit, and an element is about a kilobyte:
// at half this, one more field on `Element` left the largest previews uncached.
pub(crate) const MAX_STREAMING_CACHE_BYTES: usize = 1_024 * 1_024;
const MAX_PENDING_CACHE_BYTES: usize = 512 * 1_024;

pub(crate) struct MarkdownCacheEntry {
    pub(crate) source: String,
    pub(crate) truncated: bool,
    pub(crate) limited: bool,
    pub(crate) presentation: MarkdownPresentation,
    pub(crate) element: Element,
    pub(crate) heap_bytes: usize,
    pub(crate) blocks: Vec<Block>,
    pub(crate) incomplete_block: Option<usize>,
}

impl MarkdownCacheEntry {
    pub(crate) fn new(
        source: &str,
        truncated: bool,
        presentation: MarkdownPresentation,
        preview: &MarkdownPreview,
        blocks: &[Block],
        incomplete_block: Option<usize>,
    ) -> Option<Self> {
        // Cache bounded presentation trees. Their only actions contain the
        // immutable destination and a source hash; they do not refer to any
        // conversation, tool call, or mutable host resource.
        passive_heap_bytes(&preview.element)?;
        let source = source.to_owned();
        let element = preview.element.clone();
        // Account the stored clone's actual capacities, including child vector
        // storage, before deciding whether it fits the retained byte budget.
        let element_bytes = passive_heap_bytes(&element)?;
        let blocks = blocks.to_vec();
        let heap_bytes = source
            .capacity()
            .saturating_add(element_bytes)
            .saturating_add(blocks.capacity() * size_of::<Block>())
            .saturating_add(blocks.iter().map(Block::heap_bytes).sum::<usize>());
        Some(Self {
            source,
            truncated,
            limited: preview.limited,
            presentation,
            element,
            heap_bytes,
            blocks,
            incomplete_block,
        })
    }
}

pub(crate) struct MarkdownCache {
    // Oldest entry first. A small fixed-capacity list avoids hashing or retaining
    // an unbounded source string as a key, and source equality cannot collide.
    pub(crate) entries: Vec<MarkdownCacheEntry>,
    pub(crate) retained_bytes: usize,
    pub(crate) max_entries: usize,
    pub(crate) max_bytes: usize,
}

impl MarkdownCache {
    pub(crate) fn new(max_entries: usize, max_bytes: usize) -> Self {
        let entries = Vec::with_capacity(max_entries);
        let retained_bytes = entries.capacity() * size_of::<MarkdownCacheEntry>();
        Self {
            entries,
            retained_bytes,
            max_entries,
            max_bytes,
        }
    }

    pub(crate) fn get(
        &mut self,
        source: &str,
        truncated: bool,
        presentation: MarkdownPresentation,
    ) -> Option<MarkdownPreview> {
        let index = self.entries.iter().position(|entry| {
            entry.source == source
                && entry.truncated == truncated
                && entry.presentation == presentation
        })?;
        let entry = self.entries.remove(index);
        let preview = MarkdownPreview {
            element: entry.element.clone(),
            limited: entry.limited,
        };
        self.entries.push(entry);
        Some(preview)
    }

    pub(crate) fn can_insert_without_eviction(&self, entry: &MarkdownCacheEntry) -> bool {
        self.entries.len() < self.max_entries
            && self.retained_bytes.saturating_add(entry.heap_bytes) <= self.max_bytes
    }

    pub(crate) fn insert_entry(&mut self, entry: MarkdownCacheEntry) {
        let heap_bytes = entry.heap_bytes;
        let storage_bytes = self.entries.capacity() * size_of::<MarkdownCacheEntry>();
        if storage_bytes.saturating_add(heap_bytes) > self.max_bytes {
            return;
        }
        while self.entries.len() >= self.max_entries
            || self.retained_bytes.saturating_add(heap_bytes) > self.max_bytes
        {
            let entry = self.entries.remove(0);
            self.retained_bytes -= entry.heap_bytes;
        }
        self.retained_bytes += heap_bytes;
        self.entries.push(entry);
    }

    pub(crate) fn take_latest(&mut self) -> Option<MarkdownCacheEntry> {
        let entry = self.entries.pop()?;
        self.retained_bytes -= entry.heap_bytes;
        Some(entry)
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.retained_bytes = self.entries.capacity() * size_of::<MarkdownCacheEntry>();
    }
}

pub(crate) struct MarkdownCaches {
    pub(crate) completed: MarkdownCache,
    pub(crate) streaming: MarkdownCache,
    pub(crate) pending_completed: MarkdownCache,
    pub(crate) frame_depth: usize,
    pub(crate) frame_context: Option<(u64, u32)>,
}

impl MarkdownCaches {
    pub(crate) fn new() -> Self {
        Self {
            completed: MarkdownCache::new(MAX_CACHED_MESSAGES, MAX_CACHED_BYTES),
            // A changing response replaces its previous version instead of
            // evicting completed messages from their LRU cache.
            streaming: MarkdownCache::new(2, MAX_STREAMING_CACHE_BYTES),
            // Defer one admission until the transcript finishes its forward
            // scan. Older misses cannot evict cached rows before they are read.
            pending_completed: MarkdownCache::new(1, MAX_PENDING_CACHE_BYTES),
            frame_depth: 0,
            frame_context: None,
        }
    }

    pub(crate) fn get(
        &mut self,
        source: &str,
        truncated: bool,
        presentation: MarkdownPresentation,
        streaming: bool,
    ) -> Option<MarkdownPreview> {
        if streaming {
            self.streaming.get(source, truncated, presentation)
        } else {
            self.pending_completed
                .get(source, truncated, presentation)
                .or_else(|| self.completed.get(source, truncated, presentation))
        }
    }

    pub(crate) fn insert(&mut self, entry: MarkdownCacheEntry, streaming: bool) {
        let cache = if streaming {
            &mut self.streaming
        } else if self.frame_depth > 0 && !self.completed.can_insert_without_eviction(&entry) {
            &mut self.pending_completed
        } else {
            &mut self.completed
        };
        cache.insert_entry(entry);
    }

    pub(crate) fn finish_frame(&mut self) {
        self.frame_depth = self.frame_depth.saturating_sub(1);
        if self.frame_depth == 0 {
            if let Some(entry) = self.pending_completed.take_latest() {
                self.completed.insert_entry(entry);
            }
        }
    }
}

thread_local! {
    // Session presentation happens on the UI thread. No shared mutex is held
    // during frame construction; another UI thread has its own bounded cache.
    pub(crate) static MARKDOWN_CACHES: RefCell<MarkdownCaches> = RefCell::new(MarkdownCaches::new());
}

/// Defers completed-cache eviction during a transcript scan; made by
/// [`crate::Markdown::begin_frame`] and released when dropped. Without this guard,
/// a conversation larger than the cache could evict every upcoming hit while
/// rebuilding its older rows. At most one bounded candidate waits for admission.
pub struct MarkdownFrame {
    pub(crate) _thread: PhantomData<Rc<()>>,
}

impl Drop for MarkdownFrame {
    fn drop(&mut self) {
        MARKDOWN_CACHES.with(|caches| caches.borrow_mut().finish_frame());
    }
}

pub(crate) fn begin_frame(scope: Option<&str>, width_bits: u32) -> MarkdownFrame {
    let mut scope_hash = DefaultHasher::new();
    scope.hash(&mut scope_hash);
    // This hash only decides when to clear an old frame's cache. Each actual
    // hit still checks the bounded source and full presentation for equality.
    let context = (scope_hash.finish(), width_bits);
    MARKDOWN_CACHES.with(|caches| {
        let mut caches = caches.borrow_mut();
        if caches.frame_depth == 0 && caches.frame_context != Some(context) {
            caches.completed.clear();
            caches.streaming.clear();
            caches.pending_completed.clear();
            caches.frame_context = Some(context);
        }
        caches.frame_depth = caches.frame_depth.saturating_add(1);
    });
    MarkdownFrame {
        _thread: PhantomData,
    }
}

/// Heap retained by the passive subset generated below. Root storage is part
/// of MarkdownCacheEntry; child capacity and owned string capacity are counted.
/// Unknown or interactive kinds are deliberately ineligible for caching.
pub(crate) fn passive_heap_bytes(element: &Element) -> Option<usize> {
    if element.hover_peer.is_some()
        || element.hover_surface.is_some()
        || element.action_alias.is_some()
        || element.menu_navigation
    {
        return None;
    }
    match &element.kind {
        ElementKind::Column(children) | ElementKind::Row(children) => {
            let mut bytes = children.capacity().saturating_mul(size_of::<Element>());
            for child in children {
                bytes = bytes.saturating_add(passive_heap_bytes(child)?);
            }
            Some(bytes)
        }
        ElementKind::Panel(child) => {
            Some(size_of::<Element>().saturating_add(passive_heap_bytes(child)?))
        }
        ElementKind::Text(text) => Some(text.capacity()),
        ElementKind::FormattedText(spans) => Some(spans.iter().fold(
            spans.capacity().saturating_mul(size_of::<TextSpan>()),
            |bytes, span| {
                bytes
                    .saturating_add(span.text.capacity())
                    .saturating_add(span.action.as_ref().map_or(0, String::capacity))
            },
        )),
        ElementKind::Spacer => Some(0),
        // A diagram and the viewport it pans in hold no host state either.
        ElementKind::Flowchart(flowchart) => Some(flowchart.heap_bytes()),
        ElementKind::SequenceDiagram(diagram) => Some(diagram.heap_bytes()),
        ElementKind::HorizontalScroll { id, child, .. } => Some(
            size_of::<Element>()
                .saturating_add(id.capacity())
                .saturating_add(passive_heap_bytes(child)?),
        ),
        _ => None,
    }
}
