use bevy::{prelude::*, render::{settings::{Backends, RenderCreation, WgpuSettings}, RenderPlugin}};
use bevy_ecs_ldtk::prelude::*;

use bevy_rapier2d::prelude::*;

mod components;
mod systems;

fn main() {
    App::new()
        .add_plugins((
            BasePlugin,
            LdtkPlugin,
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
        ))
        .insert_resource(LevelSelection::index(0))
        .register_ldtk_entity::<components::PlayerBundle>("Player")
        .register_ldtk_entity::<components::GoalBundle>("Goal")
        .add_systems(Startup, systems::setup)
        .run();
}

pub struct BasePlugin;

impl Plugin for BasePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(DefaultPlugins.set(
        RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::DX12),
                            ..default()
                    }),
                }
            ).set(
                ImagePlugin::default_nearest()
            )
        );
	}
}