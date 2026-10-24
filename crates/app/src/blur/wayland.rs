use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::ManuallyDrop;

use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy as _, QueueHandle, WEnum, delegate_noop,
};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::{
    self, Capability, ExtBackgroundEffectManagerV1,
};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;

/// Covers any window size because the compositor clips the region to the surface.
const WHOLE: i32 = 1 << 16;

#[derive(Default)]
struct State {
    blur: bool,
}

struct Effects {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    compositor: WlCompositor,
    manager: Option<ExtBackgroundEffectManagerV1>,
    surfaces: HashMap<usize, ExtBackgroundEffectSurfaceV1>,
}

enum Link {
    Untried,
    Failed,
    Ready(Box<Effects>),
}

// Never dropped: thread-local destructors run in exit() after Qt has freed the wl_display
// these proxies live on, and destroying them then writes into freed memory.
thread_local! {
    static EFFECTS: ManuallyDrop<RefCell<Link>> = const { ManuallyDrop::new(RefCell::new(Link::Untried)) };
}

impl Effects {
    fn set(&mut self, surface: *mut c_void, enabled: bool) -> bool {
        if let Err(err) = self.queue.dispatch_pending(&mut self.state) {
            log::warn!("Wayland events: {err}");
        }

        let key = surface.addr();

        if !enabled || !self.state.blur || surface.is_null() {
            self.forget(key);
            return false;
        }

        if !self.surfaces.contains_key(&key) {
            let Some(effect) = self.create(surface) else {
                return false;
            };

            self.surfaces.insert(key, effect);
        }

        self.conn
            .flush()
            .inspect_err(|err| log::warn!("Wayland flush: {err}"))
            .is_ok()
    }

    fn create(&self, surface: *mut c_void) -> Option<ExtBackgroundEffectSurfaceV1> {
        let manager = self.manager.as_ref()?;

        #[expect(
            unsafe_code,
            reason = "adopts the wl_surface Qt created for a shown window"
        )]
        // SAFETY: surface is the live wl_surface of a Qt window, forgotten before Qt destroys it.
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()) }
            .inspect_err(|err| log::warn!("foreign wl_surface: {err}"))
            .ok()?;

        let wl_surface = WlSurface::from_id(&self.conn, id)
            .inspect_err(|err| log::warn!("foreign wl_surface: {err}"))
            .ok()?;

        let qh = self.queue.handle();

        let effect = manager.get_background_effect(&wl_surface, &qh, ());
        let region = self.compositor.create_region(&qh, ());
        region.add(0, 0, WHOLE, WHOLE);
        effect.set_blur_region(Some(&region));
        region.destroy();
        Some(effect)
    }

    fn forget(&mut self, key: usize) {
        if let Some(effect) = self.surfaces.remove(&key) {
            effect.destroy();

            if let Err(err) = self.conn.flush() {
                log::warn!("Wayland flush: {err}");
            }
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _registry: &WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _manager: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
            state.blur = matches!(flags, WEnum::Value(caps) if caps.contains(Capability::Blur));
        }
    }
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlRegion);
delegate_noop!(State: ExtBackgroundEffectSurfaceV1);

pub(super) fn apply(display: *mut c_void, surface: *mut c_void, enabled: bool) -> bool {
    EFFECTS.with(|cell| {
        let mut link = cell.borrow_mut();
        if matches!(*link, Link::Untried) {
            *link = connect(display).map_or(Link::Failed, |effects| Link::Ready(Box::new(effects)));
        }

        match &mut *link {
            Link::Ready(effects) => effects.set(surface, enabled),
            Link::Untried | Link::Failed => false,
        }
    })
}

/// Qt destroyed the surface, so its effect object is already inert.
pub(super) fn forget(surface: *mut c_void) {
    EFFECTS.with(|cell| {
        if let Link::Ready(effects) = &mut *cell.borrow_mut() {
            effects.forget(surface.addr());
        }
    });
}

fn connect(display: *mut c_void) -> Option<Effects> {
    if display.is_null() {
        return None;
    }

    #[expect(
        unsafe_code,
        reason = "wraps the wl_display Qt opened, which lives as long as the application"
    )]
    // SAFETY: display is Qt's wl_display, borrowed on the GUI thread for the app's lifetime.
    let backend = unsafe { Backend::from_foreign_display(display.cast()) };
    let conn = Connection::from_backend(backend);

    let (globals, mut queue) = registry_queue_init::<State>(&conn)
        .inspect_err(|err| log::warn!("Wayland registry: {err}"))
        .ok()?;

    let qh = queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&qh, 1..=1, ())
        .inspect_err(|err| log::warn!("no wl_compositor: {err}"))
        .ok()?;

    let manager = globals
        .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
        .inspect_err(|err| log::info!("no ext-background-effect-v1: {err}"))
        .ok();

    let mut state = State::default();

    if manager.is_some() {
        queue
            .roundtrip(&mut state)
            .inspect_err(|err| log::warn!("Wayland roundtrip: {err}"))
            .ok()?;
    }

    log::debug!("compositor blur capability: {}", state.blur);

    Some(Effects {
        conn,
        queue,
        state,
        compositor,
        manager,
        surfaces: HashMap::new(),
    })
}
