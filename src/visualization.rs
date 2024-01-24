use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_rapier2d::render::RapierDebugRenderPlugin;

use crate::simulation::level::{Enemy, ENEMY_RADIUS};

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RapierDebugRenderPlugin {
                enabled: true,
                ..default()
            },
            PanCamPlugin,
        ))
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, (print_cam_pos, add_enemy_sprites));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), PanCam::default()));
}

fn print_cam_pos(mut _query: Query<&Transform, With<Camera>>) {
    // for cam in query.iter_mut() {
    //     println!("cam pos: {:?}", cam.translation);
    // }
}

fn add_enemy_sprites(
    mut commands: Commands,
    mut enemy_q: Query<Entity, Added<Enemy>>,
    asset_server: Res<AssetServer>,
) {
    for enemy in enemy_q.iter_mut() {
        commands.entity(enemy).insert((
            Sprite {
                color: Color::rgb_u8(224, 49, 29),
                custom_size: Some(Vec2::splat(ENEMY_RADIUS * 2.)),
                ..default()
            },
            asset_server.load::<Image>("circle.png"),
        ));
    }
}
