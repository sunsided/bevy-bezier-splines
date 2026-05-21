//! Feature-gated Jackdaw adapter surface.
//!
//! This adapter is intentionally one-way: edits to [`crate::BezierPath`] and
//! [`crate::BezierPathNode`] are converted into immutable snapshots that Jackdaw
//! tools can consume. It does not write back into spline components.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{collect_snapshot, BezierPath, BezierPathNode, BezierSplineSnapshot};

/// Marks spline entities that should publish Jackdaw-facing snapshots.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct JackdawSplineSync;

/// One-way cache of latest snapshots keyed by spline entity.
#[derive(Debug, Default, Resource)]
pub struct JackdawSplineRegistry {
    /// Current snapshots for synced spline entities.
    pub snapshots: HashMap<Entity, BezierSplineSnapshot>,
}

/// Emitted whenever a synced spline snapshot changed or was removed.
#[derive(Debug, Clone, Message)]
pub struct JackdawSplineChanged {
    /// The spline entity whose cached snapshot changed.
    pub entity: Entity,
    /// New snapshot value; `None` indicates removal or invalid data (<2 nodes).
    pub snapshot: Option<BezierSplineSnapshot>,
}

/// Plugin that keeps [`JackdawSplineRegistry`] up-to-date via change detection.
pub struct JackdawSplineIntegrationPlugin;

impl Plugin for JackdawSplineIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<JackdawSplineSync>()
            .init_resource::<JackdawSplineRegistry>()
            .add_message::<JackdawSplineChanged>()
            .add_systems(
                Update,
                (sync_jackdaw_spline_snapshots, cleanup_removed_sync_markers),
            );
    }
}

fn sync_jackdaw_spline_snapshots(
    paths: Query<(Entity, Ref<BezierPath>, Ref<Children>), With<JackdawSplineSync>>,
    nodes: Query<(&BezierPathNode, &GlobalTransform)>,
    changed_nodes: Query<Ref<BezierPathNode>>,
    mut registry: ResMut<JackdawSplineRegistry>,
    mut changed_writer: MessageWriter<JackdawSplineChanged>,
) {
    for (entity, path, children) in &paths {
        let has_changed_node = children.iter().any(|child| {
            changed_nodes
                .get(child)
                .map(|node_ref| node_ref.is_changed())
                .unwrap_or(false)
        });

        let needs_sync = path.is_changed()
            || children.is_changed()
            || has_changed_node
            || !registry.snapshots.contains_key(&entity);
        if !needs_sync {
            continue;
        }

        let snapshot = collect_snapshot(&path, &children, &nodes);
        let changed = registry.snapshots.get(&entity) != snapshot.as_ref();
        if !changed {
            continue;
        }

        if let Some(snapshot) = snapshot.clone() {
            registry.snapshots.insert(entity, snapshot);
        } else {
            registry.snapshots.remove(&entity);
        }

        changed_writer.write(JackdawSplineChanged { entity, snapshot });
    }
}

fn cleanup_removed_sync_markers(
    mut removed_markers: RemovedComponents<JackdawSplineSync>,
    mut registry: ResMut<JackdawSplineRegistry>,
    mut changed_writer: MessageWriter<JackdawSplineChanged>,
) {
    for entity in removed_markers.read() {
        if registry.snapshots.remove(&entity).is_some() {
            changed_writer.write(JackdawSplineChanged {
                entity,
                snapshot: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BezierPathNode;

    fn setup_path(app: &mut App) -> Entity {
        let path = app
            .world_mut()
            .spawn((
                BezierPath::default(),
                Transform::default(),
                JackdawSplineSync,
            ))
            .id();

        let n0 = app
            .world_mut()
            .spawn((
                BezierPathNode::default(),
                Transform::from_xyz(-1.0, 0.0, 0.0),
            ))
            .id();
        let n1 = app
            .world_mut()
            .spawn((
                BezierPathNode::default(),
                Transform::from_xyz(1.0, 0.0, 0.0),
            ))
            .id();

        app.world_mut().entity_mut(path).add_child(n0);
        app.world_mut().entity_mut(path).add_child(n1);
        path
    }

    #[test]
    fn sync_populates_registry_and_emits_message() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(JackdawSplineIntegrationPlugin);

        let path = setup_path(&mut app);
        app.update();

        let registry = app.world().resource::<JackdawSplineRegistry>();
        assert!(registry.snapshots.contains_key(&path));

        let messages = app
            .world_mut()
            .resource_mut::<Messages<JackdawSplineChanged>>();
        assert!(!messages.is_empty());
    }

    #[test]
    fn removing_marker_cleans_registry() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(JackdawSplineIntegrationPlugin);

        let path = setup_path(&mut app);
        app.update();

        app.world_mut()
            .entity_mut(path)
            .remove::<JackdawSplineSync>();
        app.update();

        let registry = app.world().resource::<JackdawSplineRegistry>();
        assert!(!registry.snapshots.contains_key(&path));
    }
}
