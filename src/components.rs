//! ECS components for Bezier splines.

use bevy::prelude::*;

/// Controls how the two control handles of a node relate to each other.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum NodeType {
    /// Handles are kept mirror-symmetric (same direction **and** magnitude).
    Symmetric,
    /// Handles point in opposite directions but may have different magnitudes.
    #[default]
    Connected,
    /// Handles are fully independent.
    Broken,
}

/// Controls which parts of the spline gizmo are rendered.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum GizmoDrawMode {
    /// Draw the spline curve and the control handles.
    #[default]
    Complete,
    /// Draw only the spline curve.
    SplineOnly,
    /// Draw only the waypoint markers and dashed connections.
    WaypointOnly,
    /// Draw nothing.
    None,
}

/// A cubic Bezier spline path.
///
/// Place this component on an entity. Each child entity carrying a
/// [`BezierPathNode`] component becomes a waypoint of the spline, visited
/// in [`Children`] order.
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct BezierPath {
    /// Number of line segments used to approximate each cubic segment.
    pub subdivisions: u32,
    /// Whether the last node connects back to the first node.
    pub closed: bool,
    /// Controls which visual elements are rendered.
    pub gizmo_draw_mode: GizmoDrawMode,
}

impl Default for BezierPath {
    fn default() -> Self {
        Self {
            subdivisions: 30,
            closed: false,
            gizmo_draw_mode: GizmoDrawMode::Complete,
        }
    }
}

/// A waypoint (control node) on a [`BezierPath`].
///
/// The node's world position is taken from the entity's [`Transform`].
/// `incoming` and `outgoing` are **local** offsets from the node center.
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct BezierPathNode {
    /// Local offset for the incoming control handle.
    pub incoming: Vec3,
    /// Local offset for the outgoing control handle.
    pub outgoing: Vec3,
    /// How the two handles constrain each other.
    pub node_type: NodeType,
}

impl Default for BezierPathNode {
    fn default() -> Self {
        Self {
            incoming: Vec3::new(0.0, 0.0, -0.5),
            outgoing: Vec3::new(0.0, 0.0, 0.5),
            node_type: NodeType::Connected,
        }
    }
}

impl BezierPathNode {
    /// Update the outgoing handle after the incoming handle has changed.
    pub fn update_from_incoming(&mut self) {
        match self.node_type {
            NodeType::Connected => {
                let mag = self.outgoing.length();
                self.outgoing = -self.incoming.normalize_or_zero() * mag;
            }
            NodeType::Symmetric => {
                self.outgoing = -self.incoming;
            }
            NodeType::Broken => {}
        }
    }

    /// Update the incoming handle after the outgoing handle has changed.
    pub fn update_from_outgoing(&mut self) {
        match self.node_type {
            NodeType::Connected => {
                let mag = self.incoming.length();
                self.incoming = -self.outgoing.normalize_or_zero() * mag;
            }
            NodeType::Symmetric => {
                self.incoming = -self.outgoing;
            }
            NodeType::Broken => {}
        }
    }
}
