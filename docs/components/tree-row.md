# TreeRow

One row of a file, project or task tree: the guides that show its depth, an
icon, a title, and what a folder shows at its end. The host owns the tree;
`TreeRow` draws one visible row of it.

## Import

```rust
use limn::{guides_for_depths, Component, TreeGuides, TreeRow};
```

## Usage

A row takes the control that wraps it, its icon, its title, its guides and
the palette. The control is a closure that receives the row's content and
returns the element that reports the click, so the action is the host's.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let depths: Vec<usize> = state.visible.iter().map(|item| item.depth).collect();
    let rows = state
        .visible
        .iter()
        .zip(guides_for_depths(&depths))
        .map(|(item, guides)| {
            let id = item.id.clone();
            TreeRow::new(
                move |content| Element::clickable(id, content),
                Element::icon(item.icon),
                Element::truncated_text(&item.name),
                guides,
                palette,
            )
            .selected(state.selected == item.id)
            .render()
        })
        .collect();
    Element::column(rows)
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        state.select(id);
    }
}
```

The whole row is one hit target. It hovers to the palette's `hover`, and
`selected(true)` fills it with the palette's `selected`.

## Guides

`guides_for_depths` takes the depth of every visible row, in the order they
are drawn, and returns a `TreeGuides` for each: which lanes continue past
the row and whether a sibling follows it. Call it again when a folder opens
or closes, with only the rows that are shown. `TreeGuides::root()` is a row
at depth zero, which has no guides.

Each level indents the row by 12. `max_guide_depth` caps how many lanes a
row draws, 6 by default; a deeper row draws its nearest lanes and indents no
further. `guide_color` replaces the palette's `border` as their color.

## Folders

```rust
TreeRow::new(action, icon, title, guides, palette)
    .count(12)
    .disclosure(state.is_open(id))
    .render()
```

`count` shows how many items the row holds in small muted figures at its
end. `disclosure(open)` adds a muted chevron after it, pointing down when
open and right when folded. The host folds the folder on its click and
leaves the rows under it out.

## Trailing element

```rust
TreeRow::new(action, icon, title, guides, palette).trailing(more_button).render()
```

`trailing` puts an element of the host's at the end of the row, after the
count and the chevron. A control there keeps its own hit target.

## Size

`width` fixes the row's width; without it the row takes the width it is
given. `height` sets its height, which is `ControlSize::Compact.height()` by
default; a height under 18 is ignored.

## Accessibility

The row tells the platform it is an item of a tree, named by its title,
whether it is selected and, with `disclosure`, whether it is expanded. A row
without `disclosure` is a leaf.
