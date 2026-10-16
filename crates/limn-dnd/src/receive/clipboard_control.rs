//! Bounded metadata admission shared by clipboard requests and file drops.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, MutexGuard, Weak,
    },
    time::Instant,
};

use super::{
    control::{Header, IncomingLease, ReceiveControl},
    worker_pool::SurfaceId,
    ReceiveEpoch, ReceiveStatus,
};
use crate::{ClipboardError, TextClipboardLimits};

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
static NEXT_STAMP: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct TextRequestId(u64);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ClipboardStamp(u64);

fn allocate_text_request_id(counter: &AtomicU64) -> Result<TextRequestId, ClipboardError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .map(TextRequestId)
        .map_err(|_| ClipboardError::ResourceLimit)
}

fn next_text_request_id() -> Result<TextRequestId, ClipboardError> {
    allocate_text_request_id(&NEXT_REQUEST)
}

pub(super) fn next_clipboard_stamp() -> Result<ClipboardStamp, ClipboardError> {
    NEXT_STAMP
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .map(ClipboardStamp)
        .map_err(|_| ClipboardError::ResourceLimit)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextMime {
    Utf8,
    Plain,
}

#[derive(Clone, Copy)]
pub(super) struct TextSelection {
    pub(super) id: ClipboardStamp,
    pub(super) utf8: bool,
    pub(super) plain: bool,
}

#[derive(Clone, Copy)]
pub(super) struct TextFocus {
    pub(super) generation: ClipboardStamp,
    pub(super) registration: Option<SurfaceId>,
    pub(super) sealed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct TextSeatSnapshot {
    pub(super) seat: ClipboardStamp,
    pub(super) keyboard_generation: ClipboardStamp,
    pub(super) focus: Option<TextFocus>,
    pub(super) selection: Option<TextSelection>,
}

pub(super) struct TextFocusSeal {
    pub(super) seat: ClipboardStamp,
    pub(super) keyboard_generation: ClipboardStamp,
    pub(super) focus_generation: ClipboardStamp,
    pub(super) registration: SurfaceId,
    pub(super) epoch: ReceiveEpoch,
    pub(super) fence_deadline: Option<Instant>,
}

pub(super) struct TextHeader {
    limits: TextClipboardLimits,
    seats: [Option<TextSeatSnapshot>; 4],
    current: Option<Arc<TextRequestCore>>,
}

#[derive(Clone, Copy)]
struct Snapshot {
    seat: ClipboardStamp,
    keyboard: ClipboardStamp,
    focus: ClipboardStamp,
    selection: ClipboardStamp,
    mime: TextMime,
}

#[derive(Clone, Copy)]
enum RequestPhase {
    Reserved { deadline: Instant },
    Acknowledged { deadline: Instant },
    Claimed { deadline: Instant },
    Published { deadline: Instant },
    Consumed,
    Ended { error: ClipboardError, polled: bool },
}

impl RequestPhase {
    fn deadline(self) -> Option<Instant> {
        match self {
            Self::Reserved { deadline }
            | Self::Acknowledged { deadline }
            | Self::Claimed { deadline }
            | Self::Published { deadline } => Some(deadline),
            Self::Consumed | Self::Ended { .. } => None,
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Consumed | Self::Ended { .. })
    }

    fn end(&mut self, error: ClipboardError) {
        if !self.is_terminal() {
            *self = Self::Ended {
                error,
                polled: false,
            };
        }
    }

    fn take_error(&mut self) -> Option<ClipboardError> {
        if let Self::Ended { error, polled } = self {
            if !*polled {
                *polled = true;
                return Some(*error);
            }
        }
        None
    }
}

/// Never contains a control, callback, native object, or payload owner.
pub(super) struct TextRequestCore {
    id: TextRequestId,
    registration: SurfaceId,
    epoch: ReceiveEpoch,
    snapshot: Snapshot,
    phase: Mutex<RequestPhase>,
}

impl TextRequestCore {
    fn phase(&self) -> MutexGuard<'_, RequestPhase> {
        self.phase
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn cancel(&self, error: ClipboardError) {
        let mut phase = self.phase();
        phase.end(error);
    }
}

impl TextHeader {
    pub(super) fn cancel(&self, error: ClipboardError) {
        if let Some(core) = self.current.as_ref() {
            core.cancel(error);
        }
    }

    fn snapshot_is_live(&self, core: &TextRequestCore) -> bool {
        self.seats.iter().flatten().any(|seat| {
            seat.seat == core.snapshot.seat
                && seat.keyboard_generation == core.snapshot.keyboard
                && seat.focus.is_some_and(|focus| {
                    focus.registration == Some(core.registration)
                        && focus.sealed
                        && focus.generation == core.snapshot.focus
                })
                && seat.selection.is_some_and(|selection| {
                    selection.id == core.snapshot.selection
                        && match core.snapshot.mime {
                            TextMime::Utf8 => selection.utf8,
                            TextMime::Plain => selection.plain,
                        }
                })
        })
    }
}

pub(super) struct ClipboardControl {
    root: ReceiveControl,
    body: Weak<Mutex<TextBodySlot>>,
}
pub(super) struct ClipboardWorker {
    root: ReceiveControl,
    body: Arc<Mutex<TextBodySlot>>,
    staged: Option<(TextRequestId, String)>,
}

struct TextBodySlot {
    body: Option<(TextRequestId, String)>,
}

impl TextBodySlot {
    fn take(&mut self, request: TextRequestId) -> Option<(TextRequestId, String)> {
        if self.body.as_ref().is_some_and(|(id, _)| *id == request) {
            self.body.take()
        } else {
            None
        }
    }
}

/// Strong control references live outside the root's current Core.
pub(super) struct TextRequestControl {
    root: ReceiveControl,
    endpoint: ReceiveControl,
    core: Arc<TextRequestCore>,
    body: Weak<Mutex<TextBodySlot>>,
}

pub(super) struct TextReadClaim {
    pub(super) request: TextRequestId,
    pub(super) seat: ClipboardStamp,
    pub(super) selection: ClipboardStamp,
    pub(super) mime: TextMime,
    pub(super) deadline: Instant,
    pub(super) max_text_bytes: usize,
}

fn validate_context(
    root: &Header,
    endpoint: &Header,
    core: &TextRequestCore,
) -> Result<(), ClipboardError> {
    if root.is_closing() || endpoint.is_closing() {
        return Err(ClipboardError::Closed);
    }
    if root.status != ReceiveStatus::Ready || endpoint.status != ReceiveStatus::Ready {
        return Err(ClipboardError::Unavailable);
    }
    if endpoint.epoch != core.epoch || endpoint.text_sealed != Some((core.registration, core.epoch))
    {
        return Err(ClipboardError::Cancelled);
    }
    let text = root.text.as_ref().ok_or(ClipboardError::Unavailable)?;
    if root.incoming.current() != Some(IncomingLease::Text(core.id))
        || !text
            .current
            .as_ref()
            .is_some_and(|current| current.id == core.id)
    {
        return Err(ClipboardError::StaleRequest);
    }
    if !text.snapshot_is_live(core) {
        return Err(ClipboardError::Cancelled);
    }
    Ok(())
}

impl ClipboardControl {
    pub(super) fn new(
        root: ReceiveControl,
        limits: TextClipboardLimits,
    ) -> Result<(Self, ClipboardWorker), ClipboardError> {
        limits.validate()?;
        if !root.is_root() {
            return Err(ClipboardError::StaleRequest);
        }
        {
            let mut header = root.header();
            if header.is_closing() {
                return Err(ClipboardError::Closed);
            }
            if header.text.is_some() {
                return Err(ClipboardError::InvalidPhase);
            }
            header.text = Some(TextHeader {
                limits,
                seats: [None; 4],
                current: None,
            });
        }
        let body = Arc::new(Mutex::new(TextBodySlot { body: None }));
        Ok((
            Self {
                root: root.clone(),
                body: Arc::downgrade(&body),
            },
            ClipboardWorker {
                root,
                body,
                staged: None,
            },
        ))
    }

    pub(super) fn reserve(
        &self,
        endpoint: &ReceiveControl,
        registration: SurfaceId,
        now: Instant,
    ) -> Result<TextRequestControl, ClipboardError> {
        if !endpoint.belongs_to(&self.root) || endpoint.registration() != Some(registration) {
            return Err(ClipboardError::StaleRequest);
        }
        let core = {
            let mut root = self.root.header();
            let mut target = endpoint.header();
            if root.is_closing() || target.is_closing() {
                return Err(ClipboardError::Closed);
            }
            if root.status != ReceiveStatus::Ready
                || target.status != ReceiveStatus::Ready
                || target.text_sealed != Some((registration, target.epoch))
            {
                return Err(ClipboardError::Unavailable);
            }
            let text = root.text.as_ref().ok_or(ClipboardError::Unavailable)?;
            let mut focused = text.seats.iter().flatten().filter(|seat| {
                seat.focus
                    .is_some_and(|focus| focus.registration == Some(registration) && focus.sealed)
            });
            let seat = focused.next().ok_or(ClipboardError::Unavailable)?;
            if focused.next().is_some() {
                return Err(ClipboardError::AmbiguousSeat);
            }
            let selection = seat.selection.ok_or(ClipboardError::NoSelection)?;
            let mime = if selection.utf8 {
                TextMime::Utf8
            } else if selection.plain {
                TextMime::Plain
            } else {
                return Err(ClipboardError::Unavailable);
            };
            if root.incoming.current().is_some() {
                return Err(ClipboardError::Busy);
            }
            let now = now.max(Instant::now());
            let deadline = now
                .checked_add(text.limits.acknowledgement_timeout)
                .ok_or(ClipboardError::DeadlineOverflow)?;
            let core = Arc::new(TextRequestCore {
                id: next_text_request_id()?,
                registration,
                epoch: target.epoch,
                snapshot: Snapshot {
                    seat: seat.seat,
                    keyboard: seat.keyboard_generation,
                    focus: seat.focus.expect("validated focused seat").generation,
                    selection: selection.id,
                    mime,
                },
                phase: Mutex::new(RequestPhase::Reserved { deadline }),
            });
            root.incoming
                .acquire(IncomingLease::Text(core.id))
                .map_err(|_| ClipboardError::Busy)?;
            root.text.as_mut().expect("validated text policy").current = Some(core.clone());
            target.set_text_request(Arc::downgrade(&core));
            core
        };
        Ok(TextRequestControl {
            root: self.root.clone(),
            endpoint: endpoint.clone(),
            core,
            body: self.body.clone(),
        })
    }
}

impl TextRequestControl {
    #[cfg(test)]
    pub(super) fn id(&self) -> TextRequestId {
        self.core.id
    }

    pub(super) fn acknowledge(&self, now: Instant) -> Result<(), ClipboardError> {
        let root = self.root.header();
        let endpoint = self.endpoint.header();
        let mut phase = self.core.phase();
        if let RequestPhase::Ended { error, .. } = *phase {
            return Err(error);
        }
        if let Err(error) = validate_context(&root, &endpoint, &self.core) {
            phase.end(error);
            return Err(error);
        }
        let RequestPhase::Reserved { deadline } = *phase else {
            return Err(ClipboardError::InvalidPhase);
        };
        let now = now.max(Instant::now());
        if now >= deadline {
            phase.end(ClipboardError::Timeout);
            return Err(ClipboardError::Timeout);
        }
        let Some(deadline) = now.checked_add(
            root.text
                .as_ref()
                .expect("validated text policy")
                .limits
                .read_timeout,
        ) else {
            phase.end(ClipboardError::DeadlineOverflow);
            return Err(ClipboardError::DeadlineOverflow);
        };
        *phase = RequestPhase::Acknowledged { deadline };
        Ok(())
    }

    pub(super) fn cancel(&self) {
        self.core.cancel(ClipboardError::Cancelled);
    }
}

impl Drop for TextRequestControl {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl ClipboardWorker {
    pub(super) fn is_reading(&self, request: TextRequestId) -> bool {
        let root = self.root.header();
        let Some(core) = root.text.as_ref().and_then(|text| text.current.as_ref()) else {
            return false;
        };
        let reading = core.id == request
            && root.incoming.current() == Some(IncomingLease::Text(request))
            && matches!(*core.phase(), RequestPhase::Claimed { .. });
        reading
    }

    pub(super) fn observe_seat(
        &mut self,
        seat: ClipboardStamp,
        snapshot: Option<TextSeatSnapshot>,
    ) -> Result<(), ClipboardError> {
        if snapshot.is_some_and(|snapshot| snapshot.seat != seat) {
            return Err(ClipboardError::StaleRequest);
        }
        let mut root = self.root.header();
        if root.is_closing() {
            return Err(ClipboardError::Closed);
        }
        let text = root.text.as_mut().ok_or(ClipboardError::Unavailable)?;
        let index = text
            .seats
            .iter()
            .position(|current| current.is_some_and(|current| current.seat == seat));
        if let Some(mut snapshot) = snapshot {
            let index = index
                .or_else(|| text.seats.iter().position(Option::is_none))
                .ok_or(ClipboardError::ResourceLimit)?;
            if let (Some(old), Some(selection)) = (text.seats[index], snapshot.selection.as_mut()) {
                if let Some(old) = old.selection.filter(|old| old.id == selection.id) {
                    selection.utf8 |= old.utf8;
                    selection.plain |= old.plain;
                }
            }
            text.seats[index] = Some(snapshot);
        } else if let Some(index) = index {
            text.seats[index] = None;
        }
        if let Some(core) = text.current.as_ref() {
            if !text.snapshot_is_live(core) {
                core.cancel(ClipboardError::Cancelled);
            }
        }
        Ok(())
    }

    pub(super) fn seal_endpoint(
        &mut self,
        endpoint: &ReceiveControl,
        registration: SurfaceId,
        seal: TextFocusSeal,
    ) -> Result<(), ClipboardError> {
        if !endpoint.belongs_to(&self.root)
            || endpoint.registration() != Some(registration)
            || seal.registration != registration
        {
            return Err(ClipboardError::StaleRequest);
        }
        let mut root = self.root.header();
        let mut target = endpoint.header();
        if seal
            .fence_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ClipboardError::Timeout);
        }
        if root.is_closing() || target.is_closing() {
            return Err(ClipboardError::Closed);
        }
        if root.status != ReceiveStatus::Ready || target.status != ReceiveStatus::Ready {
            return Err(ClipboardError::Unavailable);
        }
        if target.epoch != seal.epoch {
            return Err(ClipboardError::StaleRequest);
        }
        let text = root.text.as_mut().ok_or(ClipboardError::Unavailable)?;
        let seat = text
            .seats
            .iter_mut()
            .flatten()
            .find(|seat| seat.seat == seal.seat)
            .ok_or(ClipboardError::StaleRequest)?;
        if seat.keyboard_generation != seal.keyboard_generation {
            return Err(ClipboardError::StaleRequest);
        }
        let focus = seat.focus.as_mut().ok_or(ClipboardError::StaleRequest)?;
        if focus.registration != Some(registration) || focus.generation != seal.focus_generation {
            return Err(ClipboardError::StaleRequest);
        }
        focus.sealed = true;
        target.text_sealed = Some((registration, seal.epoch));
        Ok(())
    }

    pub(super) fn pending_request(&self) -> Option<(TextRequestId, SurfaceId)> {
        let root = self.root.header();
        let core = root.text.as_ref()?.current.as_ref()?;
        let acknowledged = matches!(*core.phase(), RequestPhase::Acknowledged { .. });
        acknowledged.then_some((core.id, core.registration))
    }

    pub(super) fn claim_receive(
        &mut self,
        request: TextRequestId,
        endpoint: &ReceiveControl,
        registration: SurfaceId,
        now: Instant,
    ) -> Result<TextReadClaim, ClipboardError> {
        if !endpoint.belongs_to(&self.root) || endpoint.registration() != Some(registration) {
            return Err(ClipboardError::StaleRequest);
        }
        let root = self.root.header();
        let target = endpoint.header();
        let core = root
            .text
            .as_ref()
            .and_then(|text| text.current.as_ref())
            .filter(|core| core.id == request && core.registration == registration)
            .ok_or(ClipboardError::StaleRequest)?;
        let mut phase = core.phase();
        if let RequestPhase::Ended { error, .. } = *phase {
            return Err(error);
        }
        if let Err(error) = validate_context(&root, &target, core) {
            phase.end(error);
            return Err(error);
        }
        let RequestPhase::Acknowledged { deadline } = *phase else {
            return Err(ClipboardError::InvalidPhase);
        };
        let now = now.max(Instant::now());
        if now >= deadline {
            phase.end(ClipboardError::Timeout);
            return Err(ClipboardError::Timeout);
        }
        *phase = RequestPhase::Claimed { deadline };
        Ok(TextReadClaim {
            request,
            seat: core.snapshot.seat,
            selection: core.snapshot.selection,
            mime: core.snapshot.mime,
            deadline,
            max_text_bytes: root
                .text
                .as_ref()
                .expect("validated text policy")
                .limits
                .max_text_bytes,
        })
    }

    pub(super) fn reconcile(&mut self, now: Instant) {
        let root = self.root.header();
        let Some(text) = root.text.as_ref() else {
            return;
        };
        let Some(core) = text.current.as_ref() else {
            return;
        };
        let mut phase = core.phase();
        if phase.is_terminal() {
            return;
        }
        if root.is_closing() {
            phase.end(ClipboardError::Closed);
        } else if !text.snapshot_is_live(core) {
            phase.end(ClipboardError::Cancelled);
        } else if phase
            .deadline()
            .is_some_and(|deadline| now.max(Instant::now()) >= deadline)
        {
            phase.end(ClipboardError::Timeout);
        }
    }

    pub(super) fn retire_after_cleanup(&mut self, request: TextRequestId) -> bool {
        let retired = {
            let mut root = self.root.header();
            let Some(core) = root.text.as_ref().and_then(|text| text.current.as_ref()) else {
                return false;
            };
            if core.id != request
                || root.incoming.current() != Some(IncomingLease::Text(request))
                || !matches!(*core.phase(), RequestPhase::Ended { .. })
                || self.staged.is_some()
            {
                return false;
            }
            let slot = self
                .body
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if slot.body.is_some() {
                return false;
            }
            if !root.incoming.release(IncomingLease::Text(request)) {
                return false;
            }
            root.text
                .as_mut()
                .expect("current text request")
                .current
                .take()
        };
        drop(retired);
        true
    }
}

impl TextRequestControl {
    pub(super) fn poll(&self, now: Instant) -> Option<Result<String, ClipboardError>> {
        {
            let mut phase = self.core.phase();
            if phase.is_terminal() {
                return phase.take_error().map(Err);
            }
        }
        // A transient UI owner is obtained before, and dropped after, all guards.
        let Some(body) = self.body.upgrade() else {
            let mut phase = self.core.phase();
            phase.end(ClipboardError::Closed);
            return phase.take_error().map(Err);
        };
        let (result, retired) = {
            let mut root = self.root.header();
            let endpoint = self.endpoint.header();
            let mut phase = self.core.phase();
            let mut slot = body.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = now.max(Instant::now());
            if !phase.is_terminal() {
                if let Err(error) = validate_context(&root, &endpoint, &self.core) {
                    phase.end(error);
                } else if phase.deadline().is_some_and(|deadline| now >= deadline) {
                    phase.end(ClipboardError::Timeout);
                }
            }
            if phase.is_terminal() {
                (phase.take_error().map(Err), None)
            } else if matches!(*phase, RequestPhase::Published { .. }) {
                if let Some((_, text)) = slot.take(self.core.id) {
                    *phase = RequestPhase::Consumed;
                    root.incoming.release(IncomingLease::Text(self.core.id));
                    let retired = root.text.as_mut().and_then(|text| text.current.take());
                    (Some(Ok(text)), retired)
                } else {
                    phase.end(ClipboardError::StaleRequest);
                    (phase.take_error().map(Err), None)
                }
            } else {
                (None, None)
            }
        };
        drop(retired);
        drop(body);
        result
    }
}

impl ClipboardWorker {
    pub(super) fn current_request(&self) -> Option<(TextRequestId, SurfaceId)> {
        let root = self.root.header();
        let core = root.text.as_ref()?.current.as_ref()?;
        Some((core.id, core.registration))
    }

    pub(super) fn stage_validated(
        &mut self,
        request: TextRequestId,
        endpoint: &ReceiveControl,
        registration: SurfaceId,
        text: String,
        post_now: Instant,
    ) -> Result<(), ClipboardError> {
        let mut text = Some(text);
        let result = (|| {
            if !endpoint.belongs_to(&self.root) || endpoint.registration() != Some(registration) {
                return Err(ClipboardError::StaleRequest);
            }
            let root = self.root.header();
            let target = endpoint.header();
            let core = root
                .text
                .as_ref()
                .and_then(|text| text.current.as_ref())
                .filter(|core| core.id == request && core.registration == registration)
                .ok_or(ClipboardError::StaleRequest)?;
            let mut phase = core.phase();
            let slot = self
                .body
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = post_now.max(Instant::now());
            if let RequestPhase::Ended { error, .. } = *phase {
                return Err(error);
            }
            if let Err(error) = validate_context(&root, &target, core) {
                phase.end(error);
                return Err(error);
            }
            let RequestPhase::Claimed { deadline } = *phase else {
                return Err(ClipboardError::InvalidPhase);
            };
            if self.staged.is_some() || slot.body.is_some() {
                return Err(ClipboardError::InvalidPhase);
            }
            if now >= deadline {
                phase.end(ClipboardError::Timeout);
                return Err(ClipboardError::Timeout);
            }
            if text.as_ref().expect("owned stage argument").len()
                > root
                    .text
                    .as_ref()
                    .expect("validated text policy")
                    .limits
                    .max_text_bytes
            {
                phase.end(ClipboardError::TooLarge);
                return Err(ClipboardError::TooLarge);
            }
            self.staged = Some((request, text.take().expect("owned stage argument")));
            Ok(())
        })();
        // A rejected argument remains on the admitted worker, outside guards.
        drop(text);
        result
    }

    pub(super) fn fail_receive(&mut self, request: TextRequestId, error: ClipboardError) -> bool {
        if !matches!(
            error,
            ClipboardError::Io
                | ClipboardError::TooLarge
                | ClipboardError::InvalidText
                | ClipboardError::Timeout
        ) {
            return false;
        }
        let root = self.root.header();
        let Some(core) = root.text.as_ref().and_then(|text| text.current.as_ref()) else {
            return false;
        };
        if core.id != request || root.incoming.current() != Some(IncomingLease::Text(request)) {
            return false;
        }
        let mut phase = core.phase();
        if !matches!(*phase, RequestPhase::Claimed { .. }) {
            return false;
        }
        phase.end(error);
        true
    }

    /// The caller certifies real watcher removal, or that none was ever installed.
    pub(super) fn finish_after_watcher_removed(
        &mut self,
        request: TextRequestId,
        endpoint: Option<&ReceiveControl>,
        registration: SurfaceId,
        now: Instant,
    ) -> Result<bool, ClipboardError> {
        if endpoint.is_some_and(|endpoint| {
            !endpoint.belongs_to(&self.root) || endpoint.registration() != Some(registration)
        }) {
            return Err(ClipboardError::StaleRequest);
        }
        let mut staged_retirement = None;
        let mut body_retirement = None;
        let (published, cleanup) = {
            let root = self.root.header();
            let target = endpoint.map(ReceiveControl::header);
            let core = root
                .text
                .as_ref()
                .and_then(|text| text.current.as_ref())
                .filter(|core| core.id == request && core.registration == registration)
                .ok_or(ClipboardError::StaleRequest)?;
            if root.incoming.current() != Some(IncomingLease::Text(request)) {
                return Err(ClipboardError::StaleRequest);
            }
            let mut phase = core.phase();
            let mut slot = self
                .body
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = now.max(Instant::now());
            if !phase.is_terminal() {
                if let Some(target) = target.as_deref() {
                    if let Err(error) = validate_context(&root, target, core) {
                        phase.end(error);
                    }
                } else {
                    phase.end(if root.is_closing() {
                        ClipboardError::Closed
                    } else {
                        ClipboardError::Cancelled
                    });
                }
                if phase.deadline().is_some_and(|deadline| now >= deadline) {
                    phase.end(ClipboardError::Timeout);
                }
            }
            if matches!(*phase, RequestPhase::Claimed { .. })
                && self.staged.as_ref().is_some_and(|(id, _)| *id == request)
            {
                if slot.body.is_some() {
                    return Err(ClipboardError::InvalidPhase);
                }
                if let Some(deadline) = now.checked_add(
                    root.text
                        .as_ref()
                        .expect("validated text policy")
                        .limits
                        .completion_ttl,
                ) {
                    slot.body = self.staged.take();
                    *phase = RequestPhase::Published { deadline };
                    (true, false)
                } else {
                    phase.end(ClipboardError::DeadlineOverflow);
                    staged_retirement = self.staged.take();
                    body_retirement = slot.take(request);
                    (false, true)
                }
            } else if matches!(*phase, RequestPhase::Ended { .. }) {
                if self.staged.as_ref().is_some_and(|(id, _)| *id == request) {
                    staged_retirement = self.staged.take();
                }
                body_retirement = slot.take(request);
                (false, true)
            } else {
                (false, false)
            }
        };
        // Keep Incoming busy until the discarded admitted body is truly gone.
        drop(staged_retirement);
        drop(body_retirement);
        if cleanup {
            Ok(self.retire_after_cleanup(request))
        } else {
            Ok(published)
        }
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        let root = self.root.header();
        let core = root.text.as_ref()?.current.as_ref()?;
        let deadline = core.phase().deadline();
        deadline
    }
}

impl Drop for ClipboardWorker {
    fn drop(&mut self) {
        let (staged, body) = {
            let _root = self.root.header();
            let mut slot = self
                .body
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (self.staged.take(), slot.body.take())
        };
        // A temporary UI Arc may become last only after the internal slot is empty.
        drop(staged);
        drop(body);
    }
}

#[cfg(test)]
#[path = "clipboard_control_tests.rs"]
mod tests;

#[cfg(test)]
impl ClipboardControl {
    pub(super) fn observed_seats(&self) -> [Option<TextSeatSnapshot>; 4] {
        self.root
            .header()
            .text
            .as_ref()
            .map_or([None; 4], |text| text.seats)
    }
}

pub(super) fn from_receive_error(error: crate::ReceiveError) -> ClipboardError {
    use crate::ReceiveError as Error;
    match error {
        Error::Unavailable => ClipboardError::Unavailable,
        Error::Busy => ClipboardError::Busy,
        Error::ResourceLimit => ClipboardError::ResourceLimit,
        Error::InvalidLimits => ClipboardError::InvalidLimits,
        Error::InvalidUriList => ClipboardError::InvalidText,
        Error::TooLarge | Error::TooMany => ClipboardError::TooLarge,
        Error::Cancelled => ClipboardError::Cancelled,
        Error::Timeout => ClipboardError::Timeout,
        Error::Io => ClipboardError::Io,
        Error::StaleOffer => ClipboardError::StaleRequest,
        Error::InvalidPhase => ClipboardError::InvalidPhase,
        Error::Closed => ClipboardError::Closed,
    }
}
