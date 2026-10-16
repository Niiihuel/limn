//! Startup on a guest queue whose foreign connection is read by its host.

use std::{
    os::fd::OwnedFd,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use sctk::reexports::client::{
    backend::{ObjectId, WaylandError},
    protocol::{
        wl_callback, wl_data_device, wl_data_device_manager, wl_data_offer, wl_display,
        wl_registry, wl_seat, wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

use super::{
    control::ReceiveControl,
    coordinator::Coordinator,
    pipe::ReceivePipe,
    worker_pool::{SurfaceKey, Wake, WorkerPool},
    OfferId, ReceiveEpoch,
};
use crate::{ReceiveError, ReceiveLimits};

#[path = "wayland_offers.rs"]
mod offers;

static NEXT_OFFER_ID: AtomicU64 = AtomicU64::new(1);

fn next_offer_id() -> Result<OfferId, ReceiveError> {
    allocate_offer_id(&NEXT_OFFER_ID)
}

fn allocate_offer_id(counter: &AtomicU64) -> Result<OfferId, ReceiveError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .map(OfferId)
        .map_err(|_| ReceiveError::ResourceLimit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SyncPhase {
    Discovery,
    Arm(ReceiveEpoch),
}

pub(super) struct GuestStartup {
    queue: Option<EventQueue<State>>,
    state: State,
    connection: Option<Connection>,
}

struct Seat {
    name: u32,
    proxy: Option<wl_seat::WlSeat>,
    device: Option<wl_data_device::WlDataDevice>,
    hover: Option<ObjectId>,
}

struct State {
    coordinator: Coordinator,
    pool: Arc<WorkerPool>,
    control: ReceiveControl,
    registry: Option<wl_registry::WlRegistry>,
    manager_name: Option<u32>,
    manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    seats: Vec<Seat>,
    pending: Option<(wl_callback::WlCallback, SyncPhase)>,
    completed: Option<SyncPhase>,
    discovery_epoch: Option<ReceiveEpoch>,
    discovery_complete: bool,
    startup_deadline: Instant,
    first_ready: bool,
    failure: Option<ReceiveError>,
    target_surface: Option<ObjectId>,
    offers: Vec<offers::Offer>,
    pipe: Option<(OfferId, ReceivePipe)>,
    limits: ReceiveLimits,
    now: Instant,
}

impl GuestStartup {
    // Admission validated the limits before reserving/spawning. Keep the state
    // and sole wake outside the catch that installs or dispatches foreign SDK IO.
    pub(super) fn dormant(
        pool: Arc<WorkerPool>,
        control: ReceiveControl,
        limits: ReceiveLimits,
        now: Instant,
        wake: Wake,
    ) -> Self {
        let startup_deadline = now
            .checked_add(limits.startup_timeout)
            .expect("admission validated the startup deadline");
        Self {
            queue: None,
            connection: None,
            state: State {
                coordinator: Coordinator::with_wake(limits, control.clone(), wake),
                pool,
                control,
                registry: None,
                manager_name: None,
                manager: None,
                seats: Vec::with_capacity(4),
                pending: None,
                completed: None,
                discovery_epoch: None,
                discovery_complete: false,
                startup_deadline,
                first_ready: false,
                failure: None,
                target_surface: None,
                offers: Vec::with_capacity(8),
                pipe: None,
                limits,
                now,
            },
        }
    }

    fn install(&mut self, connection: Connection, surface: Option<ObjectId>) {
        self.connection = Some(connection);
        self.state.target_surface = surface;
        self.queue = Some(
            self.connection
                .as_ref()
                .expect("connection was retained before queue creation")
                .new_event_queue(),
        );
    }

    /// # Safety
    /// The retained pool owner and coordinated host reader must keep both raw
    /// handles valid until the guest backend has been cleaned and joined.
    pub(super) unsafe fn install_foreign(&mut self, key: SurfaceKey) -> Result<(), ReceiveError> {
        let backend = unsafe {
            wayland_backend::client::Backend::from_foreign_display(key.display as *mut _)
        };
        self.connection = Some(Connection::from_backend(backend));
        let surface = unsafe { ObjectId::from_ptr(WlSurface::interface(), key.surface as *mut _) }
            .map_err(|_| ReceiveError::Unavailable)?;
        let connection = self.connection.take().expect("foreign backend is retained");
        self.install(connection, Some(surface));
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn new(
        connection: Connection,
        control: ReceiveControl,
        limits: ReceiveLimits,
        now: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, ReceiveError> {
        limits.validate()?;
        let mut guest = Self::dormant(WorkerPool::isolated(), control, limits, now, Box::new(wake));
        guest.install(connection, None);
        Ok(guest)
    }

    #[cfg(test)]
    pub(super) fn for_surface(
        connection: Connection,
        surface: ObjectId,
        control: ReceiveControl,
        limits: ReceiveLimits,
        now: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, ReceiveError> {
        let mut guest = Self::new(connection, control, limits, now, wake)?;
        guest.state.target_surface = Some(surface);
        Ok(guest)
    }

    #[cfg(test)]
    pub(super) fn pump(&mut self, now: Instant) -> Result<(), ReceiveError> {
        let readable = self.state.pipe.as_ref().map(|(id, _)| *id);
        self.pump_readable(now, readable)
    }

    pub(super) fn pump_readable(
        &mut self,
        now: Instant,
        readable: Option<OfferId>,
    ) -> Result<(), ReceiveError> {
        let now = now.max(Instant::now());
        self.state.now = now;
        self.state.check(now)?;
        let connection = self.connection.as_ref().ok_or(ReceiveError::Unavailable)?;
        let queue = self.queue.as_mut().ok_or(ReceiveError::Unavailable)?;
        let handle = queue.handle();
        if let Err(error) = self.state.drive(connection, &handle, now) {
            self.state.fail(error);
            return Err(error);
        }
        // The host alone reads the foreign display. This only drains its guest queue.
        if queue.dispatch_pending(&mut self.state).is_err() {
            self.state.fail(ReceiveError::Io);
            return Err(ReceiveError::Io);
        }
        let post_now = now.max(Instant::now());
        self.state.now = post_now;
        self.state.check(post_now)?;
        if let Err(error) = self.state.drive(connection, &handle, post_now) {
            self.state.fail(error);
            return Err(error);
        }
        self.state.consume_decision();
        if self
            .state
            .pipe
            .as_ref()
            .is_some_and(|(id, _)| Some(*id) == readable)
        {
            self.state.advance_transfer();
        }
        match connection.flush() {
            Ok(()) => {}
            Err(WaylandError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                self.state.fail(ReceiveError::Io);
                return Err(ReceiveError::Io);
            }
        }
        self.state.check(self.state.now.max(Instant::now()))
    }

    pub(super) fn pipe_fd(&self) -> Option<(OfferId, Arc<OwnedFd>)> {
        self.state
            .pipe
            .as_ref()
            .map(|(id, pipe)| (*id, pipe.readiness_fd()))
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        (!self.state.first_ready)
            .then_some(self.state.startup_deadline)
            .into_iter()
            .chain(self.state.coordinator.next_deadline())
            .min()
    }

    pub(super) fn take_wake(&mut self) -> Wake {
        self.state.coordinator.take_wake()
    }

    pub(super) fn cleanup(&mut self) -> Result<(), ReceiveError> {
        self.state.control.header().request_close(None);
        self.state.failure = Some(ReceiveError::Cancelled);
        self.state.coordinator.reconcile(Instant::now())?;
        // Drain already queued callbacks while all admissions are disarmed.
        // Only this worker dispatches this guest queue; the host only reads it.
        if let Some(queue) = self.queue.as_mut() {
            queue
                .dispatch_pending(&mut self.state)
                .map_err(|_| ReceiveError::Io)?;
        }
        self.state.cleanup_offers();
        drop(self.state.pipe.take());
        for seat in &mut self.state.seats {
            if let Some(device) = seat.device.as_ref() {
                if device.is_alive() {
                    device
                        .send_request(wl_data_device::Request::Release {})
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            seat.device = None;
            if let Some(proxy) = seat.proxy.as_ref() {
                if proxy.is_alive() {
                    proxy
                        .send_request(wl_seat::Request::Release {})
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            seat.proxy = None;
        }
        if let Some(connection) = self.connection.as_ref() {
            let backend = connection.backend();
            // Manager v3 and registry/callback have no destructor request.
            // Never destroy the host's borrowed target surface.
            if let Some((callback, _)) = self.state.pending.as_ref() {
                if callback.is_alive() {
                    backend
                        .destroy_object(&callback.id())
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            self.state.pending = None;
            if let Some(manager) = self.state.manager.as_ref() {
                if manager.is_alive() {
                    backend
                        .destroy_object(&manager.id())
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            self.state.manager = None;
            if let Some(registry) = self.state.registry.as_ref() {
                if registry.is_alive() {
                    backend
                        .destroy_object(&registry.id())
                        .map_err(|_| ReceiveError::Io)?;
                }
            }
            self.state.registry = None;
            match connection.flush() {
                Ok(()) => {}
                Err(WaylandError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err(ReceiveError::Io),
            }
        }
        self.state.target_surface = None;
        self.state.seats.clear();
        drop(self.queue.take());
        drop(self.connection.take());
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn pending_sync(&self) -> Option<(ObjectId, SyncPhase)> {
        self.state
            .pending
            .as_ref()
            .map(|(callback, phase)| (callback.id(), *phase))
    }
}

impl State {
    fn check(&mut self, now: Instant) -> Result<(), ReceiveError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.coordinator.reconcile(now)?;
        self.cleanup_offers();
        let closing = {
            let header = self.control.header();
            header
                .is_closing()
                .then_some(header.close_cause.unwrap_or(ReceiveError::Cancelled))
        };
        if let Some(error) = closing {
            // A normal caller close has no failure cause. Do not turn its
            // operation-level Cancelled result into a connection failure.
            self.failure = Some(error);
            self.coordinator.notify();
            return Err(error);
        }
        if !self.first_ready && now >= self.startup_deadline {
            self.fail(ReceiveError::Timeout);
            return Err(ReceiveError::Timeout);
        }
        Ok(())
    }

    fn fail(&mut self, error: ReceiveError) {
        if self.failure.is_none() {
            self.failure = Some(error);
            {
                let mut header = self.control.header();
                // A close can win after check() but before a coordinator call.
                // Its Closed sentinel must not become a normal close's cause.
                if error != ReceiveError::Closed || !header.is_closing() {
                    header.request_close(Some(error));
                }
            }
            let _ = self.coordinator.reconcile(self.now);
            self.cleanup_offers();
            self.coordinator.notify();
        }
    }

    fn drive(
        &mut self,
        connection: &Connection,
        handle: &QueueHandle<Self>,
        now: Instant,
    ) -> Result<(), ReceiveError> {
        if self.registry.is_none() {
            let request = self
                .coordinator
                .next_sync(now)?
                .ok_or(ReceiveError::InvalidPhase)?;
            self.pool.consume_registry()?;
            let registry = connection
                .display()
                .send_constructor(
                    wl_display::Request::GetRegistry {},
                    handle.make_data::<wl_registry::WlRegistry, _>(()),
                )
                .map_err(|_| ReceiveError::Io)?;
            self.registry = Some(registry);
            self.discovery_epoch = Some(request.epoch);
            return self.send_sync(connection, handle, SyncPhase::Discovery);
        }
        if let Some(phase) = self.completed.take() {
            match phase {
                SyncPhase::Discovery => {
                    self.discovery_complete = true;
                    self.bind_globals(handle)?;
                    let epoch = self
                        .discovery_epoch
                        .take()
                        .ok_or(ReceiveError::InvalidPhase)?;
                    return self.send_sync(connection, handle, SyncPhase::Arm(epoch));
                }
                SyncPhase::Arm(epoch) => {
                    self.coordinator.sync_done(epoch, now)?;
                    let armed = {
                        let header = self.control.header();
                        header.armed && header.epoch == epoch
                    };
                    if armed {
                        self.first_ready = true;
                        self.coordinator.notify();
                    }
                }
            }
        }
        if self.pending.is_none() {
            if let Some(request) = self.coordinator.next_sync(now)? {
                self.send_sync(connection, handle, SyncPhase::Arm(request.epoch))?;
            }
        }
        Ok(())
    }

    fn send_sync(
        &mut self,
        connection: &Connection,
        handle: &QueueHandle<Self>,
        phase: SyncPhase,
    ) -> Result<(), ReceiveError> {
        if self.pending.is_some() {
            return Err(ReceiveError::Busy);
        }
        let callback = connection
            .display()
            .send_constructor(
                wl_display::Request::Sync {},
                handle.make_data::<wl_callback::WlCallback, _>(phase),
            )
            .map_err(|_| ReceiveError::Io)?;
        self.pending = Some((callback, phase));
        Ok(())
    }

    fn bind_globals(&mut self, handle: &QueueHandle<Self>) -> Result<(), ReceiveError> {
        let name = self.manager_name.ok_or(ReceiveError::Unavailable)?;
        if self.seats.is_empty() {
            return Err(ReceiveError::Unavailable);
        }
        let registry = self.registry.as_ref().ok_or(ReceiveError::InvalidPhase)?;
        if self.manager.is_none() {
            self.manager = Some(
                registry
                    .send_constructor(
                        wl_registry::Request::Bind {
                            name,
                            id: (wl_data_device_manager::WlDataDeviceManager::interface(), 3),
                        },
                        handle.make_data::<wl_data_device_manager::WlDataDeviceManager, _>(()),
                    )
                    .map_err(|_| ReceiveError::Io)?,
            );
        }
        let manager = self.manager.as_ref().ok_or(ReceiveError::Unavailable)?;
        for seat in &mut self.seats {
            if seat.proxy.is_none() {
                seat.proxy = Some(
                    registry
                        .send_constructor(
                            wl_registry::Request::Bind {
                                name: seat.name,
                                id: (wl_seat::WlSeat::interface(), 5),
                            },
                            handle.make_data::<wl_seat::WlSeat, _>(()),
                        )
                        .map_err(|_| ReceiveError::Io)?,
                );
            }
            if seat.device.is_none() {
                let proxy = seat.proxy.as_ref().ok_or(ReceiveError::Unavailable)?;
                seat.device = Some(
                    manager
                        .send_constructor(
                            wl_data_device_manager::Request::GetDataDevice {
                                seat: proxy.clone(),
                            },
                            handle.make_data::<wl_data_device::WlDataDevice, _>(()),
                        )
                        .map_err(|_| ReceiveError::Io)?,
                );
            }
        }
        Ok(())
    }

    fn remove_global(&mut self, name: u32) {
        if self.manager_name == Some(name) {
            self.fail(ReceiveError::Unavailable);
            return;
        }
        if let Some(index) = self.seats.iter().position(|seat| seat.name == name) {
            let seat = self.seats.remove(index);
            if self.discovery_complete && self.seats.is_empty() {
                self.fail(ReceiveError::Unavailable);
            }
            if let Some(device) = seat.device {
                self.retire_device_offers(&device.id());
                if device
                    .send_request(wl_data_device::Request::Release {})
                    .is_err()
                {
                    self.fail(ReceiveError::Io);
                }
            }
            if let Some(proxy) = seat.proxy {
                if proxy.send_request(wl_seat::Request::Release {}).is_err() {
                    self.fail(ReceiveError::Io);
                }
            }
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if state.failure.is_some() {
            return;
        }
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == "wl_data_device_manager" && version >= 3 {
                    state.manager_name = state.manager_name.or(Some(name));
                } else if interface == "wl_seat" && version >= 5 {
                    if state.seats.len() == 4 {
                        state.fail(ReceiveError::ResourceLimit);
                        return;
                    }
                    state.seats.push(Seat {
                        name,
                        proxy: None,
                        device: None,
                        hover: None,
                    });
                }
                if state.discovery_complete {
                    if let Err(error) = state.bind_globals(handle) {
                        state.fail(error);
                    }
                }
            }
            wl_registry::Event::GlobalRemove { name } => state.remove_global(name),
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, SyncPhase> for State {
    fn event(
        state: &mut Self,
        callback: &wl_callback::WlCallback,
        _: wl_callback::Event,
        phase: &SyncPhase,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if state
            .pending
            .as_ref()
            .is_some_and(|(pending, expected)| pending.id() == callback.id() && expected == phase)
        {
            state.pending = None;
            state.completed = Some(*phase);
        }
    }
}

impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_data_device_manager::WlDataDeviceManager,
        _: wl_data_device_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
#[path = "wayland_tests.rs"]
mod tests;
