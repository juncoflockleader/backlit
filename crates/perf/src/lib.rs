use std::time::Instant;

use backlit_compositor_backend::HeadlessCompositor;
use backlit_demo_client::{render_demo_gui, verify_demo_gui};
use backlit_protocols::protocol_smoke_report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerfBudgets {
    pub render_budget_ms: u64,
    pub present_budget_us: u64,
}

impl Default for PerfBudgets {
    fn default() -> Self {
        Self {
            render_budget_ms: 500,
            present_budget_us: 50_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfSmokeReport {
    pub render_ms: u64,
    pub present_us: u64,
    pub non_background_pixels: u64,
    pub checksum: u64,
    pub golden_ok: bool,
    pub protocol_count: u64,
    pub surface_count: u64,
    pub initial_damaged_surfaces: u64,
    pub idle_damaged_surfaces: u64,
    pub targeted_damage_surfaces: u64,
    pub post_damage_idle_surfaces: u64,
    pub frames_presented: u64,
    pub no_idle_redraw: bool,
    pub targeted_damage_ok: bool,
    pub screenshot_verified: bool,
    pub protocols_verified: bool,
    pub budgets: PerfBudgets,
}

impl PerfSmokeReport {
    pub fn passed(&self) -> bool {
        self.screenshot_verified
            && self.protocols_verified
            && self.no_idle_redraw
            && self.targeted_damage_ok
            && self.render_ms <= self.budgets.render_budget_ms
            && self.present_us <= self.budgets.present_budget_us
    }
}

pub fn run_perf_smoke(width: u32, height: u32, budgets: PerfBudgets) -> PerfSmokeReport {
    let render_started = Instant::now();
    let canvas = render_demo_gui(width, height);
    let render_ms = render_started.elapsed().as_millis() as u64;
    let verification = verify_demo_gui(&canvas);

    let present_started = Instant::now();
    let mut compositor = HeadlessCompositor::default();
    let client = compositor.connect_client("perf-smoke-client");
    let terminal = compositor
        .submit_surface(client, "perf-terminal", 800, 600)
        .expect("perf client should be registered");
    compositor
        .submit_surface(client, "perf-browser", 1200, 800)
        .expect("perf client should be registered");
    let frame = compositor.present();
    let idle_frame = compositor.present();
    compositor
        .mark_damaged(terminal)
        .expect("perf surface should be registered");
    let targeted_frame = compositor.present();
    let post_damage_idle_frame = compositor.present();
    let present_us = present_started.elapsed().as_micros() as u64;

    let protocol_report = protocol_smoke_report();
    let no_idle_redraw =
        idle_frame.damaged_surfaces == 0 && post_damage_idle_frame.damaged_surfaces == 0;
    let targeted_damage_ok = targeted_frame.damaged_surfaces == 1;

    PerfSmokeReport {
        render_ms,
        present_us,
        non_background_pixels: verification.non_background_pixels,
        checksum: verification.checksum,
        golden_ok: verification.golden_ok,
        protocol_count: protocol_report.registered_protocols as u64,
        surface_count: frame.surface_count,
        initial_damaged_surfaces: frame.damaged_surfaces,
        idle_damaged_surfaces: idle_frame.damaged_surfaces,
        targeted_damage_surfaces: targeted_frame.damaged_surfaces,
        post_damage_idle_surfaces: post_damage_idle_frame.damaged_surfaces,
        frames_presented: post_damage_idle_frame.frame,
        no_idle_redraw,
        targeted_damage_ok,
        screenshot_verified: verification.passed(),
        protocols_verified: protocol_report.passed(),
        budgets,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_perf_smoke, PerfBudgets};

    #[test]
    fn perf_smoke_passes_with_default_budgets() {
        let report = run_perf_smoke(800, 520, PerfBudgets::default());

        assert!(report.passed(), "{report:?}");
        assert!(report.non_background_pixels > 10_000);
        assert!(report.golden_ok);
        assert_eq!(report.protocol_count, 7);
        assert_eq!(report.surface_count, 2);
        assert_eq!(report.initial_damaged_surfaces, 2);
        assert_eq!(report.idle_damaged_surfaces, 0);
        assert_eq!(report.targeted_damage_surfaces, 1);
        assert_eq!(report.post_damage_idle_surfaces, 0);
        assert_eq!(report.frames_presented, 4);
        assert!(report.no_idle_redraw);
        assert!(report.targeted_damage_ok);
    }

    #[test]
    fn perf_smoke_fails_when_budget_is_impossible() {
        let report = run_perf_smoke(
            800,
            520,
            PerfBudgets {
                render_budget_ms: 0,
                present_budget_us: 0,
            },
        );

        assert!(!report.passed());
    }
}
