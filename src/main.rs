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
            //RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(LevelSelection::index(0))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::new(0.0, -2000.0),
            ..default()
        })
        .insert_resource(LdtkSettings {
            level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation { 
                load_level_neighbors: true,
            },
            set_clear_color: SetClearColor::FromLevelBackground,
            ..default()
        })
        .register_ldtk_int_cell::<components::WallBundle>(1)
        .register_ldtk_int_cell::<components::LadderBundle>(2)
        .register_ldtk_int_cell::<components::WallBundle>(3)
        .register_ldtk_entity::<components::PlayerBundle>("Player")
        .register_ldtk_entity::<components::GoalBundle>("Goal")
        .register_ldtk_entity::<components::MobBundle>("Mob")
        .register_ldtk_entity::<components::ChestBundle>("Chest")
        .register_ldtk_entity::<components::PumpkinsBundle>("Pumpkins")
        .add_systems(Startup, systems::setup)
        .add_systems(Update, (
            systems::dbg_player_items,
            systems::movement,
            systems::spawn_wall_collision,
            systems::detect_climb_range,
            systems::ignore_gravity_if_climbing,
            systems::spawn_ground_sensor,
            systems::ground_detection,
            systems::update_on_ground,
            systems::camera_fit_inside_current_level,
            systems::respawn_world,
            systems::patrol,
            systems::flip_sprite,
        ))
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