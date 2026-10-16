//! Selection/focus facts and text transfers on the owner's existing queue.

use std::os::fd::AsFd;

use sctk::reexports::client::WEnum;

use super::offers::OfferRole;
use super::*;
use crate::{
    receive::clipboard_control::{
        TextFocus, TextFocusSeal, TextMime, TextRequestId, TextSeatSnapshot, TextSelection,
    },
    ClipboardError,
};

pub(super) const UTF8: &str = "text/plain;charset=utf-8";
pub(super) const PLAIN: &str = "text/plain";

fn stamp() -> Result<ClipboardStamp, ReceiveError> {
    next_clipboard_stamp().map_err(|_| ReceiveError::ResourceLimit)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct KeyboardTag {
    seat: ClipboardStamp,
    keyboard_generation: ClipboardStamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClipboardFenceKind {
    Quarantine,
    Seal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::receive) struct ClipboardFenceTag {
    pub(super) kind: ClipboardFenceKind,
    seat: ClipboardStamp,
    keyboard_generation: ClipboardStamp,
    focus_generation: ClipboardStamp,
    selection: Option<ClipboardStamp>,
    endpoint: Option<(SurfaceId, ReceiveEpoch)>,
    cut: ClipboardStamp,
}

impl std::fmt::Debug for ClipboardFenceTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardFence")
            .field("kind", &self.kind)
            .field("endpoint", &self.endpoint.is_some())
            .finish_non_exhaustive()
    }
}

pub(super) struct SeatClipboard {
    pub(super) seat: ClipboardStamp,
    pub(super) keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_generation: ClipboardStamp,
    focus_generation: ClipboardStamp,
    focus: Option<ObjectId>,
    selection: Option<(ObjectId, ClipboardStamp)>,
    quarantined: bool,
    context: Option<(SurfaceId, ReceiveEpoch)>,
    sealed: bool,
    cut: ClipboardStamp,
    need: Option<ClipboardFenceTag>,
}

impl SeatClipboard {
    pub(super) fn new() -> Result<Self, ReceiveError> {
        Ok(Self {
            seat: stamp()?,
            keyboard: None,
            keyboard_generation: stamp()?,
            focus_generation: stamp()?,
            focus: None,
            selection: None,
            quarantined: false,
            context: None,
            sealed: false,
            cut: stamp()?,
            need: None,
        })
    }
}

impl State {
    #[cfg(test)]
    pub(super) fn observe_dispatch(&self) {
        if let Some(observed) = self.clipboard_observed.as_ref() {
            observed.guest_dispatches.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn clipboard_context(&self, seat: usize) -> Option<(SurfaceId, ReceiveEpoch)> {
        let focus = self.seats.get(seat)?.clipboard.as_ref()?.focus.as_ref()?;
        let endpoint = self
            .endpoints
            .iter()
            .find(|endpoint| !endpoint.retiring && endpoint.surface.as_ref() == Some(focus))?;
        let registration = endpoint.registration?;
        let epoch = endpoint.control.header().epoch;
        Some((registration, epoch))
    }

    fn clipboard_tag(&self, seat: usize, kind: ClipboardFenceKind) -> Option<ClipboardFenceTag> {
        let text = self.seats.get(seat)?.clipboard.as_ref()?;
        Some(ClipboardFenceTag {
            kind,
            seat: text.seat,
            keyboard_generation: text.keyboard_generation,
            focus_generation: text.focus_generation,
            selection: text.selection.as_ref().map(|(_, stamp)| *stamp),
            endpoint: text.context,
            cut: text.cut,
        })
    }

    fn publish_seat(&mut self, index: usize) -> Result<(), ReceiveError> {
        let Some(text) = self
            .seats
            .get(index)
            .and_then(|seat| seat.clipboard.as_ref())
        else {
            return Ok(());
        };
        let context = self.clipboard_context(index);
        let selection = text.selection.as_ref().and_then(|(object, stamp)| {
            self.offers
                .iter()
                .find(|offer| {
                    offer.proxy.id() == *object && offer.role == OfferRole::Selection(*stamp)
                })
                .map(|offer| TextSelection {
                    id: *stamp,
                    utf8: offer.utf8,
                    plain: offer.plain,
                })
        });
        let snapshot = TextSeatSnapshot {
            seat: text.seat,
            keyboard_generation: text.keyboard_generation,
            focus: text.focus.as_ref().map(|_| TextFocus {
                generation: text.focus_generation,
                registration: context.map(|(registration, _)| registration),
                sealed: text.sealed && context.is_some() && text.context == context,
            }),
            selection,
        };
        let Some(worker) = self.clipboard.as_mut() else {
            return Ok(());
        };
        match worker.observe_seat(snapshot.seat, Some(snapshot)) {
            Ok(()) | Err(ClipboardError::Closed) => Ok(()),
            Err(ClipboardError::ResourceLimit) => Err(ReceiveError::ResourceLimit),
            Err(_) => Err(ReceiveError::InvalidPhase),
        }
    }

    fn cut_clipboard_context(
        &mut self,
        index: usize,
        kind: ClipboardFenceKind,
    ) -> Result<(), ReceiveError> {
        let context = self.clipboard_context(index);
        let cut = stamp()?;
        let Some(text) = self
            .seats
            .get_mut(index)
            .and_then(|seat| seat.clipboard.as_mut())
        else {
            return Ok(());
        };
        text.context = context;
        text.cut = cut;
        text.sealed = false;
        let tag = self
            .clipboard_tag(index, kind)
            .ok_or(ReceiveError::InvalidPhase)?;
        self.request_clipboard_fence(tag)?;
        self.publish_seat(index)
    }

    pub(super) fn refresh_clipboard_contexts(&mut self) -> Result<(), ReceiveError> {
        if self.clipboard.is_none() {
            return Ok(());
        }
        for index in 0..self.seats.len() {
            let context = self.clipboard_context(index);
            let changed = self.seats[index]
                .clipboard
                .as_ref()
                .is_some_and(|text| text.context != context);
            if changed {
                self.cut_clipboard_context(index, ClipboardFenceKind::Seal)?;
            }
        }
        Ok(())
    }

    fn request_clipboard_fence(&mut self, tag: ClipboardFenceTag) -> Result<(), ReceiveError> {
        let text = self
            .seats
            .iter_mut()
            .filter_map(|seat| seat.clipboard.as_mut())
            .find(|text| text.seat == tag.seat)
            .ok_or(ReceiveError::InvalidPhase)?;
        text.need = Some(tag);
        Ok(())
    }

    fn next_arm_tag(&mut self, now: Instant) -> Result<Option<SyncTag>, ReceiveError> {
        let count = self.endpoints.len();
        for offset in 0..count {
            let index = (self.next_arm + offset) % count;
            let endpoint = &mut self.endpoints[index];
            if endpoint.retiring {
                continue;
            }
            match endpoint.coordinator.next_sync(now) {
                Ok(Some(request)) => {
                    let tag = SyncTag {
                        phase: SyncPhase::Arm(request.epoch),
                        registration: endpoint.registration,
                        clipboard_deadline: None,
                    };
                    self.next_arm = (index + 1) % count;
                    return Ok(Some(tag));
                }
                Ok(None) => {}
                Err(error) if self.owner.is_some() => {
                    endpoint
                        .control
                        .header()
                        .request_close((error != ReceiveError::Closed).then_some(error));
                    endpoint.retiring = true;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    fn next_clipboard_tag(&mut self) -> Option<SyncTag> {
        let count = self.seats.len();
        for offset in 0..count {
            let index = (self.next_clipboard + offset) % count;
            let Some(tag) = self.seats[index]
                .clipboard
                .as_ref()
                .and_then(|text| text.need)
            else {
                continue;
            };
            if let Some((registration, epoch)) = tag.endpoint {
                let Some(endpoint) = self.endpoints.iter().find(|endpoint| {
                    !endpoint.retiring && endpoint.registration == Some(registration)
                }) else {
                    continue;
                };
                let header = endpoint.control.header();
                if header.epoch != epoch || header.status != super::super::ReceiveStatus::Ready {
                    continue;
                }
            }
            self.seats[index]
                .clipboard
                .as_mut()
                .expect("text fence seat")
                .need = None;
            self.next_clipboard = (index + 1) % count;
            return Some(SyncTag {
                phase: SyncPhase::Clipboard(tag),
                registration: None,
                clipboard_deadline: None,
            });
        }
        None
    }

    pub(super) fn next_sync_tag(&mut self, now: Instant) -> Result<Option<SyncTag>, ReceiveError> {
        if self.prefer_clipboard {
            if let Some(tag) = self.next_clipboard_tag() {
                self.prefer_clipboard = false;
                return Ok(Some(tag));
            }
            let tag = self.next_arm_tag(now)?;
            if tag.is_some() {
                self.prefer_clipboard = true;
            }
            Ok(tag)
        } else {
            if let Some(tag) = self.next_arm_tag(now)? {
                self.prefer_clipboard = true;
                return Ok(Some(tag));
            }
            let tag = self.next_clipboard_tag();
            if tag.is_some() {
                self.prefer_clipboard = false;
            }
            Ok(tag)
        }
    }

    pub(super) fn complete_clipboard_fence(
        &mut self,
        tag: ClipboardFenceTag,
        deadline: Instant,
        now: Instant,
    ) -> Result<(), ReceiveError> {
        // Even a logical tombstone retains its original physical callback clock.
        if now.max(Instant::now()) >= deadline {
            return Err(ReceiveError::Timeout);
        }
        let Some(index) = self.seats.iter().position(|seat| {
            seat.clipboard
                .as_ref()
                .is_some_and(|text| text.seat == tag.seat)
        }) else {
            return Ok(());
        };
        if self.clipboard_tag(index, tag.kind) != Some(tag) {
            return Ok(());
        }
        if tag.kind == ClipboardFenceKind::Quarantine {
            let text = self.seats[index]
                .clipboard
                .as_mut()
                .expect("matched clipboard seat");
            if text.focus.is_some() {
                return Ok(());
            }
            let retired = text.selection.take();
            text.quarantined = false;
            self.publish_seat(index)?;
            if let Some((object, _)) = retired {
                if let Some(index) = self
                    .offers
                    .iter()
                    .position(|offer| offer.proxy.id() == object)
                {
                    self.remove_offer(index);
                }
            }
            return Ok(());
        }
        let Some((registration, epoch)) = tag.endpoint else {
            return Ok(());
        };
        let Some(endpoint) = self
            .endpoints
            .iter()
            .position(|endpoint| !endpoint.retiring && endpoint.registration == Some(registration))
        else {
            return Ok(());
        };
        #[cfg(test)]
        if let Some((entered, release)) = self.before_clipboard_seal.take() {
            entered.send(deadline).map_err(|_| ReceiveError::Io)?;
            release
                .recv_timeout(std::time::Duration::from_secs(3))
                .map_err(|_| ReceiveError::Io)?;
        }
        let control = &self.endpoints[endpoint].control;
        let result = self
            .clipboard
            .as_mut()
            .ok_or(ReceiveError::InvalidPhase)?
            .seal_endpoint(
                control,
                registration,
                TextFocusSeal {
                    seat: tag.seat,
                    keyboard_generation: tag.keyboard_generation,
                    focus_generation: tag.focus_generation,
                    registration,
                    epoch,
                    fence_deadline: Some(deadline),
                },
            );
        match result {
            Ok(()) => {
                self.seats[index]
                    .clipboard
                    .as_mut()
                    .expect("matched clipboard seat")
                    .sealed = true;
                #[cfg(test)]
                if let Some(observed) = self.clipboard_observed.as_ref() {
                    observed.clipboard_seals.fetch_add(1, Ordering::SeqCst);
                }
                self.notify();
                Ok(())
            }
            Err(ClipboardError::Timeout) => Err(ReceiveError::Timeout),
            Err(
                ClipboardError::Closed
                | ClipboardError::Cancelled
                | ClipboardError::StaleRequest
                | ClipboardError::Unavailable,
            ) => Ok(()),
            Err(_) => Err(ReceiveError::InvalidPhase),
        }
    }

    fn selection_context_pending(&self, index: usize) -> bool {
        let Some(text) = self.seats[index].clipboard.as_ref() else {
            return false;
        };
        text.need.is_some()
            || (!text.sealed && text.context.is_some())
            || self
                .pending
                .as_ref()
                .is_some_and(|(_, pending)| match pending.phase {
                    SyncPhase::Clipboard(tag) => tag.seat == text.seat,
                    _ => false,
                })
    }

    pub(super) fn select_text(
        &mut self,
        device: &wl_data_device::WlDataDevice,
        proxy: Option<wl_data_offer::WlDataOffer>,
    ) {
        let Some(seat) = self.seat_for_device(&device.id()) else {
            return;
        };
        if self.seats[seat].clipboard.is_none() {
            return;
        }
        let selected = if let Some(proxy) = proxy {
            let Some(index) = self.offer_for_device(&proxy.id(), &device.id()) else {
                return;
            };
            match self.offers[index].role {
                OfferRole::Pending => {
                    let id = match stamp() {
                        Ok(id) => id,
                        Err(error) => {
                            self.fail(error);
                            return;
                        }
                    };
                    self.offers[index].role = OfferRole::Selection(id);
                    Some((proxy.id(), id))
                }
                OfferRole::Selection(id)
                    if self.seats[seat]
                        .clipboard
                        .as_ref()
                        .and_then(|text| text.selection.as_ref())
                        .is_some_and(|(object, stamp)| *object == proxy.id() && *stamp == id) =>
                {
                    return
                }
                _ => return,
            }
        } else {
            None
        };
        let pending = self.selection_context_pending(seat);
        let text = self.seats[seat].clipboard.as_mut().expect("text seat");
        let retired = std::mem::replace(&mut text.selection, selected);
        text.quarantined = text.focus.is_none() && text.quarantined;
        let update = if pending {
            let kind = if text.focus.is_none() {
                ClipboardFenceKind::Quarantine
            } else {
                ClipboardFenceKind::Seal
            };
            self.cut_clipboard_context(seat, kind)
        } else {
            self.publish_seat(seat)
        };
        if let Err(error) = update {
            self.fail(error);
        }
        // Snapshot/cancellation precedes destruction of the previous offer.
        if let Some((object, _)) = retired {
            if let Some(index) = self
                .offers
                .iter()
                .position(|offer| offer.proxy.id() == object)
            {
                self.remove_offer(index);
            }
        }
    }

    pub(super) fn clipboard_mime_changed(&mut self, object: &ObjectId) {
        let seat = self.seats.iter().position(|seat| {
            seat.clipboard
                .as_ref()
                .and_then(|text| text.selection.as_ref())
                .is_some_and(|(current, _)| current == object)
        });
        if let Some(seat) = seat {
            if let Err(error) = self.publish_seat(seat) {
                self.fail(error);
            }
        }
    }

    pub(super) fn clipboard_offer_removed(&mut self, object: &ObjectId) {
        for index in 0..self.seats.len() {
            let Some(text) = self.seats[index].clipboard.as_mut() else {
                continue;
            };
            if text
                .selection
                .as_ref()
                .is_some_and(|(current, _)| current == object)
            {
                text.selection = None;
                text.quarantined = false;
                let _ = self.publish_seat(index);
            }
        }
    }

    fn keyboard_capability(
        &mut self,
        seat: usize,
        present: bool,
        handle: &QueueHandle<Self>,
    ) -> Result<(), ReceiveError> {
        let Some(text) = self.seats[seat].clipboard.as_ref() else {
            return Ok(());
        };
        if present == text.keyboard.is_some() {
            return Ok(());
        }
        if present {
            let generation = stamp()?;
            let text = self.seats[seat].clipboard.as_ref().expect("text seat");
            let tag = KeyboardTag {
                seat: text.seat,
                keyboard_generation: generation,
            };
            let proxy = self.seats[seat]
                .proxy
                .as_ref()
                .ok_or(ReceiveError::InvalidPhase)?;
            let keyboard = proxy
                .send_constructor(
                    wl_seat::Request::GetKeyboard {},
                    handle.make_data::<wl_keyboard::WlKeyboard, _>(tag),
                )
                .map_err(|_| ReceiveError::Io)?;
            let text = self.seats[seat].clipboard.as_mut().expect("text seat");
            text.keyboard_generation = generation;
            text.keyboard = Some(keyboard);
            self.publish_seat(seat)
        } else {
            let keyboard_generation = stamp()?;
            let focus_generation = stamp()?;
            let cut = stamp()?;
            let text = self.seats[seat].clipboard.as_mut().expect("text seat");
            let keyboard = text.keyboard.take();
            let selection = text.selection.take();
            text.keyboard_generation = keyboard_generation;
            text.focus_generation = focus_generation;
            text.focus = None;
            text.quarantined = false;
            text.context = None;
            text.sealed = false;
            text.need = None;
            text.cut = cut;
            self.publish_seat(seat)?;
            if let Some((object, _)) = selection {
                if let Some(index) = self
                    .offers
                    .iter()
                    .position(|offer| offer.proxy.id() == object)
                {
                    self.remove_offer(index);
                }
            }
            if let Some(keyboard) = keyboard {
                if keyboard.is_alive() {
                    keyboard
                        .send_request(wl_keyboard::Request::Release {})
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            Ok(())
        }
    }

    fn keyboard_focus(
        &mut self,
        seat: usize,
        surface: ObjectId,
        entered: bool,
        live: bool,
    ) -> Result<(), ReceiveError> {
        let text = self.seats[seat]
            .clipboard
            .as_ref()
            .ok_or(ReceiveError::InvalidPhase)?;
        // Destroying the focused surface can clear focus without a Leave, or send
        // one naming a dead surface: treat a new Enter as an implicit Leave.
        if entered {
            if let Some(focus) = text.focus.clone() {
                self.keyboard_focus(seat, focus, false, true)?;
            }
        } else if text.focus.is_none() || (live && text.focus.as_ref() != Some(&surface)) {
            return Ok(());
        }
        let generation = stamp()?;
        let text = self.seats[seat].clipboard.as_mut().expect("text seat");
        text.focus_generation = generation;
        text.focus = entered.then_some(surface);
        text.quarantined = !entered && text.selection.is_some();
        self.cut_clipboard_context(
            seat,
            if entered {
                ClipboardFenceKind::Seal
            } else {
                ClipboardFenceKind::Quarantine
            },
        )
    }

    pub(super) fn reconcile_clipboard(&mut self, now: Instant) {
        if let Some(worker) = self.clipboard.as_mut() {
            worker.reconcile(now.max(Instant::now()));
            if self.pipe.as_ref().is_some_and(|(tag, _)| match tag {
                ReadTag::Text(id) => !worker.is_reading(*id),
                ReadTag::Drop(_) => false,
            }) {
                drop(self.pipe.take());
            }
        }
    }

    pub(super) fn start_text_receive(&mut self) {
        self.reconcile_clipboard(self.now.max(Instant::now()));
        let Some((id, registration)) = self
            .clipboard
            .as_ref()
            .and_then(ClipboardWorker::pending_request)
        else {
            return;
        };
        let Some(endpoint) = self
            .endpoints
            .iter()
            .position(|endpoint| !endpoint.retiring && endpoint.registration == Some(registration))
        else {
            return;
        };
        let claim = match self.clipboard.as_mut().expect("text worker").claim_receive(
            id,
            &self.endpoints[endpoint].control,
            registration,
            self.now.max(Instant::now()),
        ) {
            Ok(claim) => claim,
            Err(_) => return,
        };
        let proxy = self
            .seats
            .iter()
            .find(|seat| {
                seat.clipboard
                    .as_ref()
                    .is_some_and(|text| text.seat == claim.seat)
            })
            .and_then(|seat| {
                self.offers.iter().find(|offer| {
                    offer.role == OfferRole::Selection(claim.selection)
                        && seat
                            .proxy
                            .as_ref()
                            .is_some_and(|proxy| offer.seat == proxy.id())
                })
            })
            .map(|offer| offer.proxy.clone());
        let Some(proxy) = proxy else {
            self.clipboard
                .as_mut()
                .expect("text worker")
                .fail_receive(id, ClipboardError::Io);
            return;
        };
        let (pipe, writer) = match ReceivePipe::bounded(claim.max_text_bytes, claim.deadline) {
            Ok(pair) => pair,
            Err(ReceiveError::Io) => {
                self.clipboard
                    .as_mut()
                    .expect("text worker")
                    .fail_receive(id, ClipboardError::Io);
                return;
            }
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        let mime = match claim.mime {
            TextMime::Utf8 => UTF8,
            TextMime::Plain => PLAIN,
        };
        let result = proxy.send_request(wl_data_offer::Request::Receive {
            mime_type: mime.into(),
            fd: writer.as_fd(),
        });
        drop(writer);
        if result.is_err() {
            drop(pipe);
            self.clipboard
                .as_mut()
                .expect("text worker")
                .fail_receive(id, ClipboardError::Io);
        } else {
            self.pipe = Some((ReadTag::Text(claim.request), pipe));
        }
    }

    pub(super) fn advance_text_transfer(&mut self, id: TextRequestId) {
        self.reconcile_clipboard(self.now.max(Instant::now()));
        if !self
            .pipe
            .as_ref()
            .is_some_and(|(tag, _)| *tag == ReadTag::Text(id))
        {
            return;
        }
        let result = self
            .pipe
            .as_mut()
            .expect("exact text pipe")
            .1
            .read_available(self.now.max(Instant::now()));
        self.now = self.now.max(Instant::now());
        self.reconcile_clipboard(self.now);
        if !self
            .clipboard
            .as_ref()
            .is_some_and(|worker| worker.is_reading(id))
        {
            return;
        }
        match result {
            Ok(None) => {}
            Err(error) => {
                drop(self.pipe.take());
                let error = match error {
                    ReceiveError::Timeout => ClipboardError::Timeout,
                    ReceiveError::TooLarge => ClipboardError::TooLarge,
                    _ => ClipboardError::Io,
                };
                self.clipboard
                    .as_mut()
                    .expect("text worker")
                    .fail_receive(id, error);
            }
            Ok(Some(bytes)) => {
                drop(self.pipe.take());
                let decoded = String::from_utf8(bytes);
                self.now = self.now.max(Instant::now());
                self.reconcile_clipboard(self.now);
                let worker = self.clipboard.as_mut().expect("text worker");
                match decoded {
                    Ok(text) => {
                        let current = worker.current_request();
                        if let Some((current, registration)) =
                            current.filter(|(current, _)| *current == id)
                        {
                            if let Some(endpoint) = self.endpoints.iter().find(|endpoint| {
                                !endpoint.retiring && endpoint.registration == Some(registration)
                            }) {
                                let _ = worker.stage_validated(
                                    current,
                                    &endpoint.control,
                                    registration,
                                    text,
                                    self.now,
                                );
                            }
                        }
                    }
                    Err(_) => {
                        worker.fail_receive(id, ClipboardError::InvalidText);
                    }
                }
            }
        }
    }

    pub(super) fn finish_clipboard_retirements(&mut self) -> Result<(), ReceiveError> {
        let Some(worker) = self.clipboard.as_mut() else {
            return Ok(());
        };
        worker.reconcile(self.now.max(Instant::now()));
        let Some((id, registration)) = worker.current_request() else {
            return Ok(());
        };
        if self
            .pipe
            .as_ref()
            .is_some_and(|(tag, _)| *tag == ReadTag::Text(id))
        {
            return Ok(());
        }
        let endpoint = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.registration == Some(registration));
        match worker.finish_after_watcher_removed(
            id,
            endpoint.map(|endpoint| &endpoint.control),
            registration,
            Instant::now(),
        ) {
            Ok(true) => {
                self.notify();
                Ok(())
            }
            Ok(false) | Err(ClipboardError::StaleRequest) => Ok(()),
            Err(_) => Err(ReceiveError::InvalidPhase),
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if state.failure.is_some() || state.clipboard.is_none() {
            return;
        }
        let Some(index) = state.seats.iter().position(|seat| {
            seat.proxy
                .as_ref()
                .is_some_and(|seat| seat.id() == proxy.id())
        }) else {
            return;
        };
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let present = matches!(capabilities, WEnum::Value(capabilities) if capabilities.contains(wl_seat::Capability::Keyboard));
            if let Err(error) = state.keyboard_capability(index, present, handle) {
                state.fail(error);
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, KeyboardTag> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        tag: &KeyboardTag,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if state.failure.is_some() {
            return;
        }
        let Some(index) = state.seats.iter().position(|seat| {
            seat.clipboard.as_ref().is_some_and(|text| {
                text.seat == tag.seat
                    && text.keyboard_generation == tag.keyboard_generation
                    && text
                        .keyboard
                        .as_ref()
                        .is_some_and(|keyboard| keyboard.id() == proxy.id())
            })
        }) else {
            return;
        };
        let result = match event {
            wl_keyboard::Event::Keymap { fd, .. } => {
                drop(fd);
                Ok(())
            }
            wl_keyboard::Event::Enter { surface, .. } => {
                #[cfg(test)]
                if let Some(observed) = state.clipboard_observed.as_ref() {
                    observed.keyboard_enter_dispatch.store(
                        observed.guest_dispatches.load(Ordering::SeqCst),
                        Ordering::SeqCst,
                    );
                }
                state.keyboard_focus(index, surface.id(), true, true)
            }
            wl_keyboard::Event::Leave { surface, .. } => {
                state.keyboard_focus(index, surface.id(), false, surface.is_alive())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            state.fail(error);
        }
    }
}
