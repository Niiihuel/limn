//! Bounded protocol identities; the coordinator alone owns drop phases.

use std::os::fd::AsFd;

use sctk::reexports::client::{protocol::wl_surface::WlSurface, WEnum};

use super::*;
use crate::{receive::coordinator::Phase, SurfacePoint};

const MIME: &str = "text/uri-list";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OfferRole {
    Pending,
    Drop(OfferId),
    Selection(ClipboardStamp),
}

pub(super) struct Offer {
    pub(super) proxy: wl_data_offer::WlDataOffer,
    pub(super) device: ObjectId,
    pub(super) seat: ObjectId,
    has_uri: bool,
    pub(super) utf8: bool,
    pub(super) plain: bool,
    selected_copy: bool,
    pub(super) role: OfferRole,
}

impl Offer {
    pub(super) fn drop_id(&self) -> Option<OfferId> {
        if let OfferRole::Drop(id) = self.role {
            Some(id)
        } else {
            None
        }
    }
}

impl State {
    pub(super) fn offer_proxy(&self, id: OfferId) -> Option<wl_data_offer::WlDataOffer> {
        self.offers
            .iter()
            .find(|offer| offer.drop_id() == Some(id))
            .map(|offer| offer.proxy.clone())
    }

    pub(super) fn seat_for_device(&self, device: &ObjectId) -> Option<usize> {
        self.seats.iter().position(|seat| {
            seat.device
                .as_ref()
                .is_some_and(|proxy| proxy.id() == *device)
        })
    }

    pub(super) fn offer_for_device(&self, object: &ObjectId, device: &ObjectId) -> Option<usize> {
        let seat = self
            .seats
            .get(self.seat_for_device(device)?)?
            .proxy
            .as_ref()?
            .id();
        self.offers.iter().position(|offer| {
            offer.proxy.id() == *object && offer.device == *device && offer.seat == seat
        })
    }

    fn destroy_proxy(proxy: &wl_data_offer::WlDataOffer) {
        if proxy.is_alive() {
            // A concurrent owner can already have destroyed this exact proxy.
            let _ = proxy.send_request(wl_data_offer::Request::Destroy {});
        }
    }

    pub(super) fn remove_offer(&mut self, index: usize) {
        let offer = self.offers.remove(index);
        let object = offer.proxy.id();
        self.clipboard_offer_removed(&object);
        for seat in &mut self.seats {
            if seat.hover.as_ref() == Some(&object) {
                seat.hover = None;
            }
        }
        if self
            .pipe
            .as_ref()
            .is_some_and(|(id, _)| matches!((*id, offer.role), (ReadTag::Drop(actual), OfferRole::Drop(expected)) if actual == expected))
        {
            self.pipe = None;
        }
        Self::destroy_proxy(&offer.proxy);
    }

    pub(super) fn cleanup_offers(&mut self) {
        let closing = self.control.header().is_closing();
        let mut index = 0;
        while index < self.offers.len() {
            let retired = self.offers[index].drop_id().is_some_and(|id| {
                self.endpoint_for_offer(id).is_none_or(|endpoint| {
                    self.endpoints[endpoint].retiring
                        || matches!(self.offer_phase(id), Some(Phase::TerminalQueued))
                })
            });
            if closing || retired {
                self.remove_offer(index);
            } else {
                index += 1;
            }
        }
        self.release_unwatched_drop();
    }

    pub(super) fn retire_device_offers(&mut self, device: &ObjectId) {
        let mut index = 0;
        while index < self.offers.len() {
            if self.offers[index].device == *device {
                if let Some(id) = self.offers[index].drop_id() {
                    if let Some(endpoint) = self.endpoint_for_offer(id) {
                        let _ = self.endpoints[endpoint].coordinator.cancel(id);
                    }
                }
                self.remove_offer(index);
            } else {
                index += 1;
            }
        }
    }

    fn introduce_offer(
        &mut self,
        device: &wl_data_device::WlDataDevice,
        proxy: wl_data_offer::WlDataOffer,
    ) {
        let Some(seat_index) = self.seat_for_device(&device.id()) else {
            Self::destroy_proxy(&proxy);
            return;
        };
        if (self.clipboard.is_none()
            && !self
                .endpoints
                .iter()
                .any(|endpoint| endpoint.surface.is_some() && !endpoint.retiring))
            || self.failure.is_some()
        {
            Self::destroy_proxy(&proxy);
            return;
        }
        if self.offers.len() == 8 {
            Self::destroy_proxy(&proxy);
            self.fail(ReceiveError::ResourceLimit);
            return;
        }
        let Some(seat) = self.seats[seat_index].proxy.as_ref() else {
            Self::destroy_proxy(&proxy);
            return;
        };
        self.offers.push(Offer {
            proxy,
            device: device.id(),
            seat: seat.id(),
            has_uri: false,
            utf8: false,
            plain: false,
            selected_copy: false,
            role: OfferRole::Pending,
        });
    }

    fn reject_offer(&mut self, index: usize, serial: u32) {
        let _ = self.offers[index]
            .proxy
            .send_request(wl_data_offer::Request::Accept {
                serial,
                mime_type: None,
            });
        self.remove_offer(index);
    }

    fn enter_offer(
        &mut self,
        device: &wl_data_device::WlDataDevice,
        serial: u32,
        surface: &WlSurface,
        point: SurfacePoint,
        proxy: Option<wl_data_offer::WlDataOffer>,
    ) {
        let Some(proxy) = proxy else {
            return;
        };
        let Some(index) = self.offer_for_device(&proxy.id(), &device.id()) else {
            return;
        };
        if self.offers[index].role != OfferRole::Pending {
            return;
        }
        let endpoint = self.endpoints.iter().position(|endpoint| {
            !endpoint.retiring && endpoint.surface.as_ref() == Some(&surface.id())
        });
        if endpoint.is_none()
            || self.control.incoming().is_some()
            || !self.offers[index].has_uri
            || self.failure.is_some()
        {
            self.reject_offer(index, serial);
            return;
        }
        let endpoint = endpoint.expect("validated exact registered surface");
        let id = match next_offer_id() {
            Ok(id) => id,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        let registration = self.endpoints[endpoint].registration;
        // Reserve before enter publishes Hover or invokes the caller's wake.
        if self.endpoints[endpoint]
            .control
            .reserve_incoming_drop(registration, id)
            .is_err()
        {
            self.reject_offer(index, serial);
            return;
        }
        self.offers[index].role = OfferRole::Drop(id);
        match self.endpoints[endpoint]
            .coordinator
            .enter(id, point, proxy.version(), true)
        {
            Ok(()) => {}
            Err(_) => {
                self.offers[index].role = OfferRole::Pending;
                self.reject_offer(index, serial);
                // enter failed before a pipe or watcher could exist.
                self.control
                    .release_incoming_after_cleanup(IncomingLease::Drop { registration, id });
                return;
            }
        }
        if self.endpoints[endpoint]
            .coordinator
            .action(id, self.offers[index].selected_copy)
            .is_err()
        {
            self.cleanup_offers();
            return;
        }
        if let Some(seat) = self.seat_for_device(&device.id()) {
            self.seats[seat].hover = Some(proxy.id());
        }
        let copy = wl_data_device_manager::DndAction::Copy;
        let result = proxy
            .send_request(wl_data_offer::Request::Accept {
                serial,
                mime_type: Some(MIME.into()),
            })
            .and_then(|()| {
                proxy.send_request(wl_data_offer::Request::SetActions {
                    dnd_actions: WEnum::Value(copy),
                    preferred_action: WEnum::Value(copy),
                })
            });
        if result.is_err() {
            let _ = self.endpoints[endpoint].coordinator.cancel(id);
            self.cleanup_offers();
        }
    }

    fn hovered_offer(&self, device: &ObjectId) -> Option<(usize, OfferId)> {
        let seat = self.seat_for_device(device)?;
        let object = self.seats[seat].hover.as_ref()?;
        let index = self.offer_for_device(object, device)?;
        Some((index, self.offers[index].drop_id()?))
    }

    fn leave_offer(&mut self, device: &ObjectId) {
        if let Some((_, id)) = self.hovered_offer(device) {
            if let Some(endpoint) = self.endpoint_for_offer(id) {
                let _ = self.endpoints[endpoint].coordinator.leave(id);
            }
        }
        if let Some(seat) = self.seat_for_device(device) {
            self.seats[seat].hover = None;
        }
        self.cleanup_offers();
    }

    fn start_offer(&mut self, device: &ObjectId) {
        let Some((_, id)) = self.hovered_offer(device) else {
            return;
        };
        let Some(endpoint) = self.endpoint_for_offer(id) else {
            return;
        };
        if let Some(seat) = self.seat_for_device(device) {
            self.seats[seat].hover = None;
        }
        self.now = self.now.max(Instant::now());
        if self.endpoints[endpoint]
            .coordinator
            .start(id, self.now)
            .is_err()
        {
            self.cleanup_offers();
            return;
        }
        self.now = self.now.max(Instant::now());
        let deadline = match self.endpoints[endpoint]
            .coordinator
            .reading_deadline(id, self.now)
        {
            Ok(deadline) => deadline,
            Err(_) => {
                self.cleanup_offers();
                return;
            }
        };
        let (pipe, writer) = match ReceivePipe::new(id, self.limits.max_list_bytes, deadline) {
            Ok(pair) => pair,
            Err(error) => {
                let _ = self.endpoints[endpoint].coordinator.fail(id, error);
                self.cleanup_offers();
                return;
            }
        };
        self.drop_reader = Some((id, Arc::downgrade(&pipe.readiness_fd())));
        let proxy = self.offer_proxy(id);
        self.now = self.now.max(Instant::now());
        if self.endpoints[endpoint]
            .coordinator
            .reading_deadline(id, self.now)
            .is_err()
        {
            drop(writer);
            drop(pipe);
            self.cleanup_offers();
            return;
        }
        let result = proxy.ok_or(ReceiveError::Io).and_then(|proxy| {
            proxy
                .send_request(wl_data_offer::Request::Receive {
                    mime_type: MIME.into(),
                    fd: writer.as_fd(),
                })
                .map_err(|_| ReceiveError::Io)
        });
        // EOF requires closing this copy even if the typed SDK request failed.
        drop(writer);
        if let Err(error) = result {
            drop(pipe);
            let _ = self.endpoints[endpoint].coordinator.fail(id, error);
            self.cleanup_offers();
        } else {
            self.pipe = Some((ReadTag::Drop(id), pipe));
        }
    }

    pub(super) fn consume_decision(&mut self) {
        for endpoint in 0..self.endpoints.len() {
            self.now = self.now.max(Instant::now());
            let _ = self.reconcile_endpoints(self.now);
            self.cleanup_offers();
            let decision = self.endpoints[endpoint].control.take_decision();
            self.now = self.now.max(Instant::now());
            let _ = self.reconcile_endpoints(self.now);
            self.cleanup_offers();
            let Some((id, decision)) = decision else {
                continue;
            };
            let proxy = self.offer_proxy(id);
            self.now = self.now.max(Instant::now());
            let _ = self.endpoints[endpoint]
                .coordinator
                .decide(id, decision, self.now, |_| {
                    proxy
                        .as_ref()
                        .ok_or(ReceiveError::Io)?
                        .send_request(wl_data_offer::Request::Finish {})
                        .map_err(|_| ReceiveError::Io)
                });
            self.cleanup_offers();
        }
    }

    pub(super) fn advance_transfer(&mut self) {
        let Some((id, _)) = self.pipe.as_ref() else {
            return;
        };
        let id = match *id {
            ReadTag::Drop(id) => id,
            ReadTag::Text(id) => {
                self.advance_text_transfer(id);
                return;
            }
        };
        let Some(endpoint) = self.endpoint_for_offer(id) else {
            drop(self.pipe.take());
            return;
        };
        self.now = self.now.max(Instant::now());
        if self.endpoints[endpoint]
            .coordinator
            .reading_deadline(id, self.now)
            .is_err()
        {
            self.cleanup_offers();
            return;
        }
        let result = match self.pipe.as_mut() {
            Some((_, pipe)) => pipe.read_available(self.now),
            None => return,
        };
        self.now = self.now.max(Instant::now());
        if self.endpoints[endpoint]
            .coordinator
            .reading_deadline(id, self.now)
            .is_err()
        {
            self.cleanup_offers();
            return;
        }
        match result {
            Ok(None) => {}
            Err(error) => {
                drop(self.pipe.take());
                let _ = self.endpoints[endpoint].coordinator.fail(id, error);
                self.cleanup_offers();
            }
            Ok(Some(bytes)) => {
                drop(self.pipe.take());
                // Neither the decoder nor its owned buffers run under control/mailbox guards.
                let decoded = crate::uri_list::decode_received(&bytes, &self.limits);
                self.now = self.now.max(Instant::now());
                if self.endpoints[endpoint]
                    .coordinator
                    .reading_deadline(id, self.now)
                    .is_err()
                {
                    self.cleanup_offers();
                    return;
                }
                match decoded {
                    Ok(paths) => {
                        let _ = self.endpoints[endpoint]
                            .coordinator
                            .complete_validated(id, paths, self.now);
                    }
                    Err(error) => {
                        let _ = self.endpoints[endpoint].coordinator.fail(id, error);
                    }
                }
                self.cleanup_offers();
            }
        }
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for State {
    fn event(
        state: &mut Self,
        device: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let _ = state.reconcile_endpoints(state.now.max(Instant::now()));
        state.cleanup_offers();
        match event {
            wl_data_device::Event::DataOffer { id } => state.introduce_offer(device, id),
            wl_data_device::Event::Enter {
                serial,
                surface,
                x,
                y,
                id,
            } => state.enter_offer(device, serial, &surface, SurfacePoint { x, y }, id),
            wl_data_device::Event::Motion { x, y, .. } => {
                if let Some((_, id)) = state.hovered_offer(&device.id()) {
                    if let Some(endpoint) = state.endpoint_for_offer(id) {
                        let _ = state.endpoints[endpoint]
                            .coordinator
                            .motion(id, SurfacePoint { x, y });
                    }
                }
            }
            wl_data_device::Event::Leave => state.leave_offer(&device.id()),
            wl_data_device::Event::Drop => state.start_offer(&device.id()),
            wl_data_device::Event::Selection { id } => {
                if state.clipboard.is_some() {
                    state.select_text(device, id);
                } else if let Some(proxy) = id {
                    if let Some(index) = state.offer_for_device(&proxy.id(), &device.id()) {
                        if state.offers[index].role == OfferRole::Pending {
                            state.remove_offer(index);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    sctk::reexports::client::event_created_child!(State, wl_data_device::WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (wl_data_offer::WlDataOffer, ())
    ]);
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(index) = state
            .offers
            .iter()
            .position(|offer| offer.proxy.id() == proxy.id())
        else {
            return;
        };
        match event {
            wl_data_offer::Event::Offer { mime_type } => {
                state.offers[index].has_uri |= mime_type == MIME;
                state.offers[index].utf8 |= mime_type == clipboard::UTF8;
                state.offers[index].plain |= mime_type == clipboard::PLAIN;
                state.clipboard_mime_changed(&proxy.id());
            }
            wl_data_offer::Event::Action { dnd_action } => {
                let copy = dnd_action == WEnum::Value(wl_data_device_manager::DndAction::Copy);
                state.offers[index].selected_copy = copy;
                if let Some(id) = state.offers[index].drop_id() {
                    if !copy
                        && matches!(
                            state.offer_phase(id),
                            Some(Phase::Reading { .. } | Phase::AwaitingDecision { .. })
                        )
                    {
                        if let Some(endpoint) = state.endpoint_for_offer(id) {
                            let _ = state.endpoints[endpoint].coordinator.cancel(id);
                        }
                        state.cleanup_offers();
                    } else {
                        if let Some(endpoint) = state.endpoint_for_offer(id) {
                            let _ = state.endpoints[endpoint].coordinator.action(id, copy);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
