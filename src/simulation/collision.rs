use bevy::prelude::*;

pub struct CollisionPlugin;

use bevy_rapier2d::{
    prelude::*,
    rapier::prelude::{ColliderBuilder, Isometry},
};

use super::SimulationSet;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationToRenderTime>()
            .init_resource::<RapierContext>()
            .insert_resource(RapierConfiguration {
                gravity: Vec2::ZERO,
                physics_pipeline_active: false,
                query_pipeline_active: true,
                force_update_from_transform_changes: false,
                ..default()
            })
            .add_systems(
                PreUpdate,
                (
                    init_colliders,
                    apply_collider_user_changes,
                    sync_removals,
                    update_query_pipeline,
                )
                    .chain()
                    .in_set(SimulationSet::ApplyColliders),
            );
    }
}

pub const GROUP_WALL: Group = Group::GROUP_1;
pub const GROUP_ENEMY: Group = Group::GROUP_3;

fn transform_to_iso(transform: &Transform, physics_scale: f32) -> Isometry<f32> {
    Isometry::new(
        (transform.translation / physics_scale).xy().into(),
        transform.rotation.to_scaled_axis().z,
    )
}

pub fn apply_collider_user_changes(
    _config: Res<RapierConfiguration>,
    mut context: ResMut<RapierContext>,
    changed_collider_transforms: Query<(&RapierColliderHandle, &Transform), Changed<Transform>>,
    changed_shapes: Query<(&RapierColliderHandle, &Collider), Changed<Collider>>,
    changed_disabled: Query<(&RapierColliderHandle, &ColliderDisabled), Changed<ColliderDisabled>>,
) {
    let scale = context.physics_scale();

    for (handle, transform) in changed_collider_transforms.iter() {
        if let Some(co) = context.colliders.get_mut(handle.0) {
            if co.parent().is_none() {
                co.set_position(transform_to_iso(transform, scale));
            }
        }
    }

    for (handle, shape) in changed_shapes.iter() {
        if let Some(co) = context.colliders.get_mut(handle.0) {
            // let mut scaled_shape = shape.clone();
            // scaled_shape.set_scale(shape.scale() / scale, config.scaled_shape_subdivision);
            co.set_shape(shape.raw.clone());
        }
    }

    for (handle, _) in changed_disabled.iter() {
        if let Some(co) = context.colliders.get_mut(handle.0) {
            co.set_enabled(false);
        }
    }
}

pub type ColliderComponents<'a> = (
    Entity,
    &'a Collider,
    Option<&'a Sensor>,
    Option<&'a CollisionGroups>,
    Option<&'a ColliderDisabled>,
);

pub fn init_colliders(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,
    colliders: Query<(ColliderComponents, &Transform), Without<RapierColliderHandle>>,
) {
    let context = &mut *context;
    let physics_scale = context.physics_scale();

    for ((entity, shape, sensor, collision_groups, disabled), transform) in colliders.iter() {
        // let mut scaled_shape = shape.clone();
        // scaled_shape.set_scale(shape.scale / physics_scale, config.scaled_shape_subdivision);
        let mut builder = ColliderBuilder::new(shape.raw.clone())
            .user_data(u128::from(entity.to_bits()))
            .sensor(sensor.is_some())
            .enabled(disabled.is_none());

        if let Some(collision_groups) = collision_groups {
            builder = builder.collision_groups((*collision_groups).into());
        }

        let isometry = transform_to_iso(transform, physics_scale);

        builder = builder.position(isometry);
        let handle = context.colliders.insert(builder);

        commands.entity(entity).insert(RapierColliderHandle(handle));
        context.entity2collider.insert(entity, handle);
    }
}

pub fn sync_removals(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,
    mut removed_colliders: RemovedComponents<RapierColliderHandle>,
    orphan_colliders: Query<Entity, (With<RapierColliderHandle>, Without<Collider>)>,
    mut removed_sensors: RemovedComponents<Sensor>,
    mut removed_colliders_disabled: RemovedComponents<ColliderDisabled>,
) {
    /*
     * Collider removal detection.
     */
    let context = &mut *context;
    for entity in removed_colliders.read() {
        if let Some(handle) = context.entity2collider.remove(&entity) {
            debug!("Removing collider for entity {:?}", entity);
            context
                .colliders
                .remove(handle, &mut context.islands, &mut context.bodies, true);
            context.deleted_colliders.insert(handle, entity);
        }
    }

    for entity in orphan_colliders.iter() {
        if let Some(handle) = context.entity2collider.remove(&entity) {
            context
                .colliders
                .remove(handle, &mut context.islands, &mut context.bodies, true);
            context.deleted_colliders.insert(handle, entity);
        }
        commands.entity(entity).remove::<RapierColliderHandle>();
    }

    /*
     * Marker components removal detection.
     */
    for entity in removed_sensors.read() {
        if let Some(handle) = context.entity2collider.get(&entity) {
            if let Some(co) = context.colliders.get_mut(*handle) {
                co.set_sensor(false);
            }
        }
    }

    for entity in removed_colliders_disabled.read() {
        if let Some(handle) = context.entity2collider.get(&entity) {
            if let Some(co) = context.colliders.get_mut(*handle) {
                co.set_enabled(true);
            }
        }
    }
}

fn update_query_pipeline(mut context: ResMut<RapierContext>) {
    context.update_query_pipeline();
}

// pub fn ray_cast_target(
//     rapier_ctx: &RapierContext,
//     origin: Vec2,
//     target: Vec2,
//     buffer_dist: f32,
// ) -> Vec2 {
//     let filter = QueryFilter {
//         groups: Some(CollisionGroups::new(Group::ALL, GROUP_WALL)),
//         ..default()
//     };
//     let ray_origin = origin;
//     let Some(ray_dir) = (target - origin).try_normalize() else {
//         return target;
//     };
//     let max_toi = (target - origin).length() + buffer_dist;
//     if let Some((_, dist)) = rapier_ctx.cast_ray(ray_origin, ray_dir, max_toi, true, filter) {
//         ray_origin + ray_dir * (dist - buffer_dist)
//     } else {
//         target
//     }
// }
