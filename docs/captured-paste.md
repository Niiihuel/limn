# Captured paste targets

**Unreleased API contract.** Deliver a delayed text reply to the editable state
that requested it. A control ID alone cannot distinguish a field which lost
focus, changed its selection, and later returned to the same visible state.

`PasteTarget` is a backend-neutral capture, not clipboard access. It carries no
text, buffer/controller, snapshot, window, SDK token or request resource. The
host still owns the request, text limits, cancellation and context lifetime.
This guide does not claim system clipboard integration or native clipboard QA.

## Capture and deliver

Lay out the current tree, resolve focus, and capture the target before allowing
the delayed text read to start. A deferred service may already have reserved the
request without reading. Keep the returned value with that request and deliver
only that capture when its reply arrives:

```rust
use limn::{Element, Size, UiAction, UiEvent, UiTree};

fn main() {
    let mut value = String::from("Hello");
    let mut tree = UiTree::new(
        Element::text_input("message", value.clone(), "Message").size(None, Some(32.0)),
    );
    tree.set_window_focused(true);
    tree.layout(Size::new(420.0, 100.0));
    assert!(tree.focus("message"));

    let target = tree.capture_paste_target().expect("editable target");
    let duplicate = target.clone();
    // Synthetic delayed result: no clipboard or I/O is used in this example.
    let actions = tree.handle_event(UiEvent::PasteCaptured {
        target,
        text: "!".into(),
    });
    for action in actions {
        if let UiAction::InputChanged { id, value: next } = action {
            assert_eq!(id, "message");
            value = next;
        }
    }
    assert_eq!(value, "Hello!");

    assert!(tree.handle_event(UiEvent::PasteCaptured {
        target: duplicate,
        text: "must not repeat".into(),
    }).is_empty());
    assert_eq!(tree.input_value("message"), Some("Hello!"));

    let stale = tree.capture_paste_target().expect("fresh target");
    // The application changed its document/account/session association.
    tree.invalidate_paste_targets();
    assert!(tree.handle_event(UiEvent::PasteCaptured {
        target: stale,
        text: "late reply".into(),
    }).is_empty());
    assert_eq!(tree.input_value("message"), Some("Hello!"));
}
```

`capture_paste_target(&mut self)` returns None without a fresh layout, native
window focus, or an enabled, focused editable control. It also returns None
when the code buffer is busy, an active IME composition owns the input, or the
editor's current mode does not accept this paste. It never waits for a buffer
lock. Do not replace None with an ID-based or generic paste.

## One delivery per capture

Cloning a target retains the same capture identity. Once a delivery is accepted,
all its clones are consumed. Two fresh captures compare unequal even if they
refer to the same state. Debug output is redacted, and there is no serialization
contract or public constructor.

Acceptance consumes the capture even if the supplied string is empty, sanitizes
to an empty string, or insertion fails under the editor's normal limits. These
cases must not leave authority for a second reply. An independent capture is
not consumed by another capture's empty result.

Foreign/stale targets are rejected before claiming the capture. A busy buffer
or a tree awaiting layout returns no actions and does not consume it. You may
explicitly retry with a clone after resolving that temporary condition, provided
its stamps are still current. The tree does not retain the text or schedule a
retry. The return value alone does not distinguish all rejection/no-op cases;
keep retry policy in the host rather than retrying every empty action list.

For CodeEditor, the captured buffer state is checked and edited under the same
nonblocking try_lock guard. A check on a snapshot followed by another lock would
allow the destination to change between validation and insertion. Accepted edits
preserve the editor's normal normalization, selection, Vim/read-only/disabled
rules, undo history, syntax invalidation and EditorChanged actions. A successful
validation does not guarantee an insertion or an EditorChanged action.

## What invalidates a capture

A real focus change, native focus loss/recovery, menu/modal transition, or an
observed change of field kind, lifetime or editability cuts the old destination.
Changing text, caret or selection also cuts it. Returning to the same ID, bytes,
revision-looking state or selection does not restore the capture.

A new root marks layout pending. Capture and delivery are unavailable until the
next layout. An equivalent tree rebuild or resize/reflow can retain the capture
after that layout; rebuilding alone is not permanent invalidation. Once a layout
observes removal, read-only, disabled or a different kind/scope, changing the
props back does not revive old targets.

CodeEditor mode changes are part of the editable state. Vim Insert at caret zero,
Escape to Normal, and `i` back to Insert can leave text/revision/selection equal;
the earlier target is still stale. A fresh target in Insert can be used again.

Starting or ending an observed IME composition cuts captures in both ordinary
inputs and CodeEditor, including preedit followed by cancellation without a
commit. Capture is unavailable during an active, uncancelled composition. An
empty preedit does not by itself release its existing owner: it may precede a
commit, and the composition's original scope/cancellation must be preserved.
After cancellation, a fresh capture may succeed while the earlier one stays
stale.

## Shared code buffers

`limn_editor::TextBuffer::edit_state_token()` returns an opaque EditStateToken.
It distinguishes buffer instances, text edits including undo/redo, and real
primary or secondary selection changes. Selection setters which make no actual
change preserve it. Access through EditorController::shared_buffer is covered;
the token does not retain a buffer snapshot or document contents.

```rust
use limn_editor::{Selection, TextBuffer};

fn main() {
    let mut buffer = TextBuffer::new("text").expect("small document");
    let original = buffer.edit_state_token();
    assert_eq!(original, original.clone());

    buffer.set_selection(Selection::caret(0)).unwrap();
    assert_eq!(original, buffer.edit_state_token()); // unchanged selection

    buffer.set_selection(Selection::caret(1)).unwrap();
    buffer.set_selection(Selection::caret(0)).unwrap();
    assert_ne!(original, buffer.edit_state_token()); // selection returned, identity did not
}
```

The token is not permission to edit. Compare it while holding the same buffer
guard as the edit which follows. Text revisions do not wrap, and a newly created
buffer with equal contents does not inherit the prior instance's token.

## Host responsibilities and limits

limn cannot observe an application changing an account/document/endpoint mapping
and restoring it before the tree sees the intermediate state. The same applies
to a raw buffer replacement restored between observations or semantic props
changed and restored without layout. Call invalidate_paste_targets at each such
reducer/lifetime cut, even when the final IDs match. Do not try to infer these
changes from a final-frame comparison or scan arbitrary application state.

Forward native window focus to UiTree::set_window_focused. Process navigation,
clicks and other host-owned keys at their actual positions in the event batch.
Apply earlier state updates and pending focus, then lay out before capturing a
destination. A backend's focused-widget memory is not the application's current
editable document. Closing, unresolved focus or missing fresh layout must not
start a clipboard read.

For a deferred clipboard protocol, the same application-owned clipboard service
must reserve first without reading, validate its endpoint/token and receive an
ACK only after the target is resolved. Keep completion tied to that request and
target. Embedded native-view fallbacks use the actual parent endpoint. None of
that service, ACK, completion, transfer limit or SDK event mapping is supplied by
this API. Never convert a late completion into a generic Paste for the currently
active field.

Text paste checks the current editable control and its text policy. It does not
grant send-message/file-upload permission or trigger those actions. Existing
PasteTo remains an ID/focus-based legacy path; it does not gain the captured
path's lifetime guarantees. Adding UiEvent::PasteCaptured requires external
exhaustive matches to handle the new variant.

See [Backend contract](backend.md) for event delivery and
[CodeEditor](components/code-editor.md) for editor behavior.

## Synthetic Gallery example

Run the two visible destinations with synthetic Unicode text. The example checks
a valid delivery, a stale reply after invalidation and an accepted empty reply
followed by a rejected replay. `--verify` closes after the checks succeed and
returns an error if verification does not complete. These commands select the
four theme/corner combinations:

```sh
cargo run -p limn-gallery --example captured_paste -- --verify
cargo run -p limn-gallery --example captured_paste -- --verify --light
cargo run -p limn-gallery --example captured_paste -- --verify --square
cargo run -p limn-gallery --example captured_paste -- --verify --light --square
```

Without `--verify`, use the limn Run, Close, theme and corner controls. To save
its own window after successful verification, add `--screenshot new.png`.
Screenshot mode implies verification and refuses to overwrite an existing path;
a failed verification or save is an error rather than a successful capture.

The ten-second logical deadline requires app logic ticks. Use an external
watchdog to cover startup failures or missing redraws; the internal deadline
is not a hard wall-clock guarantee. This is a synthetic simulation which does
not request clipboard reads or writes. eframe may still initialize its default
clipboard backend. Passing it does not prove real clipboard, deferred-service
ACK/completion or native host event ordering; native visual QA remains separate.
