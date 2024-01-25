//! Copied and modified from https://github.com/aevyrie/bevy_framepace
//!
//! This is a [`bevy`] plugin that adds framepacing and framelimiting to improve input latency and
//! power use.
//!
//! # How it works
//!
//! This works by sleeping the app immediately before the event loop starts. In doing so, this
//! minimizes the time from when user input is captured (start of event loop), to when the frame is
//! presented on screen. Graphically, it looks like this:
//!
//! ```none
//!           /-- latency --\             /-- latency --\
//!  sleep -> input -> render -> sleep -> input -> render
//!  \----- event loop -----/    \----- event loop -----/
//! ```
//!
//! One of the interesting benefits of this is that you can keep latency low even if the framerate
//! is limited to a low value. Assuming you are able to reach the target frametime, there should be
//! no difference in motion-to-photon latency when limited to 10fps or 120fps.
//!
//! ```none
//!                same                                              same
//!           /-- latency --\                                   /-- latency --\
//!  sleep -> input -> render -> sleeeeeeeeeeeeeeeeeeeeeeeep -> input -> render
//!  \----- event loop -----/    \---------------- event loop ----------------/
//!           60 fps                           limited to 10 fps
//! ```

#[cfg(not(target_arch = "wasm32"))]
use bevy::winit::WinitWindows;
use bevy::{app::AppExit, ecs::system::NonSend};
use bevy::{
    ecs::schedule::ScheduleLabel,
    render::{pipelined_rendering::RenderExtractApp, RenderApp, RenderSet},
    utils::Instant,
};

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::prelude::*;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameTimeNormalize;

/// Adds framepacing and framelimiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FramepaceSettings>();

        let limit = FrametimeLimit::default();
        let settings = FramepaceSettings::default();
        let settings_proxy = FramepaceSettingsProxy::default();
        let stats = FramepaceStats(Arc::new(Mutex::new(FramePaceStatsInner {
            frametime: Duration::ZERO,
            oversleep: Duration::ZERO,
            sleep_end: Instant::now(),
        })));

        app.insert_resource(settings)
            .insert_resource(settings_proxy.clone())
            .insert_resource(limit.clone())
            .insert_resource(stats.clone())
            .add_systems(
                Last,
                (disable.run_if(is_exiting), update_proxy_resources).chain(),
            );

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, get_display_refresh_rate);

        // let mut main_schedule_order = app.world.resource_mut::<MainScheduleOrder>();
        // main_schedule_order.insert_after(Last, FrameTimeNormalize);
        // app.add_systems(FrameTimeNormalize, frametime_normalizer);

        if let Ok(sub_app) = app.get_sub_app_mut(RenderExtractApp) {
            sub_app
                .insert_resource(settings_proxy)
                .insert_resource(limit)
                .insert_resource(stats)
                .add_systems(Main, framerate_limiter);
        } else {
            app.sub_app_mut(RenderApp)
                .insert_resource(settings_proxy)
                .insert_resource(limit)
                .insert_resource(stats)
                .add_systems(
                    bevy::render::Render,
                    framerate_limiter
                        .in_set(RenderSet::Cleanup)
                        .after(World::clear_entities),
                );
        }
    }
}

pub fn is_exiting(mut exit: EventReader<AppExit>) -> bool {
    exit.read().next().is_some()
}

/// Framepacing plugin configuration.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct FramepaceSettings {
    /// Configures the framerate limiting strategy.
    pub limiter: Limiter,
}

impl FramepaceSettings {
    /// Builds plugin settings with the specified [`Limiter`] configuration.
    #[allow(dead_code)]
    pub fn with_limiter(mut self, limiter: Limiter) -> Self {
        self.limiter = limiter;
        self
    }
}
impl Default for FramepaceSettings {
    fn default() -> FramepaceSettings {
        FramepaceSettings {
            limiter: Limiter::Auto,
        }
    }
}

#[derive(Default, Debug, Clone, Resource)]
struct FramepaceSettingsProxy {
    /// Configures the framerate limiting strategy.
    limiter: Arc<Mutex<Limiter>>,
}

impl FramepaceSettingsProxy {
    fn is_enabled(&self) -> bool {
        self.limiter.try_lock().iter().any(|l| l.is_enabled())
    }
}

fn disable(mut settings: ResMut<FramepaceSettings>) {
    settings.limiter = Limiter::Off;
}

fn update_proxy_resources(settings: Res<FramepaceSettings>, proxy: Res<FramepaceSettingsProxy>) {
    if settings.is_changed() {
        if let Ok(mut limiter) = proxy.limiter.try_lock() {
            *limiter = settings.limiter.clone();
        }
    }
}

/// Configures the framelimiting technique for the app.
#[derive(Debug, Default, Clone, Reflect)]
pub enum Limiter {
    /// Uses the window's refresh rate to set the frametime limit, updating when the window changes
    /// monitors.
    #[default]
    Auto,
    /// Set a fixed manual frametime limit. This should be greater than the monitors frametime
    /// (`1.0 / monitor frequency`).
    Manual(Duration),
    /// Disables frame limiting
    Off,
}

impl Limiter {
    /// Returns `true` if the [`Limiter`] is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Limiter::Off)
    }

    /// Constructs a new [`Limiter`] from the provided `framerate`.
    #[allow(dead_code)]
    pub fn from_framerate(framerate: f64) -> Self {
        Limiter::Manual(Duration::from_secs_f64(1.0 / framerate))
    }
}

impl std::fmt::Display for Limiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            Limiter::Auto => "Auto".into(),
            Limiter::Manual(t) => format!("{:.2} fps", 1.0 / t.as_secs_f32()),
            Limiter::Off => "Off".into(),
        };
        write!(f, "{output}")
    }
}

/// Current frametime limit based on settings and monitor refresh rate.
#[derive(Debug, Default, Clone, Resource)]
struct FrametimeLimit(Arc<Mutex<Duration>>);

#[cfg(not(target_arch = "wasm32"))]
fn get_display_refresh_rate(
    settings: Res<FramepaceSettings>,
    winit: NonSend<WinitWindows>,
    windows: Query<Entity, With<Window>>,
    frame_limit: Res<FrametimeLimit>,
) {
    let new_frametime = match settings.limiter {
        Limiter::Auto => match detect_frametime(winit, windows.iter()) {
            Some(frametime) => frametime,
            None => return,
        },
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            // info!("Frame limiter disabled");
            return;
        }
    };

    if let Ok(mut limit) = frame_limit.0.try_lock() {
        if new_frametime != *limit {
            // info!("Frametime limit changed to: {:?}", new_frametime);
            *limit = new_frametime;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::redundant_closure_for_method_calls)]
fn detect_frametime(
    winit: NonSend<WinitWindows>,
    windows: impl Iterator<Item = Entity>,
) -> Option<Duration> {
    let current_monitor = windows
        .filter_map(|e| winit.get_window(e))
        .find_map(|w| w.current_monitor())?;

    let best_framerate = {
        f64::from(bevy::winit::get_best_videomode(&current_monitor).refresh_rate_millihertz())
            / 1000.0
    };

    let best_frametime = Duration::from_secs_f64(1.0 / best_framerate);
    Some(best_frametime)
}

/// Holds frame time measurements for framepacing diagnostics
#[derive(Clone, Debug, Resource, Deref, DerefMut)]
pub struct FramepaceStats(Arc<Mutex<FramePaceStatsInner>>);

#[derive(Clone, Debug)]
pub struct FramePaceStatsInner {
    pub frametime: Duration,
    pub oversleep: Duration,
    pub sleep_end: Instant,
}

fn framerate_limiter(
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramepaceStats>,
    settings: Res<FramepaceSettingsProxy>,
) {
    let limit = { *target_frametime.0.lock().unwrap() };

    #[cfg(not(target_arch = "wasm32"))]
    {
        if settings.is_enabled() {
            let (sleep_end, oversleep) = {
                let stats = stats.lock().unwrap();
                (stats.sleep_end, stats.oversleep)
            };
            let sleep_time = limit.saturating_sub(sleep_end.elapsed() + oversleep);
            spin_sleep::sleep(sleep_time);
        }
    }

    if let Ok(mut stats) = stats.try_lock() {
        let frametime_actual = stats.sleep_end.elapsed();
        stats.sleep_end = Instant::now();
        stats.frametime = frametime_actual;
        stats.oversleep = frametime_actual.saturating_sub(limit);

        // info!(
        // 	"frametime_actual={frametime_actual:?}, oversleep={:?}",
        // 	stats.oversleep
        // );

        #[cfg(target_arch = "wasm32")]
        {
            // Wasm uses pacing by itself, so just detect our target from actual frametime
            *target_frametime.0.lock().unwrap() =
                frametime_actual.min(Duration::from_secs_f64(1. / 30.));
        }
        // if stats.oversleep > Duration::from_millis(1) {
        // 	warn!("HIGH OVERSLEEP");
        // }
    }
}
