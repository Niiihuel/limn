//! How the tree places a scroll area and a uniform list: the offset, the
//! rows in view and the scrollbar.

use super::*;

impl LayoutBuilder<'_> {
    pub(super) fn place_scroll(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, scrollbar_visibility: crate::ScrollbarVisibility, pinned: &mut Vec<PinnedChild>) {
        let ElementKind::Scroll {
            id,
            child,
            appearance,
            motion,
        } = &element.kind else { unreachable!() };
        self.add_region(id, rect, HitKind::Scroll, clip);
        let full_width_height = self
            .measure(child, Size::new(content.width, f32::INFINITY))
            .height;
        let child_width = appearance
            .content_width(content.width, overflow(full_width_height, content.height) > 0.0);
        let child_height = if child_width < content.width {
            self.measure(child, Size::new(child_width, f32::INFINITY))
                .height
        } else {
            full_width_height
        };
        let max_offset = overflow(child_height, content.height);
        self.frame.scroll_extents.insert(id.clone(), max_offset);
        let snap = self.scroll_to_end.remove(id).then_some(max_offset);
        let current_offset = self.scroll_position(id, max_offset, *motion, snap);
        let scrollbar =
            ScrollbarGeometry::new(content, max_offset, current_offset, *appearance);
        let child_rect = Rect::new(
            content.x,
            content.y - current_offset,
            child_width,
            child_height.max(content.height),
        );
        let new_clip = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content));
        if let Some(new_clip) = new_clip {
            self.frame.commands.push(DrawCommand::PushClip(new_clip));
            self.round_clip(content, &style);
            self.scroll_focus_context.push(ScrollFocusContext {
                id: id.clone(),
                viewport: new_clip,
                offset: current_offset,
            });
            let outer = std::mem::take(&mut self.pinned);
            self.place(child, child_rect, Some(new_clip));
            *pinned = std::mem::replace(&mut self.pinned, outer);
            self.scroll_focus_context.pop();
            self.frame.commands.push(DrawCommand::PopClip);
        }
        if let Some(scrollbar) = scrollbar {
            self.paint_scrollbar(
                id,
                scrollbar,
                content,
                clip,
                style.foreground,
                *appearance,
                scrollbar_visibility,
            );
        }
    }

    pub(super) fn place_uniform_list(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, scrollbar_visibility: crate::ScrollbarVisibility) {
        let ElementKind::UniformList(list) = &element.kind else { unreachable!() };
        let id = list.id.as_str();
        self.add_region(id, rect, HitKind::Scroll, clip);
        let rows_height = (list.row_count as f32 * list.row_height).min(f32::MAX / 4.0);
        // What a row opened under itself, at the height it takes in
        // the width the rows have: the narrower one while a bar shows.
        let open = list
            .expansion
            .as_ref()
            .filter(|(index, _)| *index < list.row_count);
        let mut extra = 0.0;
        if let Some((_, opened)) = open {
            let narrow = list.appearance.content_width(content.width, true);
            extra = self.measure(opened, Size::new(narrow, f32::INFINITY)).height;
            if rows_height + extra <= content.height && narrow != content.width {
                extra = self.measure(opened, Size::new(content.width, f32::INFINITY)).height;
            }
        }
        // The rows from `split` on stand under what was opened.
        let split = open.map_or(usize::MAX, |(index, _)| index + 1);
        let parted = open.map_or(f32::INFINITY, |(index, _)| (index + 1) as f32 * list.row_height);
        let row_top =
            |index: usize| index as f32 * list.row_height + if index >= split { extra } else { 0.0 };
        let content_height = rows_height + extra;
        let max_offset = (content_height - content.height).max(0.0);
        self.frame
            .scroll_extents
            .insert(list.id.clone(), max_offset);
        let mut snap = self.scroll_to_end.remove(id).then_some(max_offset);
        if let Some(index) = self.scroll_to_index.remove(id) {
            let mut offset =
                snap.unwrap_or_else(|| self.scroll_offsets.get(id).copied().unwrap_or(0.0));
            if list.row_count > 0 {
                let index = index.min(list.row_count - 1);
                let top = row_top(index);
                let bottom = top + list.row_height;
                if top < offset {
                    offset = top;
                } else if bottom > offset + content.height {
                    offset = bottom - content.height;
                }
            }
            snap = Some(offset);
        }
        let current_offset = self.scroll_position(id, max_offset, list.motion, snap);
        let scrollbar =
            ScrollbarGeometry::new(content, max_offset, current_offset, list.appearance);
        let row_width = list
            .appearance
            .content_width(content.width, scrollbar.is_some());
        // A height of the content as a height of the rows alone: what
        // was opened stands between two of them and holds no row.
        let among_rows = |y: f32| if y <= parted { y } else { (y - extra).max(parted) };
        let first_visible = (among_rows(current_offset) / list.row_height).floor() as usize;
        let end_visible = (among_rows(current_offset + content.height) / list.row_height)
            .ceil() as usize;
        let mut first_visible = first_visible.min(list.row_count);
        let end_visible = end_visible.min(list.row_count);
        // What a row opened is built with its row and only with it, so
        // the keyboard walks from one into the other. Its row is built
        // while any of it shows, even once the row itself scrolled off.
        let opened_shows = extra > 0.0
            && parted + extra > current_offset
            && parted < current_offset + content.height;
        if opened_shows && first_visible >= split {
            first_visible = split - 1;
        }
        let visible = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content));
        let mut rows = Vec::new();
        if let Some(visible) = visible {
            self.frame.commands.push(DrawCommand::PushClip(visible));
            for index in first_visible..end_visible {
                let row = (list.build_row)(index);
                let previous_scope = self.enter_measurement_scope();
                let mut focusables = Vec::new();
                let mut seen = crate::FastSet::default();
                self.collect_row_focus(&row, &mut focusables, &mut seen);
                self.place(
                    &row,
                    Rect::new(
                        content.x,
                        content.y + row_top(index) - current_offset,
                        row_width,
                        list.row_height,
                    ),
                    Some(visible),
                );
                if let Some((_, opened)) = open.filter(|(open, _)| *open == index) {
                    self.collect_row_focus(opened, &mut focusables, &mut seen);
                    self.place(
                        opened,
                        Rect::new(
                            content.x,
                            content.y + parted - current_offset,
                            row_width,
                            extra,
                        ),
                        Some(visible),
                    );
                }
                rows.push((index, focusables));
                self.measurement_scope = previous_scope;
            }
            self.frame.commands.push(DrawCommand::PopClip);
        }
        self.frame.virtual_lists.insert(
            list.id.clone(),
            VirtualListFocus {
                first_visible,
                end_visible,
                row_count: list.row_count,
                rows,
            },
        );
        if let Some(scrollbar) = scrollbar {
            self.paint_scrollbar(
                id,
                scrollbar,
                content,
                clip,
                style.foreground,
                list.appearance,
                scrollbar_visibility,
            );
        }
    }
}
