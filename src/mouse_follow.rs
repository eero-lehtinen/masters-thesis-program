use bevy::prelude::*;

pub struct MouseFollowPlugin;

impl Plugin for MouseFollowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FollowMouse>()
            .init_resource::<MousePosition>()
            .add_systems(Update, (toggle, follow));
    }
}

#[derive(Resource, Default)]
struct FollowMouse(bool);

#[derive(Resource, Debug, Default)]
pub struct MousePosition(pub Option<Vec2>);

fn toggle(
    mut follow_mouse: ResMut<FollowMouse>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        follow_mouse.0 = !follow_mouse.0;
    }
}

fn follow(
    follow_mouse: Res<FollowMouse>,
    mut mouse_position: ResMut<MousePosition>,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    if !follow_mouse.0 {
        mouse_position.0 = None;
        return;
    }

    let Ok(window) = window_q.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    mouse_position.0 = window
        .cursor_position()
        .and_then(|cursor_pos| camera.viewport_to_world_2d(camera_transform, cursor_pos));
}
