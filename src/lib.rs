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
pub mod math;
pub mod systems;

pub use components::{BezierPath, BezierPathNode, GizmoDrawMode, NodeType};

use bevy::prelude::*;

/// Plugin that registers all systems required to draw Bezier spline gizmos.
pub struct BezierSplinesPlugin;

impl Plugin for BezierSplinesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BezierPath>()
            .register_type::<BezierPathNode>()
            .register_type::<NodeType>()
            .register_type::<GizmoDrawMode>()
            .add_systems(Update, systems::draw_bezier_spline_gizmos);
    }
}
