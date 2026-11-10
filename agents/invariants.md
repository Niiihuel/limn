# Invariants

What always holds in the core, what holds it, and what is still open. A
change that breaks one is wrong even when every test passes; a change that
closes an open one says so here.

Each invariant names the rule of [`AGENTS.md`](../AGENTS.md#3-rules) it backs.
Status is one of:

| Status | Means |
| --- | --- |
| **Held** | A test or an assertion fails when it is broken. |
| **Reported** | The core says when it is broken, and does not refuse. |
| **Partly** | Held for some cases; the page says which are not. |
| **Open** | Nothing checks it. A reviewer does. |

## The table

| # | Invariant | Rule | Held by | Status |
| --- | --- | --- | --- | --- |
| 1 | The window is a function of the host's state. | R1 | `tests/controls.rs`: a control reports one action for a click, Enter and Space. That it changes nothing it shows by itself: review. | Partly |
| 2 | One ID, one mounted control. | R2 | `Frame::duplicate_ids`; `a_frame_lists_the_ids_two_controls_took_but_not_a_link_linked_twice`. | Reported, for controls that take focus; see below |
| 3 | A key the tree derives cannot meet a host's ID. | R2 | Nothing. | Open |
| 4 | Measuring changes no state. | R4 | `states_mut!` asserts it in debug builds, under the whole suite. | Held |
| 5 | Kept state is bounded by what is on screen. | R3, R5 | `StateStore::sweep`; `state_out_of_sight_is_kept_only_for_the_ids_laid_out_most_recently`, which asserts `inputs` and `scroll_offsets`; `scrolls_asked_of_ids_never_laid_out_are_kept_only_for_the_most_recent`. | Partly |
| 6 | An event is answered from the last frame; a request waits for the next layout. | R6 | `handle_event_at` lays nothing out; `a_focus_asked_for_before_its_control_is_laid_out_lands_in_the_layout_that_adds_it`. | Held |
| 7 | The host lays out once a frame. | R6 | The loop of `docs/backend.md` and of `limn-egui`'s examples. | Held for hosts; see below |
| 8 | After a layout, hover is what is under the pointer. | | `stationary_pointer_retargets_hover_after_tree_rebuild`. | Held |
| 9 | A frame is data, and a backend knows no widget. | R7, R8 | `limn` depends on no window or GPU crate. | Partly |
| 10 | Tokens have a scope. | R11 | `with_tokens_holds_on_its_own_thread_until_it_returns_or_unwinds`. | Held |
| 11 | Whatever moves is driven by the layout's clock, and can stand still. | R12 | `layout_at(viewport, now)`; `set_reduced_motion`. | Held |
| 12 | A broken contract is loud in a debug build and harmless in a release one. | | See below. | Partly |
| 13 | The engine holds no design. | R22, R23 | The compiler: `limn-components` sees only what `limn-core` makes public; `seams_do_not_grow`. | Partly |

## What breaking each one looks like

**1. A function of the host's state.** A component that keeps its own value
shows something the host does not know: the screen and the state disagree
after a rebuild, and a test that lays the view out cannot reach the bug.

**2. One ID, one control.** Hovering one control lights another; Tab stops
once where there are two; a transition started by one runs on both. A screen's
test asserts `frame.duplicate_ids().is_empty()`. The core lists the IDs
and goes on, because an application in front of a user is better off with a
shared hover than with a panic. The walk that finds them (`collect_focus_claims`)
sees controls that take focus in the tree as given. It does not see the rows
of a virtual list, the content of a popover opened by hover, a `pointer_only`
clickable, or two scroll areas under one ID.

**3. Derived keys.** The tree makes keys from IDs as text: `extend/{id}`,
`slide/{id}`, and a popover's surface answers as `popover_surface:{id}`. A
host ID that starts the same way would be read as the tree's own. Closing it
means a key type the host cannot write, not a longer prefix.

**4. Measuring changes no state.** A spring starts because a parent asked for
a size and then showed something else; a number rolls twice as fast inside a
parent that measures twice. The assertion's message is `a measure changed the
state store`: move the change to the placing, and have the measure predict it
from what the last layout left.

**5. Bounded state.** Memory grows with the life of the window: one entry per
message ever shown. State in the `StateStore` cannot do this. A map of its own
can, which is why R3 forbids a new one. Still on maps of their own: `inputs`,
`scroll_offsets`, `measured_lists` and `horizontal_scrolls` (trimmed to the
dormant 64 by `keep_recent`), the popover family, the code views
(`trim_code_view_cache`), and the smaller ones [State](architecture.md#state)
lists. A `scroll_to_end` or `scroll_to_index` waits for its scroll area
across layouts, since hosts ask before the view that holds the list is shown
(a search's results behind its loading state, a conversation behind a pane
grown over it); `ScrollRequests` keeps those for the 64 IDs asked of most
recently. Partly, because a test asserts the bound only for `inputs`,
`scroll_offsets` and the scroll requests: the other maps are read in review.

**6. Last frame, next layout.** A `focus(id)` that returns `false` because
the control is not on screen yet, so the host lays out, focuses and lays out
again. Anything of that shape is a request to queue: see `pending_focus`,
`pending_reveal`, `pending_center`.

**7. One layout a frame.** For a host this is a rule about its loop. Inside
`layout_at` the pass may run again: when a focus or a reveal moved a scroll
offset, on a coach mark's new step, when a popover opens with an initial
focus, when a reach that pushes its neighbours jumped, and once to paint a
hover that the layout itself changed. Each repeat costs a whole pass. A new
reason to repeat needs its cost measured. One example does not follow the
rule: `crates/limn-gallery/examples/captured_paste.rs` lays out after each
event.

**8. Hover after a layout.** A list scrolls under a still pointer and the row
that left stays lit, or its tooltip opens over another row.

**9. A frame is data.** `limn` builds and runs its tests with no display.
The part that does not hold: `DrawCommand::Spinner`, `Record`, `ProgressRing`
and `EffortSlider` say which widget to draw, not what to draw, and
`limn-egui` knows how each looks. `Icon` names an icon of the library, and
`Surface` a host's pixels by ID: those two are references, and stay.

**10. Scoped tokens.** A test that changes the radius reshapes the tests
running beside it. Inside the crate, a test wraps its tree in `with_tokens`.
A test that must call `set_tokens` goes in a binary of its own under
`tests/`, as `tokens.rs` and `shortcut_style.rs` are.

**11. The layout's clock.** An animation read from `Instant::now()` cannot be
tested at a chosen instant and does not stop under reduced motion. Take
`self.now` in the layout; name animated state in `UiTree::states_animate`, or
beside its call in `other_animation_repaint_after_at`, or the host is never
asked for the next frame and the motion stops halfway.

**12. Loud in debug, harmless in release.** What is in place:

| Contract | Debug | Release |
| --- | --- | --- |
| No state change while measuring | Panics. | Not checked. |
| A caller's number is finite and in range | | Ignored or clamped in the builder (`tests/conventions.rs`). |
| A drawing closes the clips it opens | | Balanced by the tree (`tests/drawing.rs`). |
| A viewport is not negative | | Read as zero. |

Not in place: an ID taken twice does not panic in debug (see 2), and a
derived key that meets a host's ID is not seen at all (see 3).

**13. No design in the engine.** A host that wants its own look finds it
cannot change one: a tooltip, a scrollbar, a button's fill, because the tree
paints it itself. Held for every component in `limn-components`: none can
reach inside the engine, and what was opened for them is counted. Not held
for what the engine still carries; the [plan](plan.md) lists it. A new
component that needs a seam is the sign: the engine lacks a general piece.

## Adding an invariant

Write it here with its check in the same pull request that adds the check.
An invariant with no check is entered as **Open**, with what would close it.
