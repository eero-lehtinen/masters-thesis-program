#![allow(clippy::type_complexity)]

use bevy::{app::AppExit, prelude::*, window::WindowResolution};
use framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use visualization::VisualizationPlugin;

use crate::simulation::SimulationPlugin;

mod framepace;
pub mod simulation;
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
            ))
            .insert_resource(FramepaceSettings {
                limiter: Limiter::from_framerate(FRAME_RATE as f64),
            })
            .add_systems(PostUpdate, exit_bench)
            .run();
    } else {
        App::new()
            .add_plugins((MinimalPlugins, SimulationPlugin))
            .add_systems(PostUpdate, exit_bench)
            .run();
    }
}

fn exit_bench(mut exit: ResMut<Events<AppExit>>, mut ticks: Local<u32>) {
    if *ticks >= BENCHMARK_TICKS {
        exit.send(AppExit);
    } else {
        *ticks += 1;
    }
}
