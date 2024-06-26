use crate::components::*;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use bevy_rapier2d::prelude::*;

use std::collections::{HashMap, HashSet};

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let camera = Camera2dBundle::default();
    //camera.projection.scale = 1.5;
    commands.spawn(camera);

    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("example_platformer.ldtk"),
        ..default()
    });
}

pub fn dbg_player_items(
    input: Res<Input<KeyCode>>,
    mut query: Query<(&Items, &EntityInstance), With<Player>>,
) {
    for (items, entity_instance) in &mut query {
        if input.just_pressed(KeyCode::P) {
            dbg!(&items);
            dbg!(&entity_instance);
        }
    }
}

pub fn movement(
    input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Velocity, &mut Climber, &GroundDetection), With<Player>>,
) {
    for (mut velocity, mut climber, ground_detection) in &mut query {
        let right = if input.pressed(KeyCode::D) { 1.0 } else { 0.0 };
        let left = if input.pressed(KeyCode::A) { 1.0 } else { 0.0 };

        velocity.linvel.x = (right - left) * 200.0;

        if climber.intersecting_climbables.is_empty() {
            climber.climbing = false;
        } else if input.just_pressed(KeyCode::W) || input.just_pressed(KeyCode::S) {
            climber.climbing = true;
        }

        if climber.climbing {
            let up = if input.pressed(KeyCode::W) { 1.0 } else { 0.0 };
            let down = if input.pressed(KeyCode::S) { 1.0 } else { 0.0 };

            velocity.linvel.y = (up - down) * 200.0;
        }

        if input.just_pressed(KeyCode::Space) && (ground_detection.on_ground || climber.climbing) {
            velocity.linvel.y = 500.0;
            climber.climbing = false;
        }
    }
}

pub fn direction_machine(
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut PlayerDirection, With<Player>>,
) {
    for mut direction in &mut query {
        // moving left
        if input.just_pressed(KeyCode::A) {
            *direction = PlayerDirection::Left;
        }

        // moving right
        if input.just_pressed(KeyCode::D) {
            *direction = PlayerDirection::Right;
        } 
    }
}

pub fn flip_sprite(
    mut query: Query<(&PlayerDirection, &mut TextureAtlasSprite), (With<Player>, Changed<PlayerDirection>)>,
) {
    for (direction, mut sprite) in &mut query {
        match direction {
            PlayerDirection::Left => {
                sprite.flip_x = true;
            }
            PlayerDirection::Right => {
                sprite.flip_x = false;
            }
        }
    }
}

pub fn state_machine(
    input: Res<Input<KeyCode>>,
    mut query: Query<(&mut PlayerState, &Velocity, &GroundDetection, &Climber), With<Player>>,
) {
    for (mut state, velocity, ground, climber) in &mut query {
        if input.just_pressed(KeyCode::Space) && (ground.on_ground || climber.climbing) {
            *state = PlayerState::Jumping;
        } else if !ground.on_ground && velocity.linvel.y < 0.0 && !state.eq(&PlayerState::Falling) && !state.eq(&PlayerState::Climbing) {
            *state = PlayerState::Falling;
        } else if ground.on_ground && velocity.linvel.x != 0.0 && !state.eq(&PlayerState::Moving) {
            *state = PlayerState::Moving;
        } else if ground.on_ground && velocity.linvel.x == 0.0 && velocity.linvel.y >= -0.1 && velocity.linvel.y <= 0.01 && !state.eq(&PlayerState::Idle) {
            *state = PlayerState::Idle;
        } else if climber.climbing && !state.eq(&PlayerState::Climbing) {
            *state = PlayerState::Climbing;
        }
    }
}

pub fn animation_machine(
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<(&PlayerState, &mut Handle<TextureAtlas>, &mut AnimationIndices, &mut AnimationTimer, &mut TextureAtlasSprite), (With<Player>, Changed<PlayerState>)>,
    asset_server: Res<AssetServer>,
) {
    for (state, mut atlas, mut indices, mut timer, mut sprite) in &mut query {        
        timer.timer = Timer::from_seconds(0.1, TimerMode::Repeating);

        let rows = 1;
        let cols: usize;
        let offset: Option<Vec2>;

        match state {
            PlayerState::Jumping => {
                cols = 2;
                indices.first = 0;
                indices.last = 0;
                offset = Some(Vec2::from_array([0.0, 160.0]));
                println!("jumping");
            }
            PlayerState::Falling => {
                cols = 2;
                indices.first = 1;
                indices.last = 1;
                offset = Some(Vec2::from_array([0.0, 160.0]));
                println!("falling");
            }
            PlayerState::Idle => {
                cols = 4;
                indices.first = 0;
                indices.last = 3;
                offset = None;
                println!("idling");
            }
            PlayerState::Moving => {
                cols = 6;
                indices.first = 0;
                indices.last = 5;
                offset = Some(Vec2::from_array([0.0, 32.0]));
                println!("moving");
            }
            PlayerState::Climbing => {
                cols = 4;
                indices.first = 0;
                indices.last = 3;
                offset = Some(Vec2::from_array([0.0, 64.0]));
                println!("climbing");
            }
        }

        sprite.index = indices.first;

        let image_handle = asset_server.load("Packs/Sunnyland/spritesheets/player.png");
        let layout = TextureAtlas::from_grid(image_handle, Vec2::from_array([32.0, 32.0]), cols, rows, None, offset);

        let atlas_handle = atlases.add(layout);
        *atlas = atlas_handle;
    }
}

pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlasSprite), With<Player>>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            sprite.index = if sprite.index == indices.last {
                indices.first
            } else {
                sprite.index + 1
            };
        }
    }
}

/// Spawns heron collisions for the walls of a level
///
/// You could just insert a ColliderBundle in to the WallBundle,
/// but this spawns a different collider for EVERY wall tile.
/// This approach leads to bad performance.
///
/// Instead, by flagging the wall tiles and spawning the collisions later,
/// we can minimize the amount of colliding entities.
///
/// The algorithm used here is a nice compromise between simplicity, speed,
/// and a small number of rectangle colliders.
/// In basic terms, it will:
/// 1. consider where the walls are
/// 2. combine wall tiles into flat "plates" in each individual row
/// 3. combine the plates into rectangles across multiple rows wherever possible
/// 4. spawn colliders for each rectangle
pub fn spawn_wall_collision(
    mut commands: Commands,
    wall_query: Query<(&GridCoords, &Parent), Added<Wall>>,
    parent_query: Query<&Parent, Without<Wall>>,
    level_query: Query<(Entity, &LevelIid)>,
    ldtk_projects: Query<&Handle<LdtkProject>>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
) {
    /// Represents a wide wall that is 1 tile tall
    /// Used to spawn wall collisions
    #[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
    struct Plate {
        left: i32,
        right: i32,
    }

    /// A simple rectangle type representing a wall of any size
    struct Rect {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    // Consider where the walls are
    // storing them as GridCoords in a HashSet for quick, easy lookup
    //
    // The key of this map will be the entity of the level the wall belongs to.
    // This has two consequences in the resulting collision entities:
    // 1. it forces the walls to be split along level boundaries
    // 2. it lets us easily add the collision entities as children of the appropriate level entity
    let mut level_to_wall_locations: HashMap<Entity, HashSet<GridCoords>> = HashMap::new();

    wall_query.iter().for_each(|(&grid_coords, parent)| {
        // An intgrid tile's direct parent will be a layer entity, not the level entity
        // To get the level entity, you need the tile's grandparent.
        // This is where parent_query comes in.
        if let Ok(grandparent) = parent_query.get(parent.get()) {
            level_to_wall_locations
                .entry(grandparent.get())
                .or_default()
                .insert(grid_coords);
        }
    });

    if !wall_query.is_empty() {
        level_query.iter().for_each(|(level_entity, level_iid)| {
            if let Some(level_walls) = level_to_wall_locations.get(&level_entity) {
                let ldtk_project = ldtk_project_assets
                    .get(ldtk_projects.single())
                    .expect("Project should be loaded if level has spawned");

                let level = ldtk_project
                    .as_standalone()
                    .get_loaded_level_by_iid(&level_iid.to_string())
                    .expect("Spawned level should exist in LDtk project");

                let LayerInstance {
                    c_wid: width,
                    c_hei: height,
                    grid_size,
                    ..
                } = level.layer_instances()[0];

                // combine wall tiles into flat "plates" in each individual row
                let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

                for y in 0..height {
                    let mut row_plates: Vec<Plate> = Vec::new();
                    let mut plate_start = None;

                    // + 1 to the width so the algorithm "terminates" plates that touch the right edge
                    for x in 0..width + 1 {
                        match (plate_start, level_walls.contains(&GridCoords { x, y })) {
                            (Some(s), false) => {
                                row_plates.push(Plate {
                                    left: s,
                                    right: x - 1,
                                });
                                plate_start = None;
                            }
                            (None, true) => plate_start = Some(x),
                            _ => (),
                        }
                    }

                    plate_stack.push(row_plates);
                }

                // combine "plates" into rectangles across multiple rows
                let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
                let mut prev_row: Vec<Plate> = Vec::new();
                let mut wall_rects: Vec<Rect> = Vec::new();

                // an extra empty row so the algorithm "finishes" the rects that touch the top edge
                plate_stack.push(Vec::new());

                for (y, current_row) in plate_stack.into_iter().enumerate() {
                    for prev_plate in &prev_row {
                        if !current_row.contains(prev_plate) {
                            // remove the finished rect so that the same plate in the future starts a new rect
                            if let Some(rect) = rect_builder.remove(prev_plate) {
                                wall_rects.push(rect);
                            }
                        }
                    }
                    for plate in &current_row {
                        rect_builder
                            .entry(plate.clone())
                            .and_modify(|e| e.top += 1)
                            .or_insert(Rect {
                                bottom: y as i32,
                                top: y as i32,
                                left: plate.left,
                                right: plate.right,
                            });
                    }
                    prev_row = current_row;
                }

                commands.entity(level_entity).with_children(|level| {
                    // Spawn colliders for every rectangle..
                    // Making the collider a child of the level serves two purposes:
                    // 1. Adjusts the transforms to be relative to the level for free
                    // 2. the colliders will be despawned automatically when levels unload
                    for wall_rect in wall_rects {
                        level
                            .spawn_empty()
                            .insert(Collider::cuboid(
                                (wall_rect.right as f32 - wall_rect.left as f32 + 1.)
                                    * grid_size as f32
                                    / 2.,
                                (wall_rect.top as f32 - wall_rect.bottom as f32 + 1.)
                                    * grid_size as f32
                                    / 2.,
                            ))
                            .insert(RigidBody::Fixed)
                            .insert(Friction::new(1.0))
                            .insert(Transform::from_xyz(
                                (wall_rect.left + wall_rect.right + 1) as f32 * grid_size as f32
                                    / 2.,
                                (wall_rect.bottom + wall_rect.top + 1) as f32 * grid_size as f32
                                    / 2.,
                                0.,
                            ))
                            .insert(GlobalTransform::default());
                    }
                });
            }
        });
    }
}

pub fn detect_climb_range(
    mut climbers: Query<&mut Climber>,
    climbables: Query<Entity, With<Climbable>>,
    mut collisions: EventReader<CollisionEvent>,
) {
    for collision in collisions.read() {
        match collision {
            CollisionEvent::Started(collider_a, collider_b, _) => {
                if let (Ok(mut climber), Ok(climbable)) = 
                    (climbers.get_mut(*collider_a), climbables.get(*collider_b))
                {
                    climber.intersecting_climbables.insert(climbable);
                } 

                if let (Ok(mut climber), Ok(climbable)) = 
                    (climbers.get_mut(*collider_b), climbables.get(*collider_a))
                {
                    climber.intersecting_climbables.insert(climbable);
                };
            }
            CollisionEvent::Stopped(collider_a, collider_b, _) => {
                if let (Ok(mut climber), Ok(climbable)) = 
                    (climbers.get_mut(*collider_a), climbables.get(*collider_b))
                {
                    climber.intersecting_climbables.remove(&climbable);
                } 

                if let (Ok(mut climber), Ok(climbable)) = 
                    (climbers.get_mut(*collider_b), climbables.get(*collider_a))
                {
                    climber.intersecting_climbables.remove(&climbable);
                };
            }
        }
    }
}

pub fn ignore_gravity_if_climbing(
    mut query: Query<(&Climber, &mut GravityScale), Changed<Climber>>,
) {
    for (climber, mut gravity_scale) in &mut query {
        if climber.climbing {
            gravity_scale.0 = 0.0;
        } else {
            gravity_scale.0 = 1.0;
        }
    }
}

pub fn spawn_ground_sensor(
    mut commands: Commands,
    detect_ground_for: Query<(Entity, &Collider), Added<GroundDetection>>,
) {
    for (entity, shape) in &detect_ground_for {
        if let Some(cuboid) = shape.as_cuboid() {
            let Vec2 {
                x: half_extents_x,
                y: half_extents_y,
            } = cuboid.half_extents();

            let detector_shape = Collider::cuboid(half_extents_x / 2.0, 2.0);

            let sensor_translation = Vec3::new(0.0, -half_extents_y, 0.0);

            commands.entity(entity).with_children(|builder| {
                builder
                    .spawn_empty()
                    .insert(ActiveEvents::COLLISION_EVENTS)
                    .insert(detector_shape)
                    .insert(Sensor)
                    .insert(Transform::from_translation(sensor_translation))
                    .insert(GlobalTransform::default())
                    .insert(GroundSensor {
                        ground_detection_entity: entity,
                        intersecting_ground_entities: HashSet::new(),
                    });
            });
        }
    }
}

pub fn ground_detection(
    mut ground_sensors: Query<&mut GroundSensor>,
    mut collisions: EventReader<CollisionEvent>,
    collidables: Query<Entity, (With<Collider>, Without<Sensor>)>,
) {
    for collision_event in collisions.read() {
        match collision_event {
            CollisionEvent::Started(e1, e2, _) => {
                if collidables.contains(*e1) {
                    if let Ok(mut sensor) = ground_sensors.get_mut(*e2) {
                        sensor.intersecting_ground_entities.insert(*e1);
                    }
                } else if collidables.contains(*e2) {
                    if let Ok(mut sensor) = ground_sensors.get_mut(*e1) {
                        sensor.intersecting_ground_entities.insert(*e2);
                    }
                }
            }
            CollisionEvent::Stopped(e1, e2, _) => {
                if collidables.contains(*e1) {
                    if let Ok(mut sensor) = ground_sensors.get_mut(*e2) {
                        sensor.intersecting_ground_entities.remove(e1);
                    }
                } else if collidables.contains(*e2) {
                    if let Ok(mut sensor) = ground_sensors.get_mut(*e1) {
                        sensor.intersecting_ground_entities.remove(e2);
                    }
                }
            }
        }
    }
}

pub fn update_on_ground(
    mut ground_detectors: Query<&mut GroundDetection>,
    ground_sensors: Query<&GroundSensor, Changed<GroundSensor>>,
) {
    for sensor in &ground_sensors {
        if let Ok(mut ground_detection) = ground_detectors.get_mut(sensor.ground_detection_entity) {
            ground_detection.on_ground = !sensor.intersecting_ground_entities.is_empty();
        }
    }
}

const ASPECT_RATIO: f32 = 16. / 9.;

#[allow(clippy::type_complexity)]
pub fn camera_fit_inside_current_level(
    mut camera_query: Query<
        (
            &mut bevy::render::camera::OrthographicProjection,
            &mut Transform,
        ),
        Without<Player>,
    >,
    player_query: Query<&Transform, With<Player>>,
    level_query: Query<(&Transform, &LevelIid), (Without<OrthographicProjection>, Without<Player>)>,
    ldtk_projects: Query<&Handle<LdtkProject>>,
    level_selection: Res<LevelSelection>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
) {
    if let Ok(Transform {
        translation: player_translation,
        ..
    }) = player_query.get_single()
    {
        let player_translation = *player_translation;

        let (mut orthographic_projection, mut camera_transform) = camera_query.single_mut();

        for (level_transform, level_iid) in &level_query {
            let ldtk_project = ldtk_project_assets
                .get(ldtk_projects.single())
                .expect("Project should be loaded if level has spawned");

            let level = ldtk_project
                .get_raw_level_by_iid(&level_iid.to_string())
                .expect("Spawned level should exist in LDtk project");

            if level_selection.is_match(&LevelIndices::default(), level) {
                let level_ratio = level.px_wid as f32 / level.px_hei as f32;
                orthographic_projection.viewport_origin = Vec2::ZERO;
                if level_ratio > ASPECT_RATIO {
                    // level is wider than the screen
                    let height = (level.px_hei as f32 / 9.).round() * 9.;
                    let width = height * ASPECT_RATIO;
                    orthographic_projection.scaling_mode =
                        bevy::render::camera::ScalingMode::Fixed { width, height };
                    camera_transform.translation.x =
                        (player_translation.x - level_transform.translation.x - width / 2.)
                            .clamp(0., level.px_wid as f32 - width);
                    camera_transform.translation.y = 0.;
                } else {
                    // level is taller than the screen
                    let width = (level.px_wid as f32 / 16.).round() * 16.;
                    let height = width / ASPECT_RATIO;
                    orthographic_projection.scaling_mode =
                        bevy::render::camera::ScalingMode::Fixed { width, height };
                    camera_transform.translation.y =
                        (player_translation.y - level_transform.translation.y - height / 2.)
                            .clamp(0., level.px_hei as f32 - height);
                    camera_transform.translation.x = 0.;
                }

                camera_transform.translation.x += level_transform.translation.x;
                camera_transform.translation.y += level_transform.translation.y;
            }
        }
    }
}

pub fn respawn_world(
    mut commands: Commands,
    ldtk_projects: Query<Entity, With<Handle<LdtkProject>>>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::R) {
        commands.entity(ldtk_projects.single()).insert(Respawn);
    }
}

pub fn patrol(
    mut query: Query<(&mut Transform, &mut Velocity, &mut Patrol)>,
) {
    for (mut transform, mut velocity, mut patrol) in &mut query {
        if patrol.points.len() <= 1 {
            continue;
        }

        let mut new_velocity = (patrol.points[patrol.index] - transform.translation.truncate()).normalize() * 75.0;

        if new_velocity.dot(velocity.linvel) < 0.0 {
            if patrol.index == 0 {
                patrol.forward = true;
            } else if patrol.index == patrol.points.len() - 1 {
                patrol.forward = false;
            }

            transform.translation.x = patrol.points[patrol.index].x;
            transform.translation.y = patrol.points[patrol.index].y;

            if patrol.forward {
                patrol.index += 1;
            } else {
                patrol.index -= 1;
            }

            new_velocity = (patrol.points[patrol.index] - transform.translation.truncate()).normalize() * 75.0;
        }

        velocity.linvel = new_velocity;
    }
}