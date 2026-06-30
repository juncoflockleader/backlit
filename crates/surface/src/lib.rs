use backlit_protocols::{lookup_protocol, ProtocolDomain};
use backlit_window_policy::{OutputLayout, Rect, WindowId, WindowPolicy, WindowState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRole {
    XdgToplevel,
    XdgPopup,
}

impl SurfaceRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XdgToplevel => "xdg-toplevel",
            Self::XdgPopup => "xdg-popup",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePhase {
    Created,
    Configured,
    Mapped,
    Closed,
}

impl SurfacePhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Configured => "configured",
            Self::Mapped => "mapped",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Configure {
    pub serial: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    pub fullscreen: bool,
    pub close_requested: bool,
}

impl Configure {
    const fn initial(serial: u64, width: i32, height: i32) -> Self {
        Self {
            serial,
            x: 0,
            y: 0,
            width,
            height,
            maximized: false,
            fullscreen: false,
            close_requested: false,
        }
    }

    const fn maximized(serial: u64, area: Rect) -> Self {
        Self {
            serial,
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
            maximized: true,
            fullscreen: false,
            close_requested: false,
        }
    }

    const fn fullscreen(serial: u64, area: Rect) -> Self {
        Self {
            serial,
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
            maximized: false,
            fullscreen: true,
            close_requested: false,
        }
    }

    const fn close(serial: u64) -> Self {
        Self {
            serial,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            maximized: false,
            fullscreen: false,
            close_requested: true,
        }
    }

    const fn popup(serial: u64, area: Rect) -> Self {
        Self {
            serial,
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
            maximized: false,
            fullscreen: false,
            close_requested: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToplevelSurface {
    pub id: SurfaceId,
    pub role: SurfaceRole,
    pub title: String,
    pub phase: SurfacePhase,
    pub window_id: Option<WindowId>,
    pub parent: Option<SurfaceId>,
    preferred_size: (i32, i32),
    popup_offset: (i32, i32),
    pending_configure: Option<Configure>,
    acked_serial: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SurfaceManager {
    policy: WindowPolicy,
    layout: OutputLayout,
    surfaces: Vec<ToplevelSurface>,
    next_surface_id: u64,
    next_serial: u64,
}

impl SurfaceManager {
    pub fn new(layout: OutputLayout) -> Self {
        Self {
            policy: WindowPolicy::default(),
            layout,
            surfaces: Vec::new(),
            next_surface_id: 1,
            next_serial: 1,
        }
    }

    pub fn create_toplevel(
        &mut self,
        title: impl Into<String>,
        preferred_size: (i32, i32),
    ) -> SurfaceId {
        let id = SurfaceId(self.next_surface_id);
        self.next_surface_id += 1;

        self.surfaces.push(ToplevelSurface {
            id,
            role: SurfaceRole::XdgToplevel,
            title: title.into(),
            phase: SurfacePhase::Created,
            window_id: None,
            parent: None,
            preferred_size,
            popup_offset: (0, 0),
            pending_configure: None,
            acked_serial: None,
        });

        id
    }

    pub fn create_popup(
        &mut self,
        parent: SurfaceId,
        title: impl Into<String>,
        preferred_size: (i32, i32),
        popup_offset: (i32, i32),
    ) -> Option<SurfaceId> {
        let parent_surface = self.surface(parent)?;
        if parent_surface.role != SurfaceRole::XdgToplevel
            || parent_surface.phase == SurfacePhase::Closed
        {
            return None;
        }

        let id = SurfaceId(self.next_surface_id);
        self.next_surface_id += 1;

        self.surfaces.push(ToplevelSurface {
            id,
            role: SurfaceRole::XdgPopup,
            title: title.into(),
            phase: SurfacePhase::Created,
            window_id: None,
            parent: Some(parent),
            preferred_size,
            popup_offset,
            pending_configure: None,
            acked_serial: None,
        });

        Some(id)
    }

    pub fn send_initial_configure(&mut self, id: SurfaceId) -> Option<Configure> {
        let index = self.surface_index(id)?;
        let serial = self.next_serial();
        let configure = self.initial_configure_for(index, serial)?;
        self.surfaces[index].pending_configure = Some(configure);
        self.surfaces[index].phase = SurfacePhase::Configured;
        Some(configure)
    }

    pub fn ack_configure(&mut self, id: SurfaceId, serial: u64) -> bool {
        let Some(surface) = self.surface_mut(id) else {
            return false;
        };

        if surface
            .pending_configure
            .map(|configure| configure.serial == serial)
            .unwrap_or(false)
        {
            surface.acked_serial = Some(serial);
            true
        } else {
            false
        }
    }

    pub fn commit(&mut self, id: SurfaceId) -> bool {
        let Some(index) = self.surface_index(id) else {
            return false;
        };

        let surface = &self.surfaces[index];
        let Some(configure) = surface.pending_configure else {
            return false;
        };
        if surface.acked_serial != Some(configure.serial) || surface.phase == SurfacePhase::Closed {
            return false;
        }

        if self.surfaces[index].role == SurfaceRole::XdgPopup {
            if !self.popup_parent_mapped(index) {
                return false;
            }
            self.surfaces[index].phase = SurfacePhase::Mapped;
            return true;
        }

        if let Some(window_id) = self.surfaces[index].window_id {
            self.policy.focus(window_id)
        } else {
            let window_id = self.policy.add_window(
                self.surfaces[index].title.clone(),
                (configure.width, configure.height),
            );
            self.surfaces[index].window_id = Some(window_id);
            self.surfaces[index].phase = SurfacePhase::Mapped;
            true
        }
    }

    pub fn request_maximize(&mut self, id: SurfaceId) -> Option<Configure> {
        let index = self.surface_index(id)?;
        let window_id = self.surfaces[index].window_id?;
        let area = self.layout.work_area();
        if !self.policy.maximize_window(window_id, area) {
            return None;
        }

        let serial = self.next_serial();
        let configure = Configure::maximized(serial, area);
        self.surfaces[index].pending_configure = Some(configure);
        self.surfaces[index].acked_serial = Some(serial);
        Some(configure)
    }

    pub fn request_fullscreen(&mut self, id: SurfaceId) -> Option<Configure> {
        let index = self.surface_index(id)?;
        let window_id = self.surfaces[index].window_id?;
        let area = self.layout.output;
        if !self.policy.fullscreen_window(window_id, area) {
            return None;
        }

        let serial = self.next_serial();
        let configure = Configure::fullscreen(serial, area);
        self.surfaces[index].pending_configure = Some(configure);
        self.surfaces[index].acked_serial = Some(serial);
        Some(configure)
    }

    pub fn request_close(&mut self, id: SurfaceId) -> Option<Configure> {
        let index = self.surface_index(id)?;
        if self.surfaces[index].phase == SurfacePhase::Closed {
            return None;
        }

        let serial = self.next_serial();
        let configure = Configure::close(serial);
        self.surfaces[index].pending_configure = Some(configure);
        Some(configure)
    }

    pub fn close(&mut self, id: SurfaceId) -> bool {
        let Some(index) = self.surface_index(id) else {
            return false;
        };

        if let Some(window_id) = self.surfaces[index].window_id.take() {
            self.policy.remove_window(window_id);
        }
        self.surfaces[index].phase = SurfacePhase::Closed;
        for surface in &mut self.surfaces {
            if surface.parent == Some(id) && surface.phase != SurfacePhase::Closed {
                surface.phase = SurfacePhase::Closed;
            }
        }
        true
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&ToplevelSurface> {
        self.surfaces.iter().find(|surface| surface.id == id)
    }

    pub fn policy(&self) -> &WindowPolicy {
        &self.policy
    }

    pub fn layout(&self) -> OutputLayout {
        self.layout
    }

    fn surface_index(&self, id: SurfaceId) -> Option<usize> {
        self.surfaces.iter().position(|surface| surface.id == id)
    }

    fn surface_mut(&mut self, id: SurfaceId) -> Option<&mut ToplevelSurface> {
        self.surfaces.iter_mut().find(|surface| surface.id == id)
    }

    fn initial_configure_for(&self, index: usize, serial: u64) -> Option<Configure> {
        let surface = &self.surfaces[index];
        match surface.role {
            SurfaceRole::XdgToplevel => {
                let (width, height) = surface.preferred_size;
                Some(Configure::initial(serial, width, height))
            }
            SurfaceRole::XdgPopup => self
                .popup_rect(index)
                .map(|area| Configure::popup(serial, area)),
        }
    }

    fn popup_parent_mapped(&self, index: usize) -> bool {
        let Some(parent) = self.surfaces[index].parent else {
            return false;
        };
        self.surface(parent)
            .map(|surface| {
                surface.role == SurfaceRole::XdgToplevel && surface.phase == SurfacePhase::Mapped
            })
            .unwrap_or(false)
    }

    fn popup_rect(&self, index: usize) -> Option<Rect> {
        let surface = &self.surfaces[index];
        let parent_id = surface.parent?;
        let parent_window = self.surface(parent_id)?.window_id?;
        let parent_geometry = self.policy.window(parent_window)?.geometry;
        let width = surface
            .preferred_size
            .0
            .max(1)
            .min(self.layout.output.width);
        let height = surface
            .preferred_size
            .1
            .max(1)
            .min(self.layout.output.height);
        let max_x = self.layout.output.x + self.layout.output.width - width;
        let max_y = self.layout.output.y + self.layout.output.height - height;
        let x = clamp_i32(
            parent_geometry.x + surface.popup_offset.0,
            self.layout.output.x,
            max_x,
        );
        let y = clamp_i32(
            parent_geometry.y + surface.popup_offset.1,
            self.layout.output.y,
            max_y,
        );

        Some(Rect::new(x, y, width, height))
    }

    fn next_serial(&mut self) -> u64 {
        let serial = self.next_serial;
        self.next_serial += 1;
        serial
    }
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max.max(min))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceLifecycleSmokeReport {
    pub xdg_shell_registered: bool,
    pub created_toplevel: bool,
    pub initial_configured: bool,
    pub ack_configure_ok: bool,
    pub mapped_window: bool,
    pub focused_after_map: bool,
    pub created_popup: bool,
    pub popup_configured: bool,
    pub popup_ack_configure_ok: bool,
    pub popup_mapped: bool,
    pub popup_position_constrained: bool,
    pub popup_keeps_parent_focus: bool,
    pub popup_did_not_create_window: bool,
    pub popup_close_requested: bool,
    pub popup_closed: bool,
    pub windows_after_popup_close: u64,
    pub maximize_configured: bool,
    pub maximize_uses_work_area: bool,
    pub fullscreen_configured: bool,
    pub fullscreen_uses_output: bool,
    pub close_requested: bool,
    pub window_removed: bool,
    pub windows_after_close: u64,
}

impl SurfaceLifecycleSmokeReport {
    pub fn passed(self) -> bool {
        self.xdg_shell_registered
            && self.created_toplevel
            && self.initial_configured
            && self.ack_configure_ok
            && self.mapped_window
            && self.focused_after_map
            && self.created_popup
            && self.popup_configured
            && self.popup_ack_configure_ok
            && self.popup_mapped
            && self.popup_position_constrained
            && self.popup_keeps_parent_focus
            && self.popup_did_not_create_window
            && self.popup_close_requested
            && self.popup_closed
            && self.windows_after_popup_close == 1
            && self.maximize_configured
            && self.maximize_uses_work_area
            && self.fullscreen_configured
            && self.fullscreen_uses_output
            && self.close_requested
            && self.window_removed
            && self.windows_after_close == 0
    }
}

pub fn run_surface_lifecycle_smoke() -> SurfaceLifecycleSmokeReport {
    let xdg_shell_registered = lookup_protocol("xdg_wm_base")
        .map(|protocol| protocol.mvp_required && protocol.domain == ProtocolDomain::XdgShell)
        .unwrap_or(false);

    let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
    let surface = manager.create_toplevel("terminal", (640, 480));
    let created_toplevel = manager
        .surface(surface)
        .map(|surface| {
            surface.role == SurfaceRole::XdgToplevel && surface.phase == SurfacePhase::Created
        })
        .unwrap_or(false);

    let initial_configure = manager.send_initial_configure(surface);
    let initial_configured = initial_configure
        .map(|configure| configure.width == 640 && configure.height == 480)
        .unwrap_or(false);
    let ack_configure_ok = initial_configure
        .map(|configure| manager.ack_configure(surface, configure.serial))
        .unwrap_or(false);
    let mapped_window = manager.commit(surface);

    let window_id = manager
        .surface(surface)
        .and_then(|surface| surface.window_id);
    let focused_after_map = window_id
        .map(|window_id| manager.policy().focused() == Some(window_id))
        .unwrap_or(false);
    let popup = manager.create_popup(surface, "terminal-menu", (240, 160), (32, 36));
    let created_popup = popup
        .and_then(|popup| manager.surface(popup))
        .map(|popup_surface| {
            popup_surface.role == SurfaceRole::XdgPopup
                && popup_surface.parent == Some(surface)
                && popup_surface.phase == SurfacePhase::Created
        })
        .unwrap_or(false);
    let popup_configure = popup.and_then(|popup| manager.send_initial_configure(popup));
    let popup_configured = popup_configure
        .map(|configure| {
            configure.x >= manager.layout().output.x
                && configure.y >= manager.layout().output.y
                && configure.width == 240
                && configure.height == 160
        })
        .unwrap_or(false);
    let popup_ack_configure_ok = match (popup, popup_configure) {
        (Some(popup), Some(configure)) => manager.ack_configure(popup, configure.serial),
        _ => false,
    };
    let popup_mapped = popup.map(|popup| manager.commit(popup)).unwrap_or(false);
    let popup_position_constrained = popup_configure
        .map(|configure| {
            configure.x >= manager.layout().output.x
                && configure.y >= manager.layout().output.y
                && configure.x + configure.width
                    <= manager.layout().output.x + manager.layout().output.width
                && configure.y + configure.height
                    <= manager.layout().output.y + manager.layout().output.height
        })
        .unwrap_or(false);
    let popup_keeps_parent_focus = window_id
        .map(|window_id| manager.policy().focused() == Some(window_id))
        .unwrap_or(false);
    let popup_did_not_create_window = manager.policy().windows().len() == 1;
    let popup_close_requested = popup
        .and_then(|popup| manager.request_close(popup))
        .map(|configure| configure.close_requested)
        .unwrap_or(false);
    let popup_closed = popup
        .map(|popup| {
            manager.close(popup)
                && manager
                    .surface(popup)
                    .map(|surface| surface.phase == SurfacePhase::Closed)
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    let windows_after_popup_close = manager.policy().windows().len() as u64;

    let maximize_configured = manager
        .request_maximize(surface)
        .map(|configure| configure.maximized && !configure.fullscreen)
        .unwrap_or(false);
    let maximize_uses_work_area = window_id
        .and_then(|window_id| manager.policy().window(window_id))
        .map(|window| {
            window.state == WindowState::Maximized
                && window.geometry == manager.layout().work_area()
        })
        .unwrap_or(false);

    let fullscreen_configured = manager
        .request_fullscreen(surface)
        .map(|configure| configure.fullscreen && !configure.maximized)
        .unwrap_or(false);
    let fullscreen_uses_output = window_id
        .and_then(|window_id| manager.policy().window(window_id))
        .map(|window| {
            window.state == WindowState::Fullscreen && window.geometry == manager.layout().output
        })
        .unwrap_or(false);

    let close_requested = manager
        .request_close(surface)
        .map(|configure| configure.close_requested)
        .unwrap_or(false);
    let close_ok = manager.close(surface);
    let window_removed = close_ok
        && window_id
            .map(|window_id| manager.policy().window(window_id).is_none())
            .unwrap_or(false)
        && manager
            .surface(surface)
            .map(|surface| surface.phase == SurfacePhase::Closed)
            .unwrap_or(false);
    let windows_after_close = manager.policy().windows().len() as u64;

    SurfaceLifecycleSmokeReport {
        xdg_shell_registered,
        created_toplevel,
        initial_configured,
        ack_configure_ok,
        mapped_window,
        focused_after_map,
        created_popup,
        popup_configured,
        popup_ack_configure_ok,
        popup_mapped,
        popup_position_constrained,
        popup_keeps_parent_focus,
        popup_did_not_create_window,
        popup_close_requested,
        popup_closed,
        windows_after_popup_close,
        maximize_configured,
        maximize_uses_work_area,
        fullscreen_configured,
        fullscreen_uses_output,
        close_requested,
        window_removed,
        windows_after_close,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_surface_lifecycle_smoke, SurfaceManager, SurfacePhase, SurfaceRole};
    use backlit_window_policy::{OutputLayout, WindowState};

    #[test]
    fn surface_lifecycle_smoke_passes() {
        let report = run_surface_lifecycle_smoke();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.windows_after_close, 0);
    }

    #[test]
    fn toplevel_maps_after_configure_ack_and_commit() {
        let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
        let surface = manager.create_toplevel("browser", (640, 480));
        let configure = manager.send_initial_configure(surface).unwrap();

        assert!(manager.ack_configure(surface, configure.serial));
        assert!(manager.commit(surface));

        let toplevel = manager.surface(surface).unwrap();
        assert_eq!(toplevel.role, SurfaceRole::XdgToplevel);
        assert_eq!(toplevel.phase, SurfacePhase::Mapped);
        assert_eq!(manager.policy().windows().len(), 1);
        assert_eq!(manager.policy().focused(), toplevel.window_id);
    }

    #[test]
    fn mapped_toplevel_accepts_maximize_fullscreen_and_close() {
        let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
        let surface = manager.create_toplevel("video", (640, 480));
        let configure = manager.send_initial_configure(surface).unwrap();
        assert!(manager.ack_configure(surface, configure.serial));
        assert!(manager.commit(surface));
        let window_id = manager.surface(surface).unwrap().window_id.unwrap();

        let maximize = manager.request_maximize(surface).unwrap();
        assert!(maximize.maximized);
        assert_eq!(
            manager.policy().window(window_id).unwrap().state,
            WindowState::Maximized
        );

        let fullscreen = manager.request_fullscreen(surface).unwrap();
        assert!(fullscreen.fullscreen);
        assert_eq!(
            manager.policy().window(window_id).unwrap().state,
            WindowState::Fullscreen
        );

        let close = manager.request_close(surface).unwrap();
        assert!(close.close_requested);
        assert!(manager.close(surface));
        assert!(manager.policy().window(window_id).is_none());
    }

    #[test]
    fn popup_maps_under_parent_without_creating_policy_window() {
        let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
        let parent = manager.create_toplevel("browser", (640, 480));
        let configure = manager.send_initial_configure(parent).unwrap();
        assert!(manager.ack_configure(parent, configure.serial));
        assert!(manager.commit(parent));
        let parent_window = manager.surface(parent).unwrap().window_id;

        let popup = manager
            .create_popup(parent, "browser-menu", (240, 160), (32, 36))
            .unwrap();
        let popup_configure = manager.send_initial_configure(popup).unwrap();
        assert_eq!(popup_configure.width, 240);
        assert_eq!(popup_configure.height, 160);
        assert!(manager.ack_configure(popup, popup_configure.serial));
        assert!(manager.commit(popup));

        let popup_surface = manager.surface(popup).unwrap();
        assert_eq!(popup_surface.role, SurfaceRole::XdgPopup);
        assert_eq!(popup_surface.parent, Some(parent));
        assert_eq!(popup_surface.phase, SurfacePhase::Mapped);
        assert_eq!(manager.policy().windows().len(), 1);
        assert_eq!(manager.policy().focused(), parent_window);
    }
}
