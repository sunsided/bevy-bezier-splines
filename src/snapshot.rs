//! Stable conversion helpers for extracting Bezier spline data from ECS.

use bevy::prelude::*;

use crate::{BezierPath, BezierPathNode};

/// A cubic segment represented by four absolute control points.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct CubicBezierSegment {
    /// Start waypoint.
    pub p0: Vec3,
    /// Outgoing control point from [`Self::p0`].
    pub p1: Vec3,
    /// Incoming control point into [`Self::p3`].
    pub p2: Vec3,
    /// End waypoint.
    pub p3: Vec3,
}

/// Immutable snapshot of a spline suitable for external consumers.
#[derive(Debug, Clone, Default, PartialEq, Reflect)]
pub struct BezierSplineSnapshot {
    /// Whether the last node connects back to the first node.
    pub closed: bool,
    /// Sampling density to use for downstream consumers.
    pub subdivisions: u32,
    /// Ordered cubic segments composing the spline.
    pub segments: Vec<CubicBezierSegment>,
}

impl BezierSplineSnapshot {
    /// Returns `true` when there are no segments to consume.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Collects ordered node tuples from `children` for a [`BezierPath`].
///
/// Each tuple is `(center, incoming, outgoing)` where `center` is in world-space
/// and handle offsets are local to that center.
pub fn collect_node_data(
    children: &Children,
    nodes: &Query<(&BezierPathNode, &GlobalTransform)>,
) -> Vec<(Vec3, Vec3, Vec3)> {
    children
        .iter()
        .filter_map(|child| {
            nodes
                .get(child)
                .ok()
                .map(|(node, tf)| (tf.translation(), node.incoming, node.outgoing))
        })
        .collect()
}

/// Builds a spline snapshot from already-collected node data.
///
/// Returns `None` when there are fewer than two valid nodes.
pub fn build_snapshot_from_node_data(
    path: &BezierPath,
    node_data: &[(Vec3, Vec3, Vec3)],
) -> Option<BezierSplineSnapshot> {
    if node_data.len() < 2 {
        return None;
    }

    let segment_count = if path.closed {
        node_data.len()
    } else {
        node_data.len() - 1
    };

    let mut segments = Vec::with_capacity(segment_count);
    for i in 0..segment_count {
        let next = (i + 1) % node_data.len();
        let (p0, _in0, out0) = node_data[i];
        let (p3, in3, _out3) = node_data[next];
        segments.push(CubicBezierSegment {
            p0,
            p1: p0 + out0,
            p2: p3 + in3,
            p3,
        });
    }

    Some(BezierSplineSnapshot {
        closed: path.closed,
        subdivisions: path.subdivisions.max(1),
        segments,
    })
}

/// Collects node data and converts it into a snapshot in one call.
pub fn collect_snapshot(
    path: &BezierPath,
    children: &Children,
    nodes: &Query<(&BezierPathNode, &GlobalTransform)>,
) -> Option<BezierSplineSnapshot> {
    let node_data = collect_node_data(children, nodes);
    build_snapshot_from_node_data(path, &node_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeType;

    #[test]
    fn builds_open_snapshot_segments() {
        let path = BezierPath {
            closed: false,
            subdivisions: 0,
            ..default()
        };
        let nodes = vec![
            (
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ),
            (
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(-0.5, 0.0, 0.0),
                Vec3::new(0.5, 0.0, 0.0),
            ),
            (
                Vec3::new(4.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ),
        ];

        let snapshot = build_snapshot_from_node_data(&path, &nodes).unwrap();
        assert!(!snapshot.closed);
        assert_eq!(snapshot.subdivisions, 1);
        assert_eq!(snapshot.segments.len(), 2);
        assert_eq!(snapshot.segments[0].p0, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(snapshot.segments[0].p1, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(snapshot.segments[0].p2, Vec3::new(1.5, 0.0, 0.0));
        assert_eq!(snapshot.segments[0].p3, Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn builds_closed_snapshot_segments() {
        let path = BezierPath {
            closed: true,
            ..default()
        };
        let nodes = vec![
            (
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
            ),
            (
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::ZERO,
            ),
        ];

        let snapshot = build_snapshot_from_node_data(&path, &nodes).unwrap();
        assert!(snapshot.closed);
        assert_eq!(snapshot.segments.len(), 2);
        assert_eq!(snapshot.segments[1].p0, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(snapshot.segments[1].p3, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn none_for_too_few_nodes() {
        let path = BezierPath::default();
        let nodes = vec![(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)];
        assert!(build_snapshot_from_node_data(&path, &nodes).is_none());
    }

    #[test]
    fn node_type_does_not_affect_snapshot_shape() {
        let mut node = BezierPathNode {
            incoming: Vec3::new(-1.0, 0.0, 0.0),
            outgoing: Vec3::new(1.0, 0.0, 0.0),
            node_type: NodeType::Connected,
        };
        node.update_from_incoming();
        let path = BezierPath::default();
        let nodes = vec![
            (Vec3::ZERO, node.incoming, node.outgoing),
            (
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::ZERO,
            ),
        ];
        assert!(build_snapshot_from_node_data(&path, &nodes).is_some());
    }
}
