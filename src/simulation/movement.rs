use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use super::{spawning::Enemy, SimulationSet};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, move_with_flow_field.in_set(SimulationSet::Move));
    }
}

const ENEMY_SPEED: f32 = 4.;
use super::navigation::{Flow, FlowField, NavGrid, NavGridInner};

pub fn move_with_flow_field(world: &mut World) {
    let mut system_state: SystemState<(
        Query<(&mut Transform, &mut Velocity), With<Enemy>>,
        Res<NavGrid>,
        Option<Res<FlowField>>,
        ResMut<Statistics>,
    )> = SystemState::new(world);

    let (mut enemy_q, nav_grid, flow_field, mut stats) = system_state.get_mut(world);

    let Some(flow_field) = flow_field.as_ref() else {
        return;
    };

    let start = Instant::now();
    enemy_q
        .iter_mut()
        .for_each(|(mut transform, mut velocity)| {
            #[cfg(not(feature = "new_movement"))]
            let max_speed_change = ENEMY_SPEED * 0.4;
            #[cfg(feature = "new_movement")]
            let max_speed_change = ENEMY_SPEED * 0.05;

            let pos = transform.translation.truncate();
            let idx = nav_grid.pos_to_index(pos);
            // #[cfg(navigation1)]
            let add_vel = flow_field.get(idx).copied().map_or_else(
                || Vec2::ZERO,
                |flow| {
                    if flow == Flow::Source {
                        (NavGridInner::index_to_pos(idx) - pos).normalize_or_zero()
                            * max_speed_change
                    } else if flow == Flow::None {
                        Vec2::ZERO
                    } else {
                        flow.to_dir() * max_speed_change
                    }
                },
            );

            let mut new_vel = velocity.0 + add_vel;

            cfg_if::cfg_if! {
                if #[cfg(feature = "new_move_clamp")] {
                    let length = new_vel.length();
                    let speeding = length - ENEMY_SPEED;
                    if speeding > 0. {
                        new_vel = new_vel / length * (length - speeding * 0.5)
                    }
                } else {
                    let length = new_vel.length();
                    // If over maximum, scale it down slowly
                    let max = (length - ENEMY_SPEED * 0.5).clamp(ENEMY_SPEED, ENEMY_SPEED * 5.0);
                    new_vel = new_vel / length * max;
                }
            }

            let pos = pos + new_vel * DELTA_TIME;
            let valid = flow_field
                .get(nav_grid.pos_to_index(pos))
                .is_some_and(|flow| *flow != Flow::None);

            if valid {
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
                velocity.0 = new_vel;
            } else {
                velocity.0 = Vec2::ZERO;
            }
        });

    stats.add("movement", start.elapsed());
}
