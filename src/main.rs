#![allow(clippy::type_complexity)]

use bevy::{
    app::AppExit, core::FrameCount, prelude::*, time::TimePlugin, window::WindowResolution,
};
use framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use statistics::StatisticsPlugin;
use visualization::VisualizationPlugin;

use crate::simulation::SimulationPlugin;

mod framepace;
pub mod simulation;
pub mod statistics;
pub mod utils;
pub mod visualization;

/// How often the simulation is updated when visualizing.
/// Benchmarks are ran as fast as possible.
const FRAME_RATE: i32 = 60;
const DELTA_TIME: f32 = 1.0 / FRAME_RATE as f32;

const BENCHMARK_TICKS: u32 = 1000;

fn main() {
    if cfg!(not(feature = "bench")) {
        App::new()
            .add_plugins((
                DefaultPlugins.set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(700., 700.),
                        ..default()
                    }),
                    ..default()
                }),
                SimulationPlugin,
                VisualizationPlugin,
                FramepacePlugin,
                StatisticsPlugin,
            ))
            .insert_resource(FramepaceSettings {
                limiter: Limiter::from_framerate(FRAME_RATE as f64),
            })
            .run();
    } else {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins.build().disable::<TimePlugin>(),
            SimulationPlugin,
            StatisticsPlugin,
        ))
        .add_systems(First, exit_bench);
        // bevy_mod_debugdump::print_schedule_graph(&mut app, PreUpdate);
        app.run();
    }
}

fn exit_bench(mut exit: ResMut<Events<AppExit>>, frames: Res<FrameCount>) {
    if frames.0 >= BENCHMARK_TICKS {
        exit.send(AppExit);
    }
}
