//! Axis-aligned handle interaction types, helpers, systems, and plugin.
//!
//! Provides the building blocks for 3D axis-constrained dragging of Bezier
//! spline control points: types (`HandleKind`, `DragAxis`, `AxisDrag`), a
//! collection helper (`collect_axis_handles`), the closest-point math
//! (`ray_axis_closest_s`), and an opt-in `AxisHandleInteractionPlugin` that
//! wires up a turnkey pick/drag/release flow.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{
    components::{BezierPath, BezierPathNode},
    systems::draw_bezier_spline_gizmos,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Which draggable point on a Bezier node is targeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    /// The node center (waypoint).
    Center,
    /// The incoming control handle tip.
    Incoming,
    /// The outgoing control handle tip.
    Outgoing,
}

/// World-axis constraint used while dragging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAxis {
    X,
    Y,
    Z,
}

impl DragAxis {
    /// Unit vector along this axis.
    pub fn unit(self) -> Vec3 {
        match self {
            DragAxis::X => Vec3::X,
            DragAxis::Y => Vec3::Y,
            DragAxis::Z => Vec3::Z,
        }
    }

    /// Canonical colour for gizmo rendering.
    pub fn color(self) -> Color {
        match self {
            DragAxis::X => Color::srgb(1.0, 0.0, 0.0),
            DragAxis::Y => Color::srgb(0.0, 1.0, 0.0),
            DragAxis::Z => Color::srgb(0.0, 0.0, 1.0),
        }
    }
}

/// Per-point axis handles for one node (3 pick targets per point x 3 points = 9 per node).
pub struct AxisHandlePick {
    pub entity: Entity,
    pub kind: HandleKind,
    pub axis: DragAxis,
    /// The point the arrow originates from (world space).
    pub anchor: Vec3,
    /// `anchor + axis.unit() * arrow_length` (world space).
    pub tip: Vec3,
}

/// Collect all axis-aligned pick targets for every node in every path.
pub fn collect_axis_handles(
    nodes: &Query<(&BezierPathNode, &GlobalTransform)>,
    paths: &Query<(&BezierPath, &Children)>,
    arrow_length: f32,
) -> Vec<AxisHandlePick> {
    let mut picks = Vec::new();
    for (_path, children) in paths.iter() {
        for child in children.iter() {
            let Ok((node, tf)) = nodes.get(child) else {
                continue;
            };
            let center = tf.translation();
            let points = [
                (HandleKind::Center, center),
                (HandleKind::Incoming, center + node.incoming),
                (HandleKind::Outgoing, center + node.outgoing),
            ];
            for (kind, point) in points {
                for axis in [DragAxis::X, DragAxis::Y, DragAxis::Z] {
                    let dir = axis.unit();
                    picks.push(AxisHandlePick {
                        entity: child,
                        kind,
                        axis,
                        anchor: point,
                        tip: point + dir * arrow_length,
                    });
                }
            }
        }
    }
    picks
}

/// Signed distance along `axis_dir` (through `axis_origin`) to the closest
/// point on the given ray.
///
/// Returns `None` when the ray is nearly parallel to the axis line (denom
/// close to zero).
pub fn ray_axis_closest_s(ray: &Ray3d, axis_origin: Vec3, axis_dir: Vec3) -> Option<f32> {
    let w = ray.origin - axis_origin;
    let b = ray.direction.dot(axis_dir);
    let denom = 1.0 - b * b;
    if denom.abs() < 1e-6 {
        return None;
    }
    let d_oa = ray.direction.dot(w);
    let a_w = axis_dir.dot(w);
    Some((a_w - b * d_oa) / denom)
}

/// Active drag session state.
#[derive(Debug, Clone, Copy)]
pub struct AxisDrag {
    pub entity: Entity,
    pub kind: HandleKind,
    pub axis: DragAxis,
    /// World-space position of the handle at mouse-down.
    pub anchor: Vec3,
    /// Signed distance along the axis at mouse-down (used to avoid jump).
    pub grab_offset: f32,
}

// ---------------------------------------------------------------------------
// Plugin resources
// ---------------------------------------------------------------------------

/// Configuration for axis-handle interaction.
#[derive(Resource)]
pub struct AxisHandleConfig {
    pub arrow_length: f32,
    pub pick_radius_px: f32,
    pub active: bool,
}

impl Default for AxisHandleConfig {
    fn default() -> Self {
        Self {
            arrow_length: 0.25,
            pick_radius_px: 12.0,
            active: true,
        }
    }
}

/// Drag session state resource.
#[derive(Resource, Default)]
pub struct AxisHandleDragState {
    pub dragging: Option<AxisDrag>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Emitted when an axis handle is grabbed.
#[derive(Message)]
pub struct AxisHandleGrabbed {
    pub entity: Entity,
    pub kind: HandleKind,
    pub axis: DragAxis,
}

/// Emitted when an axis handle is released.
#[derive(Message)]
pub struct AxisHandleReleased {
    pub entity: Entity,
}

// ---------------------------------------------------------------------------
// Systems (turnkey plugin)
// ---------------------------------------------------------------------------

/// Helper: build a cursor ray from the primary window + first 3D camera.
fn cursor_ray(window: &Window, camera_tf: &GlobalTransform, camera: &Camera) -> Option<Ray3d> {
    let cursor = window.cursor_position()?;
    camera.viewport_to_world(camera_tf, cursor).ok()
}

/// Helper: project world point to screen (pixel) coordinates.
fn world_to_screen(
    world: Vec3,
    camera: &Camera,
    camera_tf: &GlobalTransform,
    viewport: Vec2,
) -> Option<Vec2> {
    let ndc = camera.world_to_ndc(camera_tf, world)?;
    if ndc.z < 0.0 || ndc.z > 1.0 {
        return None;
    }
    Some(Vec2::new(
        (ndc.x * 0.5 + 0.5) * viewport.x,
        (1.0 - (ndc.y * 0.5 + 0.5)) * viewport.y,
    ))
}

/// Helper: distance from a point to a 2D line segment.
fn dist_point_to_segment_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq < 1e-8 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// Pick system: on left-click, find the closest arrow segment to the cursor.
pub fn axis_handle_pick(
    config: Res<AxisHandleConfig>,
    mut drag_state: ResMut<AxisHandleDragState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
    paths: Query<(&BezierPath, &Children)>,
    mut grabbed_writer: MessageWriter<AxisHandleGrabbed>,
) {
    if !config.active
        || !mouse_button.just_pressed(MouseButton::Left)
        || drag_state.dragging.is_some()
    {
        return;
    }

    let window = windows.single().unwrap();
    let (camera, cam_tf) = camera_query.single().unwrap();
    let viewport = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::new(window.width(), window.height()));
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let handles = collect_axis_handles(&nodes, &paths, config.arrow_length);

    let mut best_dist = f32::MAX;
    let mut best_pick: Option<usize> = None;

    for (i, pick) in handles.iter().enumerate() {
        let Some(anchor_screen) = world_to_screen(pick.anchor, camera, cam_tf, viewport) else {
            continue;
        };
        let Some(tip_screen) = world_to_screen(pick.tip, camera, cam_tf, viewport) else {
            continue;
        };
        let d = dist_point_to_segment_2d(cursor, anchor_screen, tip_screen);
        if d < config.pick_radius_px && d < best_dist {
            best_dist = d;
            best_pick = Some(i);
        }
    }

    let Some(idx) = best_pick else {
        return;
    };
    let pick = &handles[idx];

    let Some(ray) = cursor_ray(window, cam_tf, camera) else {
        return;
    };

    let Some(s) = ray_axis_closest_s(&ray, pick.anchor, pick.axis.unit()) else {
        return;
    };
    drag_state.dragging = Some(AxisDrag {
        entity: pick.entity,
        kind: pick.kind,
        axis: pick.axis,
        anchor: pick.anchor,
        grab_offset: s,
    });
    grabbed_writer.write(AxisHandleGrabbed {
        entity: pick.entity,
        kind: pick.kind,
        axis: pick.axis,
    });
}

/// Drag system: constrain the dragged handle to its axis.
pub fn axis_handle_drag(
    _config: Res<AxisHandleConfig>,
    drag_state: Res<AxisHandleDragState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut nodes: Query<(&mut BezierPathNode, &mut Transform)>,
) {
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }
    let Some(drag) = drag_state.dragging else {
        return;
    };

    let window = windows.single().unwrap();
    let (camera, cam_tf) = camera_query.single().unwrap();
    let Some(ray) = cursor_ray(window, cam_tf, camera) else {
        return;
    };

    let Some(s) = ray_axis_closest_s(&ray, drag.anchor, drag.axis.unit()) else {
        return;
    };
    let displacement = drag.axis.unit() * (s - drag.grab_offset);
    let new_point = drag.anchor + displacement;

    let Ok((mut node, mut tf)) = nodes.get_mut(drag.entity) else {
        return;
    };

    match drag.kind {
        HandleKind::Center => {
            tf.translation = new_point;
        }
        HandleKind::Incoming => {
            node.incoming = new_point - tf.translation;
            node.update_from_incoming();
        }
        HandleKind::Outgoing => {
            node.outgoing = new_point - tf.translation;
            node.update_from_outgoing();
        }
    }
}

/// Release system: clear drag state on left-button release.
pub fn axis_handle_release(
    mut drag_state: ResMut<AxisHandleDragState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut released_writer: MessageWriter<AxisHandleReleased>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        if let Some(drag) = drag_state.dragging {
            released_writer.write(AxisHandleReleased {
                entity: drag.entity,
            });
            drag_state.dragging = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Turnkey plugin that wires up axis-handle picking, dragging, and release.
///
/// Add this plugin alongside [`crate::BezierSplinesPlugin`]:
/// ```ignore
/// app.add_plugins(BezierSplinesPlugin)
///    .add_plugins(AxisHandleInteractionPlugin);
/// ```
///
/// Systems run in `Update`, chained in order: pick -> drag -> release.
/// They are ordered **before** [`draw_bezier_spline_gizmos`] so that drag
/// state is available to downstream systems (e.g. camera orbit guards).
pub struct AxisHandleInteractionPlugin;

impl Plugin for AxisHandleInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AxisHandleConfig>()
            .init_resource::<AxisHandleDragState>()
            .add_message::<AxisHandleGrabbed>()
            .add_message::<AxisHandleReleased>()
            .add_systems(
                Update,
                (
                    axis_handle_pick,
                    axis_handle_drag.after(axis_handle_pick),
                    axis_handle_release
                        .after(axis_handle_drag)
                        .before(draw_bezier_spline_gizmos),
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ray(origin: Vec3, dir: Vec3) -> Ray3d {
        Ray3d::new(origin, Dir3::new_unchecked(dir.normalize()))
    }

    #[test]
    fn exact_hit_along_x_axis() {
        // Ray aimed directly at X axis through origin.
        let ray = make_ray(Vec3::new(0.0, 5.0, 3.0), Vec3::new(1.0, -1.0, -0.6));
        // The ray crosses the X axis; s should give the x where it crosses.
        // Closest point between ray and X-axis line through origin:
        // w = (0,5,3) - (0,0,0) = (0,5,3)
        // b = d . a = normalize(1,-1,-0.6) . (1,0,0)
        let d = Vec3::new(1.0, -1.0, -0.6).normalize();
        let b = d.dot(Vec3::X);
        let w = Vec3::new(0.0, 5.0, 3.0);
        let denom = 1.0 - b * b;
        let expected_s = (Vec3::X.dot(w) - b * d.dot(w)) / denom;
        let s = ray_axis_closest_s(&ray, Vec3::ZERO, Vec3::X).unwrap();
        assert!((s - expected_s).abs() < 1e-5);
    }

    #[test]
    fn ray_parallel_to_axis_returns_none() {
        let ray = make_ray(Vec3::new(0.0, 1.0, 0.0), Vec3::X);
        // Ray is parallel to X axis and offset by Y=1 => denom ~ 0.
        let result = ray_axis_closest_s(&ray, Vec3::ZERO, Vec3::X);
        assert!(result.is_none());
    }

    #[test]
    fn ray_crosses_y_axis_at_origin() {
        // Ray from (3,0,0) aimed towards (-3,0,0) crosses Y axis at y=0.
        let ray = make_ray(Vec3::new(3.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
        let s = ray_axis_closest_s(&ray, Vec3::ZERO, Vec3::Y).unwrap();
        assert!(s.abs() < 1e-5);
    }

    #[test]
    fn ray_hits_z_axis_at_known_point() {
        // Ray from (3, 0, 0) going straight left crosses Z axis at z=0.
        let ray = make_ray(Vec3::new(3.0, 0.0, 0.0), -Vec3::X);
        let s = ray_axis_closest_s(&ray, Vec3::ZERO, Vec3::Z).unwrap();
        assert!(s.abs() < 1e-5);
    }
}
