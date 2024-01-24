use bevy::prelude::*;
use bevy_rapier2d::render::RapierDebugRenderPlugin;

struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RapierDebugRenderPlugin {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    todo!()
}
