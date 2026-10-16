//! Legal text opt-in for the existing private Wayland protocol server.

use super::offers::OfferRequest;
use super::*;
use wayland_server::backend::ObjectId;

pub(super) struct KeyboardRecord {
    seat: ObjectId,
    resource: wl_keyboard::WlKeyboard,
    focused: Option<ObjectId>,
    enabled: bool,
}

impl ServerState {
    pub(super) fn create_keyboard(
        &mut self,
        seat: &wl_seat::WlSeat,
        resource: New<wl_keyboard::WlKeyboard>,
        init: &mut DataInit<'_, Self>,
    ) {
        assert!(
            self.text_keyboards,
            "DnD-only mode must not bind a keyboard"
        );
        let seat_index = self
            .seat_resources
            .iter()
            .position(|actual| actual.id() == seat.id())
            .unwrap();
        assert!(self.keyboard_capabilities[seat_index]);
        assert!(!self
            .keyboards
            .iter()
            .any(|keyboard| keyboard.seat == seat.id() && keyboard.enabled));
        assert!(self.keyboards.len() < 4);
        let keyboard = init.init(resource, ());
        assert_eq!(keyboard.version(), 5);
        keyboard.repeat_info(0, 0);
        self.keyboards.push(KeyboardRecord {
            seat: seat.id(),
            resource: keyboard,
            focused: None,
            enabled: true,
        });
    }

    fn keyboard_index(&self, device: usize) -> usize {
        let seat = &self.device_seats[device];
        self.keyboards
            .iter()
            .position(|keyboard| {
                keyboard.seat == *seat && keyboard.enabled && keyboard.resource.is_alive()
            })
            .expect("this exact data device's seat has a live keyboard")
    }

    fn keyboard_serial(&mut self) -> u32 {
        self.keyboard_serial = self.keyboard_serial.checked_add(1).unwrap();
        self.keyboard_serial
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        resource: &wl_keyboard::WlKeyboard,
        request: wl_keyboard::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        assert!(matches!(request, wl_keyboard::Request::Release));
        let index = state
            .keyboards
            .iter()
            .position(|keyboard| keyboard.resource.id() == resource.id())
            .unwrap();
        state.keyboards.remove(index);
        state.released_keyboards += 1;
    }
}

impl Fixture {
    pub(super) fn new_text(manager: Option<u32>, seats: &[u32]) -> Self {
        let mut fixture = Self::new(manager, seats);
        fixture.server.text_keyboards = true;
        fixture
    }

    pub(super) fn has_keyboard(&self, device: usize) -> bool {
        let Some(seat) = self.server.device_seats.get(device) else {
            return false;
        };
        self.server.keyboards.iter().any(|keyboard| {
            keyboard.seat == *seat && keyboard.enabled && keyboard.resource.is_alive()
        })
    }

    pub(super) fn keyboard_enter(&mut self, device: usize, surface: usize) {
        let index = self.server.keyboard_index(device);
        let serial = self.server.keyboard_serial();
        let surface = self.server.surfaces[surface].clone();
        let keyboard = &mut self.server.keyboards[index];
        assert!(
            keyboard.focused.is_none(),
            "Enter requires a preceding Leave"
        );
        keyboard.focused = Some(surface.id());
        keyboard.resource.enter(serial, &surface, Vec::new());
        keyboard.resource.modifiers(serial, 0, 0, 0, 0);
    }

    pub(super) fn keyboard_leave(&mut self, device: usize, surface: usize) {
        let index = self.server.keyboard_index(device);
        let serial = self.server.keyboard_serial();
        let surface = self.server.surfaces[surface].clone();
        let keyboard = &mut self.server.keyboards[index];
        assert_eq!(keyboard.focused, Some(surface.id()));
        keyboard.focused = None;
        keyboard.resource.leave(serial, &surface);
    }

    pub(super) fn keyboard_capability(&mut self, device: usize, present: bool) {
        let seat = self.server.device_seats[device].clone();
        let index = self
            .server
            .seat_resources
            .iter()
            .position(|actual| actual.id() == seat)
            .unwrap();
        assert_ne!(self.server.keyboard_capabilities[index], present);
        self.server.keyboard_capabilities[index] = present;
        if !present {
            for keyboard in &mut self.server.keyboards {
                if keyboard.seat == seat {
                    keyboard.enabled = false;
                    keyboard.focused = None;
                }
            }
        }
        let capabilities = if present {
            wl_seat::Capability::Pointer | wl_seat::Capability::Keyboard
        } else {
            wl_seat::Capability::Pointer
        };
        self.server.seat_resources[index].capabilities(capabilities);
    }

    pub(super) fn text_selection(
        &self,
        device: usize,
        key: usize,
        utf8: bool,
        plain: bool,
    ) -> wayland_server::protocol::wl_data_offer::WlDataOffer {
        let offer = self.introduce_offer(device, key);
        if utf8 {
            offer.offer("text/plain;charset=utf-8".to_owned());
        }
        if plain {
            offer.offer("text/plain".to_owned());
        }
        self.server.device_resources[device].selection(Some(&offer));
        offer
    }

    pub(super) fn selection_none(&self, device: usize) {
        self.server.device_resources[device].selection(None);
    }

    pub(super) fn text_receive_count(&self, key: usize, expected_mime: &str) -> usize {
        assert!(matches!(
            expected_mime,
            "text/plain;charset=utf-8" | "text/plain"
        ));
        self.server
            .requests
            .iter()
            .filter(|request| match request {
                OfferRequest::Receive(actual, mime, _)
                | OfferRequest::ReceiveTaken(actual, mime)
                    if *actual == key =>
                {
                    assert_eq!(mime, expected_mime, "unexpected text Receive MIME");
                    true
                }
                _ => false,
            })
            .count()
    }
}
