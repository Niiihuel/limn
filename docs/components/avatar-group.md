# AvatarGroup

Several people as avatars that overlap in a row, with a count for those that
do not fit. Use it where who is here matters more than each name: the members
of a project, the people who answered.

## Import

```rust
use limn::{Avatar, AvatarGroup, Component};
```

## Usage

A group takes its avatars in the order they are shown, and the palette. The
host owns the people; the group reports nothing, so there is no action to
handle.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let avatars = state.members.iter().map(|member| Avatar::new(&member.name, palette)).collect();
    AvatarGroup::new(avatars, palette)
        .limit(4)
        .id("members")
        .accessibility_label(format!("{} members", state.members.len()))
        .render()
}
```

The first avatar lies on top and each one after it is partly under the one
before, so the row reads from its start. Each has a ring 2 px wide in the
palette's `surface`, over a disc of the same color, which separates it from
the one it lies over and hides it. A group of
one is that avatar with its ring; a group of none draws nothing and is zero
wide.

## Limit

```rust
AvatarGroup::new(avatars, palette).limit(3).render()
```

`limit(n)` shows the first `n` avatars and then a circle of the same size that
counts the rest: `+2`. More than 99 reads `99+`. There is no circle when all
fit, and a limit of zero is raised to one. Without `limit` all are shown.

The circle takes a place of its own: four avatars with `limit(3)` are three
and `+1`.

## Size

```rust
AvatarGroup::new(avatars, palette).size(24.0).render()
```

`size` is the diameter of every avatar and of the count, 32 unless set. It
replaces the size each `Avatar` was given, and is clamped as `Avatar::size`
clamps it, from 16 to 256. A size that is not finite is ignored.

## Overlap

```rust
AvatarGroup::new(avatars, palette).overlap(0.4).render()
```

`overlap` is the share of its diameter that one avatar covers of the next: a
quarter unless set, clamped to 0 (they touch) to 0.6. A value that is not
finite is ignored. A group of `n` circles of diameter `d` is
`d + (n - 1) * d * (1 - overlap)` wide and `d` high.

## Ring

```rust
AvatarGroup::new(avatars, palette).ring(palette.raised).render()
```

The ring should be the color of the ground the group stands on. `ring` sets
it for a group that is not on the palette's `surface`.

## Disabled

There is nothing to disable: the group is not a control.

## Keyboard

None. The group adds no tab stop.

## Actions

None. The group has no hit region.

## Accessibility

A group tells the platform nothing unless it is given both an ID and a name;
then it is one image with that name. The avatars inside are never nodes of
their own.

```rust
AvatarGroup::new(avatars, palette)
    .id("members")
    .accessibility_label("Ada, Grace and 3 others")
    .render()
```

The words are the host's: the group does not make a name out of its avatars,
because it often stands beside text that already says who they are. An empty
group has no node.

## Limits

- The count is not a control: it opens nothing. Wrap the group in
  `Element::clickable` to make the whole of it one.
- The ring is drawn over the edge of each avatar, so 2 px of its rim are
  covered.
- The group does not wrap or shrink: it keeps its width in a narrower place.
