use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use super::{spawning::Enemy, SimulationSet};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, move_with_flow_field.in_set(SimulationSet::Move));
    }
}

const ENEMY_SPEED: f32 = 6.;
use super::navigation::NavGrid;

#[cfg(not(feature = "navigation2"))]
use super::navigation::NavGridInner;

pub fn move_with_flow_field(world: &mut World) {
    use super::navigation::{Flow, FlowField};

    let mut system_state: SystemState<(
        Query<(&mut Transform, &mut Velocity), With<Enemy>>,
        Res<NavGrid>,
        Option<Res<FlowField>>,
        ResMut<Statistics>,
    )> = SystemState::new(world);

    let (mut enemy_q, nav_grid, flow_field, mut stats) = system_state.get_mut(world);

    let Some(flow_field) = flow_field else {
        return;
    };

    let start = Instant::now();
    enemy_q
        .iter_mut()
        .for_each(|(mut transform, mut velocity)| {
            let max_speed_change = ENEMY_SPEED * 0.4; // Takes 5 ticks to completely change direction
            let pos = transform.translation.truncate();
            let idx = nav_grid.pos_to_index(pos);
            let add_vel = flow_field.get(idx).copied().map_or_else(
                || Vec2::ZERO,
                |flow| {
                    #[cfg(not(feature = "navigation2"))]
                    if flow == Flow::Source {
                        (NavGridInner::index_to_pos(idx) - pos).normalize_or_zero()
                            * max_speed_change
                    } else if flow == Flow::None {
                        Vec2::ZERO
                    } else {
                        flow.to_dir() * max_speed_change
                    }

                    #[cfg(feature = "navigation2")]
                    if flow == Flow::LineOfSight {
                        let closest = flow_field.closest_target(pos).unwrap();
                        (closest - pos).normalize_or_zero() * max_speed_change
                    } else if flow == Flow::None {
                        Vec2::ZERO
                    } else {
                        flow.to_dir() * max_speed_change
                    }
                },
            );

            let new_vel = velocity.0 + add_vel;

            let length = new_vel.length();
            // If over maximum, scale it down slowly
            let max = (length - ENEMY_SPEED * 0.5).clamp(ENEMY_SPEED, ENEMY_SPEED * 5.0);
            let vel = max * (new_vel / length);

            let pos = pos + vel * DELTA_TIME;
            let valid = flow_field
                .get(nav_grid.pos_to_index(pos))
                .is_some_and(|flow| *flow != Flow::None);

            if valid {
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
                velocity.0 = vel;
            } else {
                velocity.0 = Vec2::ZERO;
            }
        });

    stats.add("movement", start.elapsed());
}
