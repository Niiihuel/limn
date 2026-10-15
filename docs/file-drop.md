# Native file drops

`limn-dnd::FileDropReceiver` implements local-file URI-list receiving for
Linux/Wayland. It negotiates Copy only, reports the gesture before reading
its metadata, and asks the application to accept or reject the complete list. It never
opens a selected file, prepares its contents, or sends a message.
Native interoperability is compositor-specific: a private nested Weston run
verified metadata, explicit decisions and normal closure; the tested
eframe/Hyprland combination did not deliver the gesture to this receiver.
See the validation scope below.

For several surfaces on one connection, use the explicit
[`WaylandDataOwner`](#shared-owner-and-surface-endpoints) and register one
`FileDropEndpoint` per surface. This API is DnD-only; it does not yet provide
the clipboard backend needed to replace a stock SDK's data-device owner.

## Attach and control

The public methods are:

```rust,ignore
pub unsafe fn attach_owned<W>(
    window: Arc<W>,
    limits: ReceiveLimits,
    wake: impl Fn() + Send + 'static,
) -> Result<Option<FileDropReceiver>, ReceiveError>
where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static;

pub fn poll(&self) -> Vec<ReceiveEvent>;
pub fn decide(&self, id: OfferId, decision: DropDecision) -> Result<(), ReceiveError>;
pub fn cancel(&self, id: OfferId);
pub fn invalidate_pending(&self) -> Result<ReceiveEpoch, ReceiveError>;
pub fn close(&self);
pub fn status(&self) -> ReceiveStatus;
```

The handle traits come from `raw-window-handle`; the other types are exported
by `limn-dnd`. Validate custom limits with `ReceiveLimits::validate()`.
Invalid limits fail before querying handles. Non-Wayland handles return
`Ok(None)` without a fallback. Successful admission returns in `Starting`;
missing protocol support is an asynchronous failure, followed by cleanup.
The required manager is announced at least at version 3 and bound at 3;
compatible seats are announced at least at 5 and bound at 5.

`wake` runs on a service thread and requires `Send`, not `Sync`. It can ask
the host to repaint; the UI then drains events with `poll`. `cancel` and
`close` are infallible requests and never wait for worker progress.

## Shared owner and surface endpoints

`WaylandDataOwner` owns one worker and registry for the connection, and one
data device per compatible seat. It starts with no surfaces; an empty owner can reach
Ready and remain alive. `register_surface_owned` admits an actual surface on
that same display, with its own control, mailbox, context epoch and unique
Send-only wake. Adding a surface creates no worker, registry or data device.

```rust,ignore
pub unsafe fn attach_owned<W>(
    display_owner: Arc<W>,
    limits: ReceiveLimits,
    wake: impl Fn() + Send + 'static,
) -> Result<Option<WaylandDataOwner>, ReceiveError>
where W: HasDisplayHandle + Send + Sync + 'static;

pub unsafe fn register_surface_owned<W>(
    &self,
    window: Arc<W>,
    wake: impl Fn() + Send + 'static,
) -> Result<FileDropEndpoint, ReceiveError>
where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static;
```

The owner exposes `status()` and infallible `close()`. Each endpoint exposes
`poll`, `decide`, `cancel` and `invalidate_pending` with the receiving
semantics above, plus infallible `detach()` and `status() -> EndpointStatus`.
Neither handle is Clone; they may be shared with Arc. Limits are validated
before querying display handles. Unsupported attach returns `Ok(None)`;
unavailable or mismatched registration handles return `Unavailable`.

Endpoint status is `Registering`, `Ready`, `Detaching` or
`Detached(Option<ReceiveError>)`. Its first Ready deadline starts at pool
admission, including time waiting behind another surface's barrier. Neither
invalidation nor an old callback renews it; expiry retires only that endpoint.
The one physical sync is identified by callback, registration and phase. A
retired registration's pending callback remains a tombstone until its reply,
and cannot arm a replacement of the same native surface. Pending barriers are
scheduled round-robin among eligible registered surfaces.

There is one active drop lease across the owner's endpoints. B cannot replace
A while A is reading, awaiting a decision or holding an undrained Ended, even
after EOF has removed A's pipe. Draining A's exact Ended releases its lease.
Complete detach is the other release path: A's old terminal remains drainable
after B starts, and an old poll cannot release B or a new registration of A.

Detach cuts authority, retires local routing/offers/pipe, removes the runtime
watcher and its final reader ownership, then releases the registered window
Arc and surface permit before publishing Detached. That is not a thread join
or proof that the host destroyed the window. Keep the native surface valid
until Detached, and keep the display and coordinated host reader progressing
until the owner reaches Closed after cleanup and actual off-UI join. Owner
close cuts admission and all endpoint headers synchronously; an already-won
finish claim retains its authorized attempt. Drop requests close or detach
without waiting. Admitted resources retire off the caller; a rejected call
may drop its still-unadmitted arguments on the caller outside guards.

The explicit owner excludes other limn owners, standalone receivers and
legacy attachments on its entire display. Standalone receivers and legacy
attachments retain their previous surface-level exclusion between themselves.
The pool cannot detect another SDK's data devices: the integrator must ensure
there is no parallel clipboard/data-device owner on that connection. Stock
eframe initializes its own clipboard, so attaching this owner behind that
default is outside the contract. Do not force binding order, disable text
clipboard, or substitute a no-op clipboard backend to make an example work.

This slice still destroys Selection offers without receiving or finishing
them and does not bind Primary. A functional shared clipboard integration is
required before an SDK host demo can use this owner. The gallery's
[lifecycle illustration](gallery.md#shared-owner-lifecycle-illustration) is
synthetic and performs no native attachment. Private protocol/runtime tests
exercise the owner; the native receiver results below are not owner/clipboard
interoperability results.

## The unsafe lifetime obligation

`Arc<W>` retains the Rust owner; its handle traits do not prove that its
foreign display, surface or event loop stays valid. The caller must retain
those resources and preserve coordinated host reading and dispatch until
`status()` is `Closed(_)`. Readers must coordinate through libwayland/SDK;
an unrelated reader must not consume raw bytes from the shared socket.
Use the Arc and handles of the same target viewport.

For normal window closure, cancel each native close request, call `close`,
and keep the host event loop progressing until `Closed(_)`. Only then allow
the window/event loop to exit. This also applies while the window is hidden.
Calling `close` just before destroying the event loop is insufficient.
`Drop` requests closure without joining on the caller; it does not discharge
this unsafe obligation. The [standalone demo](gallery.md#native-file-drop-demo)
shows a close handshake.

## One offer and an explicit decision

`OfferId` and `ReceiveEpoch` are opaque, checked identities that never wrap.
An admitted drop follows `DropStarted { id, epoch, point }`, then either
`DropReady { id, paths }` or an error terminal, and one `DropEnded`.
Hover updates are coalesced. Leaving before the drop releases the hover
without promising an Ended; the normal Leave after Started does not cancel it.
The drop point is frozen in originating `wl_surface` coordinates. The host
must convert it for its scale, zoom and layout before choosing a target.

Ready requires EOF and validation of the entire list. A valid prefix cannot
produce Ready while the writer stays open. `paths` transfers to the caller;
the caller controls its lifetime and the thread that eventually drops it.
Debug output and `ReceiveError` omit names and paths.

After checking its own target and policy, the application calls
`decide(id, DropDecision::Accept)` or `Reject`. Accept before Ready returns
`InvalidPhase`; a stale decision returns `StaleOffer`. Reject or `cancel(id)`
can stop a transfer before Ready. A new offer cannot replace an active one,
and the next drop waits until `poll` retires the previous Ended.

Admitting Accept does not finish Copy immediately. The worker checks the
current offer, Copy action and deadline, then claims the attempt atomically.
Cancel, invalidation, close or timeout winning before that claim means no
finish call. A claimed attempt continues even if a later control arrives.
`DropEnded { result: Ok(()), .. }` means only that the typed local finish
request returned Ok. It does not prove buffering or transmission by
libwayland, compositor receipt or ACK, file preparation, or application send.
Move, Ask, None and unknown selected actions are rejected.

## Context changes

Call `invalidate_pending()` when navigation, permissions, target layout or
other context changes could redirect a pending gesture. It disarms admission
synchronously and returns the new epoch. Save that epoch, associate Started
with the current target, and reject events from an older epoch, including
a batch already returned by `poll`.

The worker performs at most one guest-queue sync at a time, keeps the latest
requested epoch, and requires a new Enter after its current barrier completes.
A stale callback cannot rearm a newer context. This is a cut in observed
protocol order, not proof of when a physical gesture happened. `Ready` status
describes startup completion; it does not bypass a pending context barrier.

## Limits and worker behavior

Defaults are a 64 KiB URI list, 32 files, 4096 bytes per decoded path and
64 KiB of decoded paths in total. Hard ceilings are respectively 1 MiB,
256 files, 64 KiB and 1 MiB. Limits count bytes, including Unix non-UTF-8 names
represented by escapes. Startup, transfer and decision deadlines default to
5 s, 5 s and 2 s; each must be positive and at most 60 s. Startup has one
deadline through first Ready, including stale barriers. Partial input never
renews a transfer deadline.

The strict codec accepts absolute local `file://` URIs (empty or `localhost`
authority). It rejects the whole batch on malformed escapes, remote URIs,
control bytes, dot segments, duplicates or any exceeded limit. It decodes
once, preserves decoded bytes, and never canonicalizes, stats or opens paths.

Each worker has one active drop and pipe, at most four compatible seats and eight
offers including active, unclassified and clipboard offers. The mailbox has
five reliable slots plus one coalesced hover slot per receiving surface;
`poll` drains at most six. The process shares four worker permits and four
surface permits with legacy FileDnd. A standalone receiver or legacy
attachment consumes one of each. An explicit owner consumes one worker and
its endpoints share the four global surface permits; retiring one endpoint
does not return its owner's worker or display lease. Owned workers use one
global reaper. All paths share a non-refundable budget
of 64 initial get_registry attempts; startup failure or close does not restore
it. An existing Ready receiver needs no new registry to receive more drops.

The worker uses Calloop Ping for controls, level-triggered readiness on the
same pipe reader, and a guest-queue tick of at most 20 ms between attempts,
shortened by deadlines. Readable chunks do not wait for the next tick. It
does not read or prepare a read of the shared Wayland socket. Flush WouldBlock
is retried without blocking. The watcher can retain the reader after EOF
through decoding; it is removed in the same worker turn, before cleanup.

Closed requires actual guest cleanup, off-UI join, and release of the retained
owner and slot. SDK dispatch/flood, TLS or a destructor may prevent progress:
there is no hard teardown deadline in that case. Status remains Closing,
resources stay reserved, and reattachment cannot evade the four-slot bound.
These bounds cover library state, not all C/SDK queues or transport memory.

## Legacy and validation scope

`unsafe FileDnd::attach(&window, wake)` keeps its borrowed-handle contract and
synchronous Drop/join. Its outgoing drags and per-drop reader threads are
outside the receive-only hardening. Do not attach both APIs to one surface;
shared surface exclusion rejects this. Migrate receiving to FileDropReceiver
when explicit decisions and asynchronous closure are required.

Private Wayland-server tests exercise startup, identity, codec, real pipe EOF,
Copy decisions, runtime readiness and cleanup. A real fixture-only test on
2026-10-15 used Hyprland 0.55.0, Dolphin 26.04.3 and the eframe 0.36.2 demo.
Startup reached Ready and the source issued `start_drag`, but no drop metadata
reached FileDropReceiver. The trace showed the host clipboard's data device
being created first and receiving Enter/Motion/Leave; the receiver's separate
device received none. The demo closed normally, without an accepted drop.

Ready establishes initialization, not working native DnD or detection of this
host conflict. On this compositor, sharing a client with an existing clipboard
data device limits the current integration. This observation does not claim
that Wayland requires broadcast or that a compositor violates the protocol.
A correct integration needs shared data-device ownership and a host/SDK hook
that preserves clipboard text and routes drops to the appropriate surface.
Forcing binding order or disabling clipboard support is not a solution.

A later fixture-only run on private nested Weston 15.0.1, with Dolphin and the
same eframe demo, received two basenames for each of two offers. An explicit
native Accept Copy click on the first offer produced Ended Ok; an explicit
Reject click on the second produced Ended Cancelled. The demo then exited
normally with code 0 after its Close control was activated. Only afterwards
were the owned source and Weston processes terminated. The receiver demo did
not open file contents; no application upload or message send was involved. Ended Ok keeps
its local typed finish-request meaning; this run does not establish remote
receipt or an ACK.

The earlier nested run received metadata but cancelled when growing details
made the outer scrollbar change the frozen target width. Two real UiTree
regressions reproduced the 14 px change. The demo now keeps its header, target
and Close control outside the scroll area; only details, names and logs scroll.
That earlier run timed out and was terminated, so it is not a normal-close
proof. The corrected receiver checkpoint had three demo tests, alongside 124 library tests. The
2026-10-15 `cargo test --workspace --offline` result was 1646 passed and five
ignored across 21 suites, including doctests; those totals are limn's, not an
application integration result.

The Weston result does not resolve the observed Hyprland clipboard conflict,
verify clipboard functionality or establish support for other combinations.
Applications should keep an explicit file-picker fallback.
