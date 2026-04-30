//! # Road Placement Example
//!
//! Demonstrates interactive Bezier spline editing in Bevy, replicating the
//! Unity gizmo-based road placement demo from
//! <https://github.com/sunsided/unity-bezier-splines>.
//!
//! ## Controls
//!
//! | Action | Input |
//! |--------|-------|
//! | Drag node center | Left-click on a gray cross, then move mouse |
//! | Drag control handle | Left-click on a blue/red cross, then move mouse |
//! | Toggle closed path | `C` |
//! | Add waypoint at end | `A` |
//! | Remove last waypoint | `R` |
//! | Reset to default | `Space` |
//! | Orbit camera | Right-click drag |
//! | Zoom | Scroll wheel |

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
use bevy_bezier_splines::{BezierPath, BezierPathNode, BezierSplinesPlugin, GizmoDrawMode, NodeType};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Bezier Splines – Road Placement".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BezierSplinesPlugin)
        .init_resource::<InteractionState>()
        .init_resource::<CameraState>()
        .add_systems(Startup, (setup_scene, spawn_default_road))
        .add_systems(
            Update,
            (
                update_camera_orbit,
                pick_handle,
                drag_handle,
                release_handle,
            )
                .chain(),
        )
        .add_systems(Update, (keyboard_controls, update_status_text))
        .run();
}

// ---------------------------------------------------------------------------
// Components & resources
// ---------------------------------------------------------------------------

/// Marks the status-text UI entity.
#[derive(Component)]
struct StatusText;

/// Which part of a node is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleKind {
    Center,
    Incoming,
    Outgoing,
}

/// Which node + handle is currently being dragged, plus the Y plane used.
#[derive(Resource, Default)]
struct InteractionState {
    dragging: Option<(Entity, HandleKind)>,
    drag_plane_y: f32,
}

/// Camera orbit state.
#[derive(Resource)]
struct CameraState {
    yaw: f32,
    pitch: f32,
    distance: f32,
    focus: Vec3,
    /// Tracks whether we are in an orbit drag (right-button held).
    orbiting: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.3,
            pitch: 1.0,
            distance: 18.0,
            focus: Vec3::ZERO,
            orbiting: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_state: Res<CameraState>,
) {
    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.28, 0.18),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    // Camera
    let cam_pos = orbit_position(&camera_state);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(cam_pos).looking_at(camera_state.focus, Vec3::Y),
    ));

    // UI status text
    commands.spawn((
        Text::new(status_text_content(false)),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));
}

fn spawn_default_road(mut commands: Commands) {
    spawn_road(
        &mut commands,
        false,
        &[
            (Vec3::new(-6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.5), Vec3::new(0.0, 0.0, 1.5)),
            (Vec3::new(-2.0, 0.0, 3.0), Vec3::new(-1.5, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0)),
            (Vec3::new(2.0, 0.0, -3.0), Vec3::new(-1.5, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0)),
            (Vec3::new(6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.5), Vec3::new(0.0, 0.0, 1.5)),
        ],
    );
}

/// Spawn a [`BezierPath`] entity with child [`BezierPathNode`] entities.
/// `nodes` is `(center, incoming_offset, outgoing_offset)`.
fn spawn_road(commands: &mut Commands, closed: bool, nodes: &[(Vec3, Vec3, Vec3)]) {
    let path_entity = commands
        .spawn((
            BezierPath {
                subdivisions: 30,
                closed,
                gizmo_draw_mode: GizmoDrawMode::Complete,
            },
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    for &(center, incoming, outgoing) in nodes {
        let node_entity = commands
            .spawn((
                BezierPathNode { incoming, outgoing, node_type: NodeType::Connected },
                Transform::from_translation(center),
                Visibility::default(),
            ))
            .id();
        commands.entity(path_entity).add_child(node_entity);
    }
}

fn status_text_content(closed: bool) -> String {
    format!(
        "Bevy Bezier Splines – Road Placement\n\
         Left-click & drag: move a node or control handle\n\
         Right-click drag: orbit  |  Scroll: zoom\n\
         [A] Add  [R] Remove last  [C] Toggle closed ({})  [Space] Reset",
        if closed { "ON" } else { "OFF" }
    )
}

// ---------------------------------------------------------------------------
// Camera orbit
// ---------------------------------------------------------------------------

fn orbit_position(state: &CameraState) -> Vec3 {
    let x = state.distance * state.pitch.sin() * state.yaw.sin();
    let y = state.distance * state.pitch.cos();
    let z = state.distance * state.pitch.sin() * state.yaw.cos();
    state.focus + Vec3::new(x, y, z)
}

fn update_camera_orbit(
    mut camera_state: ResMut<CameraState>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    interaction: Res<InteractionState>,
) {
    // Orbit with right-click drag – only when not dragging a spline handle.
    if mouse_button.pressed(MouseButton::Right) && interaction.dragging.is_none() {
        camera_state.orbiting = true;
        camera_state.yaw -= mouse_motion.delta.x * 0.005;
        camera_state.pitch = (camera_state.pitch + mouse_motion.delta.y * 0.005)
            .clamp(0.1, std::f32::consts::PI - 0.1);
    } else {
        camera_state.orbiting = false;
    }

    // Zoom
    let scroll_delta = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y * 0.8,
        MouseScrollUnit::Pixel => scroll.delta.y * 0.02,
    };
    camera_state.distance = (camera_state.distance - scroll_delta).clamp(2.0, 80.0);

    let pos = orbit_position(&camera_state);
    if let Ok(mut tf) = camera_query.single_mut() {
        *tf = Transform::from_translation(pos).looking_at(camera_state.focus, Vec3::Y);
    }
}

// ---------------------------------------------------------------------------
// Handle picking and dragging
// ---------------------------------------------------------------------------

/// Cast a ray from the camera through the cursor position.
fn cursor_ray(window: &Window, camera_tf: &GlobalTransform, camera: &Camera) -> Option<Ray3d> {
    let cursor = window.cursor_position()?;
    camera.viewport_to_world(camera_tf, cursor).ok()
}

/// Intersect a ray with the horizontal plane at height `y`.
fn ray_plane_y(ray: &Ray3d, y: f32) -> Option<Vec3> {
    let denom = ray.direction.y;
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (y - ray.origin.y) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + *ray.direction * t)
}

/// Collect all pick targets (center + both handles) for every node in every path.
fn collect_handles(
    nodes: &Query<(&BezierPathNode, &GlobalTransform)>,
    paths: &Query<(&BezierPath, &Children)>,
) -> Vec<(Entity, HandleKind, Vec3)> {
    let mut targets = Vec::new();
    for (_path, children) in paths.iter() {
        for child in children.iter() {
            if let Ok((node, tf)) = nodes.get(child) {
                let center = tf.translation();
                targets.push((child, HandleKind::Center, center));
                targets.push((child, HandleKind::Incoming, center + node.incoming));
                targets.push((child, HandleKind::Outgoing, center + node.outgoing));
            }
        }
    }
    targets
}

fn pick_handle(
    mut interaction: ResMut<InteractionState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
    paths: Query<(&BezierPath, &Children)>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) || interaction.dragging.is_some() {
        return;
    }

    let window = windows.single().unwrap();
    let (camera, cam_tf) = camera_query.single().unwrap();

    let handles = collect_handles(&nodes, &paths);

    let mut best: Option<(f32, Entity, HandleKind, f32)> = None;
    for (entity, kind, world_pos) in &handles {
        let Some(ndc) = camera.world_to_ndc(cam_tf, *world_pos) else { continue };
        if ndc.z < 0.0 || ndc.z > 1.0 { continue; }

        let vp = camera
            .logical_viewport_size()
            .unwrap_or(Vec2::new(window.width(), window.height()));
        let screen = Vec2::new(
            (ndc.x * 0.5 + 0.5) * vp.x,
            (1.0 - (ndc.y * 0.5 + 0.5)) * vp.y,
        );
        let cursor_pos = window.cursor_position().unwrap_or_default();
        let dist = (screen - cursor_pos).length();

        const PICK_RADIUS_PX: f32 = 18.0;
        if dist < PICK_RADIUS_PX && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, *entity, *kind, world_pos.y));
        }
    }

    if let Some((_, entity, kind, plane_y)) = best {
        interaction.dragging = Some((entity, kind));
        interaction.drag_plane_y = plane_y;
    }
}

fn drag_handle(
    interaction: Res<InteractionState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut nodes: Query<(&mut BezierPathNode, &mut Transform)>,
) {
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }
    let Some((entity, kind)) = interaction.dragging else { return };

    let window = windows.single().unwrap();
    let (camera, cam_tf) = camera_query.single().unwrap();
    let Some(ray) = cursor_ray(window, cam_tf, camera) else { return };
    let Some(world_pos) = ray_plane_y(&ray, interaction.drag_plane_y) else { return };

    let Ok((mut node, mut tf)) = nodes.get_mut(entity) else { return };

    match kind {
        HandleKind::Center => {
            tf.translation = Vec3::new(world_pos.x, interaction.drag_plane_y, world_pos.z);
        }
        HandleKind::Incoming => {
            node.incoming = world_pos - tf.translation;
            node.update_from_incoming();
        }
        HandleKind::Outgoing => {
            node.outgoing = world_pos - tf.translation;
            node.update_from_outgoing();
        }
    }
}

fn release_handle(
    mut interaction: ResMut<InteractionState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        interaction.dragging = None;
    }
}

// ---------------------------------------------------------------------------
// Keyboard controls
// ---------------------------------------------------------------------------

fn keyboard_controls(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut paths: Query<(Entity, &mut BezierPath, &Children)>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
) {
    let Ok((path_entity, mut path, children)) = paths.single_mut() else { return };

    if keys.just_pressed(KeyCode::KeyC) {
        path.closed = !path.closed;
    }

    if keys.just_pressed(KeyCode::KeyA) {
        let last_pos = children
            .iter()
            .last()
            .and_then(|c| nodes.get(c).ok())
            .map(|(_, tf)| tf.translation())
            .unwrap_or(Vec3::ZERO);

        let node = commands
            .spawn((
                BezierPathNode::default(),
                Transform::from_translation(last_pos + Vec3::new(2.0, 0.0, 0.0)),
                Visibility::default(),
            ))
            .id();
        commands.entity(path_entity).add_child(node);
    }

    if keys.just_pressed(KeyCode::KeyR) && children.len() > 2 {
        if let Some(last) = children.iter().last() {
            commands.entity(last).despawn();
        }
    }

    if keys.just_pressed(KeyCode::Space) {
        commands.entity(path_entity).despawn_related::<Children>();
        commands.entity(path_entity).despawn();
        spawn_road(
            &mut commands,
            false,
            &[
                (Vec3::new(-6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.5), Vec3::new(0.0, 0.0, 1.5)),
                (Vec3::new(-2.0, 0.0, 3.0), Vec3::new(-1.5, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0)),
                (Vec3::new(2.0, 0.0, -3.0), Vec3::new(-1.5, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0)),
                (Vec3::new(6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.5), Vec3::new(0.0, 0.0, 1.5)),
            ],
        );
    }
}

fn update_status_text(
    paths: Query<&BezierPath>,
    mut text_query: Query<&mut Text, With<StatusText>>,
) {
    let closed = paths.iter().next().map(|p| p.closed).unwrap_or(false);
    if let Ok(mut text) = text_query.single_mut() {
        **text = status_text_content(closed);
    }
}
