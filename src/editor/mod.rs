#![allow(clippy::too_many_arguments)]

use std::f32::consts::TAU;
use std::fmt::Write;
use std::fs::File;

use bevy::ecs::query::ReadOnlyWorldQuery;
use bevy::ecs::system::SystemState;
use bevy::utils::HashMap;
use bevy::{ui::*, utils::HashSet};
use enum_map::{enum_map, Enum, EnumMap};

use bevy::prelude::*;
use itertools::Itertools;

use crate::level::*;
use crate::utils::Vertices;
use crate::visualization::draw_gizmo_cross;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<EditingState>()
            .init_resource::<WallVertices>()
            .init_resource::<SelectionArea>()
            .init_resource::<Selected>()
            .init_resource::<EditorInputs>()
            .init_resource::<MousePos>()
            .init_resource::<EditorMouseButtons>()
            .init_resource::<PreviewEntity>()
            .init_resource::<PreviewVertex>()
            .init_resource::<PreviewWall>()
            .configure_sets(
                Update,
                (
                    EditorUpdateSet::Ui,
                    EditorUpdateSet::SetState,
                    EditorUpdateSet::Delete,
                    EditorUpdateSet::Create,
                    EditorUpdateSet::Flush,
                    EditorUpdateSet::Select,
                    EditorUpdateSet::Edit,
                    EditorUpdateSet::Draw,
                    EditorUpdateSet::Last,
                )
                    .chain(),
            )
            .add_systems(PreUpdate, collect_inputs)
            .add_systems(PostUpdate, clear_inputs)
            .add_systems(Startup, init_editor)
            .add_systems(Update, set_editing_state.in_set(EditorUpdateSet::SetState))
            .add_systems(
                Update,
                (
                    delete_previews.run_if(state_changed::<EditingState>()),
                    delete_selection,
                )
                    .in_set(EditorUpdateSet::Delete),
            )
            .add_systems(
                Update,
                (
                    add_wall_vertex_preview.run_if(in_state(EditingState::AddingWallVertex)),
                    create_square_wall_preview.run_if(in_state(EditingState::CreatingSquareWall)),
                    create_circle_wall_preview.run_if(in_state(EditingState::CreatingCircleWall)),
                    create_circle_wall_big_preview
                        .run_if(in_state(EditingState::CreatingBigCircleWall)),
                    create_preview_entity,
                    duplicate_selection,
                )
                    .chain()
                    .in_set(EditorUpdateSet::Create),
            )
            .add_systems(Update, apply_deferred.in_set(EditorUpdateSet::Flush))
            .add_systems(
                Update,
                (handle_area_selection, select_linked).in_set(EditorUpdateSet::Select),
            )
            .add_systems(
                Update,
                (
                    rotate_selection,
                    move_selection,
                    scale_selection,
                    scale_level_size,
                )
                    .in_set(EditorUpdateSet::Edit),
            )
            .add_systems(
                Update,
                (
                    draw_area_selection,
                    draw_wall_lines,
                    draw_selection_marks,
                    update_hide_after,
                )
                    .in_set(EditorUpdateSet::Draw),
            )
            .add_systems(
                Update,
                (update_preview, place_preview, confirm_states, save)
                    .chain()
                    .in_set(EditorUpdateSet::Last),
            );
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum EditorUpdateSet {
    Ui,
    SetState,
    Delete,
    Create,
    Flush,
    Edit,
    Select,
    Draw,
    Last,
}

#[derive(Debug, Enum)]
pub enum EditorAction {
    Save,
    CreateSquareWall,
    CreateCircleWall,
    CreateBigCircleWall,
    CreateEnemySpawn,
    CreatePlayerSpawn,
    AddWallVertex,
    MoveSelection,
    RotateSelection,
    ScaleSelection,
    IncreaseLevelSize,
    DecreaseLevelSize,
    DuplicateSelection,
    DeleteSelection,
    SelectLinked,
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EditingState {
    #[default]
    None,
    Moving,
    Rotating,
    Scaling,
    AddingWallVertex,
    CreatingPlayerSpawn,
    CreatingEnemySpawn,
    CreatingSquareWall,
    CreatingCircleWall,
    CreatingBigCircleWall,
}

#[derive(Resource, Default)]
pub struct WallVertices(pub Vec<Vertices>);

#[derive(Resource, Default, Debug, PartialEq, Eq, Deref, DerefMut)]
struct PreviewEntity(Option<Entity>);

#[derive(Resource, Default)]
struct PreviewVertex(Option<[usize; 2]>);

#[derive(Resource, Default)]
struct PreviewWall(Option<usize>);

#[derive(Debug, Default, Resource, Deref, DerefMut)]
struct EditorInputs(EnumMap<EditorAction, InputState>);

#[derive(Debug, Default, Resource)]
struct EditorMouseButtons {
    confirm: bool,
    cancel: bool,
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
struct MousePos(Vec2);

#[derive(Debug, Default)]
struct InputState {
    just_released: bool,
    just_pressed: bool,
    pressed: bool,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum KeyModifier {
    None,
    Shift,
    Ctrl,
    Alt,
}

pub struct EditorKeyBind {
    pub modifier: KeyModifier,
    pub key: KeyCode,
}

impl From<KeyCode> for EditorKeyBind {
    fn from(key: KeyCode) -> Self {
        EditorKeyBind {
            modifier: KeyModifier::None,
            key,
        }
    }
}

impl From<(KeyModifier, KeyCode)> for EditorKeyBind {
    fn from((modifier, key): (KeyModifier, KeyCode)) -> Self {
        EditorKeyBind { modifier, key }
    }
}

use once_cell::sync::Lazy;

static EDITOR_KEYS: Lazy<EnumMap<EditorAction, (EditorKeyBind, &'static str)>> = Lazy::new(|| {
    use EditorAction::*;
    enum_map! {
        Save => ((KeyModifier::Ctrl, KeyCode::S).into(), "Save level"),
        CreateSquareWall => (KeyCode::Key1.into(), "Create a square wall"),
        CreateCircleWall => (KeyCode::Key2.into(), "Create a circle wall"),
        CreateBigCircleWall => (KeyCode::Key3.into(), "Create a big circle wall"),
        CreateEnemySpawn => (KeyCode::Key9.into(), "Create an enemy spawn"),
        CreatePlayerSpawn => (KeyCode::Key0.into(), "Create a player spawn"),
        AddWallVertex => (KeyCode::W.into(), "Add a wall vertex"),
        MoveSelection => (KeyCode::G.into(), "Move selection"),
        RotateSelection => (KeyCode::R.into(), "Rotate selection"),
        ScaleSelection => (KeyCode::S.into(), "Scale selection"),
        DuplicateSelection => (KeyCode::D.into(), "Duplicate selection"),
        DeleteSelection => (KeyCode::X.into(), "Delete selection"),
        SelectLinked => (KeyCode::L.into(), "Select linked vertices"),
        IncreaseLevelSize => (KeyCode::Period.into(), "Increase level size"),
        DecreaseLevelSize => (KeyCode::Comma.into(), "Decrease level size"),
    }
});

fn collect_inputs(
    inputs: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut editor_inputs: ResMut<EditorInputs>,
    mut editor_mouse_buttons: ResMut<EditorMouseButtons>,
    mut mouse_pos: ResMut<MousePos>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera>>,
) {
    let mut used_keys = HashSet::new();
    for (action, (key_bind, _)) in EDITOR_KEYS.iter() {
        let modifier_pressed = match key_bind.modifier {
            KeyModifier::None => true,
            KeyModifier::Shift => {
                inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight)
            }
            KeyModifier::Ctrl => {
                inputs.pressed(KeyCode::ControlLeft) || inputs.pressed(KeyCode::ControlRight)
            }
            KeyModifier::Alt => {
                inputs.pressed(KeyCode::AltLeft) || inputs.pressed(KeyCode::AltRight)
            }
        };
        if !modifier_pressed {
            continue;
        }
        if !used_keys.insert(key_bind.key) {
            continue;
        }
        let input = &mut editor_inputs[action];
        input.pressed = inputs.pressed(key_bind.key);
        input.just_pressed = inputs.just_pressed(key_bind.key);
        input.just_released = inputs.just_released(key_bind.key);
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_g_transform) = camera.single();
    **mouse_pos = camera
        .viewport_to_world_2d(camera_g_transform, pos)
        .expect("positions shouldn't be nan");
    editor_mouse_buttons.confirm = mouse_buttons.just_pressed(MouseButton::Left);
    editor_mouse_buttons.cancel = mouse_buttons.pressed(MouseButton::Right);
}

fn clear_inputs(mut editor_inputs: ResMut<EditorInputs>) {
    for (_, input) in editor_inputs.0.iter_mut() {
        input.pressed = false;
        input.just_pressed = false;
        input.just_released = false;
    }
}

#[derive(Resource, Default)]
struct Selected(Option<SelectedInner>);

#[derive(Resource, Default, Clone, Debug)]
struct SelectedInner {
    wall_indices: HashSet<[usize; 2]>,
    entities: HashSet<Entity>,
}

impl SelectedInner {
    pub fn from_walls(walls: &[[usize; 2]]) -> Self {
        Self {
            wall_indices: walls.iter().copied().collect(),
            entities: HashSet::default(),
        }
    }
}

#[derive(Resource, Default)]
struct SelectionArea(Option<Rect>);

#[derive(Component)]
struct KeyBindNode;

#[derive(Component)]
struct SaveText;

fn init_editor(
    mut commands: Commands,
    player_spawn_q: Query<Entity, With<Target>>,
    level: Res<Level>,
) {
    commands.insert_resource(LevelSize(level.size));

    commands.insert_resource(WallVertices(level.walls.clone()));

    commands
        .spawn((
            KeyBindNode,
            NodeBundle {
                style: Style {
                    // size: Size::all(Val::Percent(100.0)),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    bottom: Val::Px(0.0),

                    // size: Size::all(Val::Undefined),
                    // margin: UiRect::top(Val::Px(200.0)),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((TextBundle {
                text: Text::from_section(
                    "Key Bindings:".to_owned()
                        + &EDITOR_KEYS.iter().fold(
                            String::new(),
                            |mut output, (_, (key_bind, description))| {
                                let _ = write!(
                                    output,
                                    "\n{}{:?}: {}",
                                    if key_bind.modifier == KeyModifier::None {
                                        String::new()
                                    } else {
                                        format!("{:?} + ", key_bind.modifier)
                                    },
                                    key_bind.key,
                                    description
                                );
                                output
                            },
                        ),
                    TextStyle {
                        font_size: 20.0,
                        color: Color::BLACK,
                        ..default()
                    },
                ),
                ..default()
            },));
        });

    commands.spawn((
        SaveText,
        HideAfter(0.0),
        TextBundle {
            style: Style {
                margin: UiRect::new(Val::Auto, Val::Auto, Val::Auto, Val::Px(10.0)),
                ..default()
            },
            text: Text::from_section(
                String::new(),
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            ..default()
        },
    ));

    if player_spawn_q.iter().count() == 0 {
        commands.spawn(TargetBundle::new(Vec2::ZERO));
    }
}

fn set_editing_state(
    editor_inputs: Res<EditorInputs>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
) {
    #[rustfmt::skip]
	let state_transitions = [
		(EditorAction::CreateSquareWall, EditingState::CreatingSquareWall),
		(EditorAction::CreateCircleWall, EditingState::CreatingCircleWall),
		(EditorAction::CreateBigCircleWall, EditingState::CreatingBigCircleWall),
		(EditorAction::CreateEnemySpawn, EditingState::CreatingEnemySpawn),
		(EditorAction::CreatePlayerSpawn, EditingState::CreatingPlayerSpawn),
		(EditorAction::AddWallVertex, EditingState::AddingWallVertex),
		(EditorAction::MoveSelection, EditingState::Moving),
		(EditorAction::RotateSelection, EditingState::Rotating),
		(EditorAction::ScaleSelection, EditingState::Scaling),
		(EditorAction::DuplicateSelection, EditingState::None),
		(EditorAction::DeleteSelection, EditingState::None),
		(EditorAction::SelectLinked, EditingState::None),
		(EditorAction::Save, EditingState::None),
	];

    for (action, state) in state_transitions {
        if editor_inputs[action].just_pressed {
            next_editing_state.set(state);
        }
    }
}

fn create_preview_entity(
    mut commands: Commands,
    editing_state: Res<State<EditingState>>,
    mut preview_entity: ResMut<PreviewEntity>,
    mouse_pos: Res<MousePos>,
) {
    if preview_entity.is_some() {
        return;
    }
    match *editing_state.get() {
        EditingState::CreatingPlayerSpawn => {
            preview_entity.0 = Some(commands.spawn(TargetBundle::new(**mouse_pos)).id());
        }
        EditingState::CreatingEnemySpawn => {
            preview_entity.0 = Some(commands.spawn(SpawnPointBundle::new(**mouse_pos)).id());
        }
        _ => {}
    }
}

fn delete_previews(world: &mut World) {
    let mut system_state: SystemState<(
        ResMut<PreviewEntity>,
        ResMut<PreviewVertex>,
        ResMut<PreviewWall>,
        ResMut<WallVertices>,
    )> = SystemState::new(world);

    let (mut preview_entity, mut preview_vertex, mut preview_wall, mut wall_vertices) =
        system_state.get_mut(world);

    let preview_entity = preview_entity.take();

    if let Some([w_i, v_i]) = preview_vertex.0.take() {
        wall_vertices.0[w_i].remove(v_i);
        if wall_vertices.0[w_i].is_empty() {
            wall_vertices.0.remove(w_i);
        }
    }

    if let Some(w_i) = preview_wall.0.take() {
        wall_vertices.0.remove(w_i);
    }

    if let Some(e) = preview_entity {
        world.entity_mut(e).despawn_recursive();
    }
}

fn update_preview(
    mut tr_q: Query<&mut Transform>,
    preview_entity: Res<PreviewEntity>,
    mouse_pos: Res<MousePos>,
    preview_vertex: Res<PreviewVertex>,
    preview_wall: Res<PreviewWall>,
    mut wall_vertices: ResMut<WallVertices>,
) {
    if let Some(e) = preview_entity.0 {
        if let Ok(mut transform) = tr_q.get_mut(e) {
            transform.translation.x = mouse_pos.0.x;
            transform.translation.y = mouse_pos.0.y;
        }
    }

    if let Some([w_i, v_i]) = preview_vertex.0 {
        wall_vertices.0[w_i][v_i] = **mouse_pos;
    }

    if let Some(w_i) = preview_wall.0 {
        let center = wall_center(&wall_vertices.0[w_i]);
        let diff = **mouse_pos - center;
        for v in wall_vertices.0[w_i].iter_mut() {
            *v += diff;
        }
    }
}

fn place_preview(
    mut commands: Commands,
    editor_mouse_buttons: Res<EditorMouseButtons>,
    mut preview_entity: ResMut<PreviewEntity>,
    mut preview_vertex: ResMut<PreviewVertex>,
    mut preview_wall: ResMut<PreviewWall>,
    wall_vertices: Res<WallVertices>,
    mut selected: ResMut<Selected>,
    player_spawn_q: Query<Entity, With<Target>>,
) {
    if editor_mouse_buttons.confirm {
        // Ensure invariant that there is only one spawn
        if let Some(new_e) = preview_entity.0 {
            if player_spawn_q.get(new_e).is_ok() {
                for e in player_spawn_q.iter().filter(|e| *e != new_e) {
                    commands.entity(e).despawn_recursive();
                }
            }
        }
        preview_entity.0 = None;
        if let Some(index) = preview_vertex.0.take() {
            selected.0 = Some(SelectedInner::from_walls(&[index]));
        }
        if let Some(wall_index) = preview_wall.0.take() {
            let indices = (0..wall_vertices.0[wall_index].len())
                .map(|i| [wall_index, i])
                .collect_vec();
            selected.0 = Some(SelectedInner::from_walls(&indices));
        }
    }
}

fn confirm_states(
    editor_mouse_buttons: Res<EditorMouseButtons>,
    editing_state: Res<State<EditingState>>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
) {
    if editor_mouse_buttons.cancel {
        next_editing_state.set(EditingState::None);
    }
    if editor_mouse_buttons.confirm && *editing_state != EditingState::AddingWallVertex {
        next_editing_state.set(EditingState::None);
    }
}

const SQUARE_SIZE: f32 = 10.0;

fn create_square_wall_preview(
    mouse_pos: Res<MousePos>,
    mut wall_vertices: ResMut<WallVertices>,
    mut preview_wall: ResMut<PreviewWall>,
) {
    if preview_wall.0.is_none() {
        let center = **mouse_pos;
        let half = SQUARE_SIZE / 2.0;
        wall_vertices.0.push(vec![
            center + Vec2::new(-half, -half),
            center + Vec2::new(-half, half),
            center + Vec2::new(half, half),
            center + Vec2::new(half, -half),
        ]);
        preview_wall.0 = Some(wall_vertices.0.len() - 1);
    }
}

const CIRCLE_SEGMENTS: usize = 32;
const CIRCLE_SIZE: f32 = 10.0;
const CIRCLE_SEGMENTS_BIG: usize = 80;
const CIRCLE_SIZE_BIG: f32 = 30.0;

fn make_circle(center: Vec2, radius: f32, segments: usize) -> Vec<Vec2> {
    let mut vertices = Vec::with_capacity(segments);
    for i in 0..segments {
        let angle = -(i as f32 / segments as f32) * std::f32::consts::TAU;
        vertices.push(center + Vec2::new(angle.cos(), angle.sin()) * radius);
    }
    vertices
}

fn create_circle_wall_preview(
    mouse_pos: Res<MousePos>,
    mut wall_vertices: ResMut<WallVertices>,
    mut preview_wall: ResMut<PreviewWall>,
) {
    if preview_wall.0.is_none() {
        let vertices = make_circle(**mouse_pos, CIRCLE_SIZE, CIRCLE_SEGMENTS);
        wall_vertices.0.push(vertices);
        preview_wall.0 = Some(wall_vertices.0.len() - 1);
    }
}

fn create_circle_wall_big_preview(
    mouse_pos: Res<MousePos>,
    mut wall_vertices: ResMut<WallVertices>,
    mut preview_wall: ResMut<PreviewWall>,
) {
    if preview_wall.0.is_none() {
        let vertices = make_circle(**mouse_pos, CIRCLE_SIZE_BIG, CIRCLE_SEGMENTS_BIG);
        wall_vertices.0.push(vertices);
        preview_wall.0 = Some(wall_vertices.0.len() - 1);
    }
}

fn rotate_selection(
    editor_inputs: Res<EditorInputs>,
    mouse_pos: Res<MousePos>,
    editor_mouse_buttons: Res<EditorMouseButtons>,
    mut wall_vertices: ResMut<WallVertices>,
    mut selected: ResMut<Selected>,
    mut transforms: Query<&mut Transform>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
    editing_state: Res<State<EditingState>>,
    mut start_rot: Local<Option<Vec2>>,
    mut last_rot: Local<Vec2>,
) {
    let Some(selected) = &mut selected.0 else {
        return;
    };

    if editor_inputs[EditorAction::RotateSelection].just_pressed {
        next_editing_state.set(EditingState::Rotating);
        return;
    }
    let mut reset = false;
    if *editing_state != EditingState::Rotating {
        if start_rot.is_none() {
            return;
        }
        reset = true;
    }
    if editor_mouse_buttons.confirm {
        *start_rot = None;
        return;
    }

    let get_center = |selected: &SelectedInner| {
        let mut points = Vec::new();
        for e in selected.entities.iter() {
            let Ok(transform) = transforms.get(*e) else {
                continue;
            };
            points.push(transform.translation.truncate());
        }

        for [w_i, v_i] in selected.wall_indices.iter() {
            points.push(wall_vertices.0[*w_i][*v_i]);
        }

        points.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / points.len() as f32
    };

    if start_rot.is_none() {
        let center = get_center(selected);
        *last_rot = (**mouse_pos - center).normalize();
        *start_rot = Some(*last_rot);
    }

    let center = get_center(selected);

    let angle_diff = if !reset {
        let d = last_rot.angle_between(**mouse_pos - center);
        // Too small angles make inaccuracies in further rotations.
        // Don't update last_rot to accumulate small movements.
        if d.abs() < TAU / 10000.0 {
            return;
        }
        d
    } else {
        (**mouse_pos - center).angle_between(start_rot.take().expect("start_rot should be Some"))
    };

    let vec_angle = Vec2::from_angle(angle_diff);

    for e in selected.entities.iter() {
        let Ok(mut transform) = transforms.get_mut(*e) else {
            continue;
        };

        let v = transform.translation.truncate() - center;
        transform.translation = (center + vec_angle.rotate(v)).extend(0.0);
    }

    for [w_i, v_i] in selected.wall_indices.iter() {
        let v = wall_vertices.0[*w_i][*v_i] - center;
        wall_vertices.0[*w_i][*v_i] = center + vec_angle.rotate(v);
    }
    *last_rot = (**mouse_pos - center).normalize();
}

fn move_selection(
    editor_inputs: Res<EditorInputs>,
    mouse_pos: Res<MousePos>,
    editor_mouse_buttons: Res<EditorMouseButtons>,
    mut wall_vertices: ResMut<WallVertices>,
    mut selected: ResMut<Selected>,
    mut transforms: Query<&mut Transform>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
    editing_state: Res<State<EditingState>>,
    mut start_mouse_pos: Local<Option<Vec2>>,
    mut last_mouse_pos: Local<Vec2>,
) {
    let Some(selected) = &mut selected.0 else {
        return;
    };

    if editor_inputs[EditorAction::MoveSelection].just_pressed {
        next_editing_state.set(EditingState::Moving);
    }
    let mut reset = false;
    if *editing_state.get() != EditingState::Moving {
        if start_mouse_pos.is_none() {
            return;
        }
        reset = true;
    }
    if editor_mouse_buttons.confirm {
        *start_mouse_pos = None;
        return;
    }

    if start_mouse_pos.is_none() {
        *start_mouse_pos = Some(**mouse_pos);
        *last_mouse_pos = **mouse_pos;
    }

    let diff = if !reset {
        **mouse_pos - *last_mouse_pos
    } else {
        start_mouse_pos
            .take()
            .expect("start_mouse_pos should be Some")
            - **mouse_pos
    };

    for [w_i, v_i] in selected.wall_indices.iter() {
        wall_vertices.0[*w_i][*v_i] += diff;
    }

    for e in selected.entities.iter() {
        let Ok(mut transform) = transforms.get_mut(*e) else {
            continue;
        };
        transform.translation += diff.extend(0.0);
    }

    *last_mouse_pos = **mouse_pos;
}

fn wall_center(wall: &[Vec2]) -> Vec2 {
    wall.iter().fold(Vec2::ZERO, |acc, &v| acc + v) / wall.len() as f32
}

fn scale_selection(
    editor_inputs: Res<EditorInputs>,
    mouse_pos: Res<MousePos>,
    mut wall_vertices: ResMut<WallVertices>,
    mut selected: ResMut<Selected>,
    mut transforms: Query<&mut Transform>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
    editing_state: Res<State<EditingState>>,
    editor_mouse_buttons: Res<EditorMouseButtons>,
    mut start_mouse_pos: Local<Option<Vec2>>,
    mut last_mouse_pos: Local<Vec2>,
) {
    let Some(selected) = &mut selected.0 else {
        return;
    };

    if editor_inputs[EditorAction::ScaleSelection].just_pressed {
        next_editing_state.set(EditingState::Scaling);
    }
    let mut reset = false;
    if *editing_state.get() != EditingState::Scaling {
        if start_mouse_pos.is_none() {
            return;
        }
        reset = true;
    }
    if editor_mouse_buttons.confirm {
        *start_mouse_pos = None;
        return;
    }

    if start_mouse_pos.is_none() {
        *start_mouse_pos = Some(**mouse_pos);
        *last_mouse_pos = **mouse_pos;
    }

    let mut points = Vec::new();
    for e in selected.entities.iter() {
        let Ok(transform) = transforms.get(*e) else {
            continue;
        };
        points.push(transform.translation.truncate());
    }
    for [w_i, v_i] in selected.wall_indices.iter() {
        points.push(wall_vertices.0[*w_i][*v_i]);
    }
    let center = points.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / points.len() as f32;

    let scale = if !reset {
        (**mouse_pos - center).length() / (*last_mouse_pos - center).length()
    } else {
        (start_mouse_pos.take().unwrap() - center).length() / (**mouse_pos - center).length()
    };

    for e in selected.entities.iter() {
        let Ok(mut transform) = transforms.get_mut(*e) else {
            continue;
        };
        let pos = transform.translation.truncate();
        let v = pos - center;
        transform.translation = (center + v * scale).extend(0.0);
        // transform.scale *= scale; // Dunno if should scale
    }

    for [w_i, v_i] in selected.wall_indices.iter() {
        let v = wall_vertices.0[*w_i][*v_i] - center;
        wall_vertices.0[*w_i][*v_i] = center + v * scale;
    }

    *last_mouse_pos = **mouse_pos;
}

fn scale_level_size(mut level_size: ResMut<LevelSize>, editor_inputs: Res<EditorInputs>) {
    if editor_inputs[EditorAction::IncreaseLevelSize].just_pressed {
        level_size.0 += 10.0;
        info!("Level size: {}", level_size.0);
    }
    if editor_inputs[EditorAction::DecreaseLevelSize].just_pressed {
        if level_size.0 < 15.0 {
            return;
        }
        level_size.0 -= 10.0;
        info!("Level size: {}", level_size.0);
    }
}

fn add_wall_vertex_preview(
    mouse_pos: Res<MousePos>,
    mut wall_vertices: ResMut<WallVertices>,
    selected: Res<Selected>,
    mut next_editing_state: ResMut<NextState<EditingState>>,
    mut preview_vertex: ResMut<PreviewVertex>,
) {
    if preview_vertex.0.is_none() {
        match &selected.0 {
            None => {
                wall_vertices.0.push(vec![**mouse_pos]);
                preview_vertex.0 = Some([wall_vertices.0.len() - 1, 0]);
            }
            Some(selected) => {
                if selected.wall_indices.len() != 1 || !selected.entities.is_empty() {
                    // TODO: Warn user that we cant add vertices in this position
                    next_editing_state.set(EditingState::None);
                    return;
                }
                let [w_i, v_i] = *selected.wall_indices.iter().next().unwrap();
                let wall = &mut wall_vertices.0[w_i];
                wall.insert(v_i + 1, **mouse_pos);
                preview_vertex.0 = Some([w_i, v_i + 1]);
            }
        }
    }
}

fn duplicate_selection(
    mut commands: Commands,
    editor_inputs: Res<EditorInputs>,
    mut selected: ResMut<Selected>,
    transform_q: Query<(Entity, &Transform, Has<SpawnPoint>, Has<Target>)>,
    mut wall_vertices: ResMut<WallVertices>,
    mouse_pos: Res<MousePos>,
) {
    if !editor_inputs[EditorAction::DuplicateSelection].just_pressed {
        return;
    }

    let sel = match &selected.0 {
        None => return,
        Some(selected) => selected.clone(),
    };

    let mut points = transform_q
        .iter()
        .filter_map(|(e, t, _, _)| sel.entities.contains(&e).then_some(t))
        .map(|t| t.translation.truncate())
        .collect_vec();

    for [w_i, v_i] in sel.wall_indices.iter() {
        points.push(wall_vertices.0[*w_i][*v_i]);
    }

    let center = points.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / points.len() as f32;

    let diff = **mouse_pos - center;

    let mut new_sel = SelectedInner {
        entities: HashSet::new(),
        wall_indices: HashSet::new(),
    };

    for (_, transform, is_spawn, is_target) in
        transform_q.iter().filter(|q| sel.entities.contains(&q.0))
    {
        let mut t = *transform;
        t.translation += diff.extend(0.);
        let mut e = if is_spawn {
            commands.spawn(SpawnPointBundle::new(Vec2::ZERO))
        } else if is_target {
            commands.spawn(TargetBundle::new(Vec2::ZERO))
        } else {
            unreachable!();
        };

        let entity = e.insert(t).id();

        new_sel.entities.insert(entity);
    }

    let mut new_walls: HashMap<usize, Vec<Vec2>> = HashMap::new();

    for &[w_i, v_i] in sel.wall_indices.iter().sorted() {
        let v = wall_vertices.0[w_i][v_i] + diff;
        new_walls.entry(w_i).or_default().push(v);
    }

    for (_, wall) in new_walls {
        new_sel
            .wall_indices
            .extend((0..wall.len()).map(|i| [wall_vertices.0.len(), i]));
        wall_vertices.0.push(wall);
    }

    selected.0 = Some(new_sel);
}

fn delete_selection(
    mut commands: Commands,
    editor_inputs: Res<EditorInputs>,
    mut wall_vertices: ResMut<WallVertices>,
    mut selected: ResMut<Selected>,
) {
    if !editor_inputs[EditorAction::DeleteSelection].just_pressed {
        return;
    }

    if let Some(selected) = &selected.0 {
        let mut sorted = selected.wall_indices.iter().collect::<Vec<_>>();
        sorted.sort_unstable();
        sorted.reverse();
        for [w_i, v_i] in &sorted {
            wall_vertices.0[*w_i].remove(*v_i);
        }
        wall_vertices.0.retain(|w| !w.is_empty());

        for e in selected.entities.iter() {
            if let Some(e) = commands.get_entity(*e) {
                e.despawn_recursive();
            }
        }
    }

    selected.0 = None;
}

fn select_linked(
    editor_inputs: Res<EditorInputs>,
    wall_vertices: Res<WallVertices>,
    mut selected: ResMut<Selected>,
) {
    if !editor_inputs[EditorAction::SelectLinked].just_pressed {
        return;
    }

    let Some(sel) = &mut selected.0 else {
        return;
    };

    let mut selected_walls = HashSet::new();
    for [w_i, _] in sel.wall_indices.iter() {
        selected_walls.insert(*w_i);
    }

    for w_i in 0..wall_vertices.0.len() {
        if !selected_walls.contains(&w_i) {
            continue;
        }
        sel.wall_indices
            .extend((0..wall_vertices.0[w_i].len()).map(|v_i| [w_i, v_i]));
    }
}

fn draw_area_selection(mut gizmos: Gizmos, selection_area: Res<SelectionArea>) {
    if let Some(area) = &selection_area.0 {
        gizmos.rect_2d(area.center(), 0., area.size(), Color::WHITE);
    }
}

fn draw_wall_lines(
    mut gizmos: Gizmos,
    wall_vertices: Res<WallVertices>,
    selected: Res<Selected>,
    projection: Query<&OrthographicProjection, With<Camera>>,
) {
    let projection = projection.single();
    let dot_size = 0.6 * projection.scale;

    let selected = selected.into_inner();
    let mut hue = 200.0;

    for (w_i, vertices) in wall_vertices.0.iter().enumerate() {
        for ((i1, &v1), &v2) in vertices
            .iter()
            .enumerate()
            .zip(vertices.iter().cycle().skip(1))
        {
            let selected = match &selected.0 {
                None => false,
                Some(selected) => selected.wall_indices.contains(&[w_i, i1]),
            };
            let color = if selected {
                Color::ORANGE
            } else {
                Color::hsl(hue, 0.8, 0.7)
            };
            gizmos.line_2d(v1, v2, color);
        }
        hue += 160.0 / wall_vertices.0.len() as f32;
    }

    for (w_i, vertices) in wall_vertices.0.iter().enumerate() {
        for (i, &v) in vertices.iter().enumerate() {
            let selected = match &selected.0 {
                None => false,
                Some(selected) => selected.wall_indices.contains(&[w_i, i]),
            };

            // Draw "dots" with small lines
            let color = if selected {
                Color::ORANGE
            } else {
                Color::WHITE
            };
            draw_gizmo_cross(&mut gizmos, v, color, dot_size);
        }
        hue += 340.0 / wall_vertices.0.len() as f32;
    }
}

fn draw_selection_marks(
    mut gizmos: Gizmos,
    selected: Res<Selected>,
    transforms: Query<&mut Transform>,
) {
    let Some(selected) = &selected.0 else {
        return;
    };

    for e in selected.entities.iter() {
        let Ok(transform) = transforms.get(*e) else {
            continue;
        };
        gizmos.rect_2d(
            transform.translation.truncate(),
            0.,
            Vec2::new(2., 2.),
            Color::ORANGE,
        );
        gizmos.rect_2d(
            transform.translation.truncate(),
            0.,
            Vec2::new(1.8, 1.8),
            Color::BLACK,
        );
    }
}

const RECT_SELECTION_TRESHOLD: f32 = 1.0;
const POINT_SELECTION_DISTANCE_TRESHOLD: f32 = 5.0;

fn handle_area_selection(
    mouse_pos: Res<MousePos>,
    mouse_input: Res<Input<MouseButton>>,
    input: Res<Input<KeyCode>>,
    mut selection_area_start: Local<Vec2>,
    mut selection_area: ResMut<SelectionArea>,
    wall_vertices: Res<WallVertices>,
    mut selected: ResMut<Selected>,
    transforms: Query<(Entity, &Transform), Or<(With<Target>, With<SpawnPoint>)>>,
    editing_state: Res<State<EditingState>>,
) {
    if *editing_state != EditingState::None {
        selection_area.0 = None;
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        *selection_area_start = **mouse_pos;
        selection_area.0 = Some(Rect::from_corners(**mouse_pos, **mouse_pos));
    }

    if let Some(area) = &mut selection_area.0 {
        *area = Rect::from_corners(*selection_area_start, **mouse_pos);
    }

    if mouse_input.just_released(MouseButton::Left) {
        let Some(area) = &selection_area.0 else {
            return;
        };

        let append = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);

        let mut wall_selected = if append {
            match &selected.0 {
                None => HashSet::new(),
                Some(selected) => selected.wall_indices.clone(),
            }
        } else {
            HashSet::new()
        };

        let rect = area;
        let use_rect = rect.size().length() > RECT_SELECTION_TRESHOLD;

        let mut closest_dist = POINT_SELECTION_DISTANCE_TRESHOLD.powi(2);
        let mut best = None;

        for (w_i, wall) in wall_vertices.0.iter().enumerate() {
            for (v_i, vertex) in wall.iter().enumerate() {
                if use_rect {
                    if rect.contains(*vertex) {
                        wall_selected.insert([w_i, v_i]);
                    }
                } else {
                    let dist = mouse_pos.0.distance_squared(*vertex);
                    if dist < closest_dist {
                        closest_dist = dist;
                        best = Some([w_i, v_i]);
                    }
                }
            }
        }

        if let Some(best) = best {
            if wall_selected.contains(&best) {
                wall_selected.remove(&best);
            } else {
                wall_selected.insert(best);
            }
        }

        let mut entities_selected = if append {
            match &selected.0 {
                None => HashSet::new(),
                Some(selected) => selected.entities.clone(),
            }
        } else {
            HashSet::new()
        };

        let mut best = None;
        for (entity, transform) in transforms.iter() {
            if use_rect {
                if rect.contains(transform.translation.truncate()) {
                    entities_selected.insert(entity);
                }
            } else {
                let dist = (**mouse_pos).distance_squared(transform.translation.truncate());
                if dist < closest_dist {
                    closest_dist = dist;
                    best = Some(entity);
                }
            }
        }

        if let Some(best) = best {
            entities_selected.insert(best);
        }

        if wall_selected.is_empty() && entities_selected.is_empty() {
            selected.0 = None;
        } else {
            selected.0 = Some(SelectedInner {
                wall_indices: wall_selected,
                entities: entities_selected,
            });
        }

        selection_area.0 = None;
    }
}

#[derive(Component)]
struct HideAfter(f32);

fn update_hide_after(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut HideAfter, &mut Visibility)>,
) {
    for (mut hide_after, mut visibility) in query.iter_mut() {
        hide_after.0 -= time.delta_seconds();
        if hide_after.0 <= 0.0 {
            *visibility = Visibility::Hidden;
        }
    }
}

fn get_positions<F: ReadOnlyWorldQuery>(world: &mut World) -> Vec<Vec2> {
    world
        .query_filtered::<&Transform, F>()
        .iter(world)
        .map(|t| t.translation.truncate())
        .collect::<Vec<_>>()
}

pub fn save_level(world: &mut World, name: &str) -> anyhow::Result<()> {
    let size = world.resource::<LevelSize>().0;
    let walls = world.resource::<WallVertices>().0.clone();
    let targets = get_positions::<With<Target>>(world);
    let spawn_points = get_positions::<With<SpawnPoint>>(world);

    let level = Level {
        size,
        spawn_points,
        targets,
        walls,
    };

    let mut file = File::create(name)?;
    rmp_serde::encode::write_named(&mut file, &level)?;
    Ok(())
}

fn save(world: &mut World) {
    let inputs = world.resource::<EditorInputs>();
    if inputs[EditorAction::Save].just_pressed {
        delete_previews(world);

        let name = world
            .get_resource::<LevelPath>()
            .map_or("unnamed.level".to_string(), |p| p.0.clone());

        match save_level(world, &name) {
            Ok(()) => {
                world
                    .query_filtered::<(&mut HideAfter, &mut Visibility, &mut Text), With<SaveText>>(
                    )
                    .for_each_mut(world, |(mut hide_after, mut vis, mut text)| {
                        hide_after.0 = 2.0;
                        *vis = Visibility::Visible;
                        text.sections[0].value = format!("Saved as '{name}'");
                    });
            }
            Err(e) => {
                world
                    .query_filtered::<(&mut HideAfter, &mut Visibility, &mut Text), With<SaveText>>(
                    )
                    .for_each_mut(world, |(mut hide_after, mut vis, mut text)| {
                        hide_after.0 = 2.0;
                        *vis = Visibility::Visible;
                        text.sections[0].value = format!("Failed to save level: {e}");
                    });
            }
        }
    }
}
