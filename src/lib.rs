//! # bevy-bezier-splines
//!
//! A Bevy implementation of cubic Bezier spline gizmos, ported from the Unity
//! [`unity-bezier-splines`](https://github.com/sunsided/unity-bezier-splines) project.
//!
//! ## Usage
//!
//! Add [`BezierSplinesPlugin`] to your app, then spawn entities with
//! [`BezierPath`] and child entities carrying [`BezierPathNode`] components.

pub mod components;
pub mod interaction;
#[cfg(feature = "jackdaw")]
pub mod jackdaw;
pub mod math;
pub mod snapshot;
pub mod systems;

pub use components::{BezierPath, BezierPathNode, GizmoDrawMode, NodeType};
pub use interaction::{
    collect_axis_handles, ray_axis_closest_s, AxisDrag, AxisHandleConfig, AxisHandleDragState,
    AxisHandleGrabbed, AxisHandleInteractionPlugin, AxisHandlePick, AxisHandleReleased, DragAxis,
    HandleKind,
};
#[cfg(feature = "jackdaw")]
pub use jackdaw::{
    JackdawSplineChanged, JackdawSplineIntegrationPlugin, JackdawSplineRegistry, JackdawSplineSync,
};
pub use snapshot::{
    build_snapshot_from_node_data, collect_node_data, collect_snapshot, BezierSplineSnapshot,
    CubicBezierSegment,
};

use bevy::prelude::*;

/// Plugin that registers all systems required to draw Bezier spline gizmos.
pub struct BezierSplinesPlugin;

impl Plugin for BezierSplinesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BezierPath>()
            .register_type::<BezierPathNode>()
            .register_type::<NodeType>()
            .register_type::<GizmoDrawMode>()
            .register_type::<BezierSplineSnapshot>()
            .register_type::<CubicBezierSegment>()
            .add_systems(PostUpdate, systems::draw_bezier_spline_gizmos)
            .add_systems(
                PostUpdate,
                systems::draw_axis_handle_gizmos.after(systems::draw_bezier_spline_gizmos),
            );
    }
}
