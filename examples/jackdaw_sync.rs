//! Minimal Jackdaw adapter demonstration.
//!
//! Run with:
//! `cargo run --example jackdaw_sync --features jackdaw`

use bevy::prelude::*;
use bevy_bezier_splines::{
    BezierPath, BezierPathNode, BezierSplinesPlugin, JackdawSplineChanged,
    JackdawSplineIntegrationPlugin, JackdawSplineSync,
};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(BezierSplinesPlugin)
        .add_plugins(JackdawSplineIntegrationPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (nudge_first_node, print_sync_messages))
        .run();
}

fn setup(mut commands: Commands) {
    let path = commands
        .spawn((
            BezierPath::default(),
            Transform::default(),
            JackdawSplineSync,
        ))
        .id();

    let a = commands
        .spawn((
            BezierPathNode::default(),
            Transform::from_xyz(-1.0, 0.0, 0.0),
        ))
        .id();
    let b = commands
        .spawn((
            BezierPathNode::default(),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();
    commands.entity(path).add_child(a);
    commands.entity(path).add_child(b);
}

fn nudge_first_node(time: Res<Time>, mut nodes: Query<&mut Transform, With<BezierPathNode>>) {
    if let Some(mut first) = nodes.iter_mut().next() {
        first.translation.x = -1.0 + time.elapsed_secs().sin() * 0.25;
    }
}

fn print_sync_messages(mut messages: MessageReader<JackdawSplineChanged>) {
    for msg in messages.read() {
        let state = if msg.snapshot.is_some() {
            "updated"
        } else {
            "removed"
        };
        println!("jackdaw snapshot {state} for entity {:?}", msg.entity);
    }
}
