//! Bezier spline gizmo rendering systems.

use bevy::gizmos::prelude::*;
use bevy::prelude::*;

use crate::{
    components::{BezierPath, BezierPathNode, GizmoDrawMode},
    interaction::{collect_axis_handles, AxisHandleConfig},
    math::sample_cubic_bezier,
};

/// Draws gizmos for every [`BezierPath`] in the world.
///
/// For each path the system:
/// 1. Collects ordered child [`BezierPathNode`] entities.
/// 2. Draws the spline curve (bright teal) by sampling cubic Bezier segments.
/// 3. Draws waypoint markers (yellow circles) and control handles when enabled.
pub fn draw_bezier_spline_gizmos(
    mut gizmos: Gizmos,
    paths: Query<(&BezierPath, &Children)>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
) {
    for (path, children) in &paths {
        let node_data: Vec<(Vec3, Vec3, Vec3)> = children
            .iter()
            .filter_map(|child| {
                nodes
                    .get(child)
                    .ok()
                    .map(|(node, tf)| (tf.translation(), node.incoming, node.outgoing))
            })
            .collect();

        if node_data.len() < 2 {
            continue;
        }

        let draw_spline = matches!(
            path.gizmo_draw_mode,
            GizmoDrawMode::Complete | GizmoDrawMode::SplineOnly
        );
        let draw_supports = matches!(
            path.gizmo_draw_mode,
            GizmoDrawMode::Complete | GizmoDrawMode::WaypointOnly
        );

        let segment_count = if path.closed {
            node_data.len()
        } else {
            node_data.len() - 1
        };

        if draw_spline {
            let spline_color = Color::srgb(0.0, 0.8, 1.0);

            for i in 0..segment_count {
                let next = (i + 1) % node_data.len();
                let (p0, _in0, out0) = node_data[i];
                let (p3, in3, _out3) = node_data[next];
                let p1 = p0 + out0;
                let p2 = p3 + in3;

                let points: Vec<Vec3> =
                    sample_cubic_bezier(p0, p1, p2, p3, path.subdivisions).collect();
                gizmos.linestrip(points, spline_color);
            }
        }

        if draw_supports {
            let last = node_data.len() - 1;

            for (i, (center, incoming, outgoing)) in node_data.iter().enumerate() {
                let xz_isometry =
                    Isometry3d::new(*center, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
                gizmos
                    .circle(xz_isometry, 0.15, Color::srgb(1.0, 1.0, 0.0))
                    .resolution(24);

                let draw_incoming = i > 0 || path.closed;
                let draw_outgoing = i < last || path.closed;

                if draw_incoming && *incoming != Vec3::ZERO {
                    let ctrl = center + incoming;
                    gizmos.line(*center, ctrl, Color::srgb(0.2, 0.6, 1.0));
                    gizmos
                        .sphere(Isometry3d::from(ctrl), 0.08, Color::srgb(0.2, 0.6, 1.0))
                        .resolution(16);
                }

                if draw_outgoing && *outgoing != Vec3::ZERO {
                    let ctrl = center + outgoing;
                    gizmos.line(*center, ctrl, Color::srgb(1.0, 0.5, 0.2));
                    gizmos
                        .sphere(Isometry3d::from(ctrl), 0.08, Color::srgb(1.0, 0.5, 0.2))
                        .resolution(16);
                }
            }

            for i in 0..segment_count {
                let next = (i + 1) % node_data.len();
                gizmos.line(
                    node_data[i].0,
                    node_data[next].0,
                    Color::srgba(0.5, 0.5, 0.5, 0.5),
                );
            }
        }
    }
}

/// Draws axis-handle arrows (red=X, green=Y, blue=Z) at every node center and
/// control-handle tip. Runs in `PostUpdate` after [`draw_bezier_spline_gizmos`].
pub fn draw_axis_handle_gizmos(
    mut gizmos: Gizmos,
    config: Res<AxisHandleConfig>,
    paths: Query<(&BezierPath, &Children)>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
) {
    let handles = collect_axis_handles(&nodes, &paths, config.arrow_length);
    for pick in &handles {
        gizmos.arrow(pick.anchor, pick.tip, pick.axis.color());
    }
}
