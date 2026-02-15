use std::{collections::HashMap, f32::EPSILON};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowResolution},
};
use rayon::prelude::*;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;
const FRAC_PI_4: f32 = std::f32::consts::FRAC_PI_4;
const MOVE_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = FRAC_PI_4;
const VOXEL_SIZE: f32 = 2.0;
const INV_VOXEL_SIZE: f32 = 1.0 / VOXEL_SIZE as f32;

//TODO: Simply compute Integer Hash Noise every time height is sampled instead of wasting time and memory on a hashmap lookup
//TODO: Make your own noise algorithm that uses integers or something
//TODO: Remove unneccessary tio_arry
//Todo: Near vertical ray calulations

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    title: "Bevy".into(),
                    resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_plugins(WireframePlugin { ..default() })
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .init_resource::<TerrainStore>()
        .add_systems(Startup, (setup /* enable_auto_indirect.after(setup) */,))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                update_cam,
                player_move,
                update_fps_text,
                //march_rays
            ),
        )
        .add_systems(PostUpdate, march_rays.after(TransformSystems::Propagate))
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_store: ResMut<TerrainStore>,
) {
    // camera
    commands.spawn((Camera3d::default(),));

    // lock mouse into window by default
    if let Ok((mut primary_window, mut cursor_options)) = q_window.single_mut() {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
        let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
        primary_window.set_cursor_position(Some(center));
    }

    //add sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -FRAC_PI_4, -FRAC_PI_4, 0.0)),
    ));

    //Player mockup
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 2., 1.))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.3))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Player {
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            right: Vec3::X,
        },
    ));

    //setup global material handle
    let planet_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.49, 0.22),
        metallic: 0.0,
        perceptual_roughness: 0.90,
        reflectance: 0.04,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    commands.insert_resource(PlanetMaterial(planet_material.clone()));

    //terrain base
    /*
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(20.0, 20.)
                    .subdivisions(9)
                    .build(),
            ),
        ),
        MeshMaterial3d(planet_material),
    ));
    */

    //fps text
    commands
        .spawn((Text::new("Fps:"), TextColor(Color::BLACK)))
        .with_child((TextSpan::default(), TextColor(Color::BLACK), FpsText));

    let mut to_add = Vec::new();
    for x in -4..4 {
        for y in -4..4 {
            let coords = IVec2::new(x, y);
            to_add.push(
                (coords,QuadInfo::new_simple(coords))
            );
        }
    }

    terrain_store.quad_map.extend(to_add);
}

fn grab_mouse(
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if let Ok((mut primary_window, mut cursor_options)) = q_window.single_mut() {
        if mouse.just_pressed(MouseButton::Left) {
            cursor_options.grab_mode = CursorGrabMode::Locked;
            cursor_options.visible = false;
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));
        }

        if key.just_pressed(KeyCode::Escape) {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

fn toggle_wireframe(key: Res<ButtonInput<KeyCode>>, mut config: ResMut<WireframeConfig>) {
    if key.just_pressed(KeyCode::Space) {
        config.global = !config.global;
    }
}

fn update_cam(
    camera_q: Single<&mut Transform, With<Camera>>,
    player_q: Single<&Transform, (With<Player>, Without<Camera>)>,
    acc_mouse_motion: Res<AccumulatedMouseMotion>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    cursor_q: Single<&CursorOptions, With<PrimaryWindow>>,
) {
    let mut camera_transform = camera_q.into_inner();
    let cursor_options = *cursor_q;
    let player_transform = *player_q;

    camera_transform.translation = player_transform.translation + Vec3::new(0.0, 1.75, 0.0);

    if cursor_options.grab_mode != CursorGrabMode::Locked {
        mouse.release_all();
        return;
    }

    let delta = acc_mouse_motion.delta;
    if delta != Vec2::ZERO {
        let yaw_rotation = Quat::from_rotation_y(-delta.x * 0.003);
        let pitch_rotation = Quat::from_rotation_x(-delta.y * 0.002);

        camera_transform.rotation *= yaw_rotation;

        let possible_pitch = ((camera_transform.rotation * pitch_rotation) * Vec3::NEG_Z)
            .y
            .asin();
        if possible_pitch.abs() < PITCH_LIMIT {
            camera_transform.rotation *= pitch_rotation;
        }
        let forward: Vec3 = camera_transform.forward().into();
        //keep roll at zero
        camera_transform.look_to(forward, Vec3::Y);
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &mut Transform)>,
) {
    if let Ok((player, mut player_transform)) = player_q.single_mut() {
        let mut input = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyE) {
            input.z += 1.0;
        }
        if keys.pressed(KeyCode::KeyQ) {
            input.z -= 1.0;
        }
        if input != Vec3::ZERO {
            input = input.normalize();
        }

        let move_direction =
            (player.right * input.x + player.forward * input.y + player.up * input.z).normalize();

        if input.length_squared() > 1e-6 {
            player_transform.translation += move_direction * MOVE_SPEED * time.delta_secs();
        }
    }
}

/// Updates the FPS text display.
fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut TextSpan, With<FpsText>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if let Ok(mut span) = query.single_mut() {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(value) = fps.smoothed()
        {
            let pos = if let Ok(player_transform) = player_q.single() {
                player_transform.translation
            } else {
                Vec3::ZERO
            };
            let coords = coord(pos);
            **span = format!("{value:.2}  {coords}");
        }
    }
}

///converts world position to horizontal voxel coordinates
fn coord(p: Vec3) -> IVec2 {
    (p.xz() * INV_VOXEL_SIZE).floor().as_ivec2()
}

//get the height of a given point in world coornates
fn get_height(x: i32, z: i32) -> f32 {
    let _ = x + z;
    0.0
}

///Raymarches in 2D space along the xz plane, testing when the heightmap is collided with
fn march_rays(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window>,
    mut gizmos: Gizmos,
    terrain_store: ResMut<TerrainStore>,
) {
    //TEMPORARY
    let max_height = 10.0;

    let (camera, camera_transform) = *camera_query;

    //for testing
    let width = window.width();
    let height = window.height();
    let row1 = height * 0.6;
    let row2 = height * 0.8;
    let columns: Vec<f32> = (1..8).map(|x| width * (x as f32 * 0.125)).collect();

    let pixels: Vec<Vec2> = [row1, row2]
        .iter()
        .flat_map(|row| {
            columns
                .iter()
                .map(|col| Vec2::new(col.to_owned(), row.to_owned()))
        })
        .collect();

    for pixel in pixels {
        let ray = camera.viewport_to_world(camera_transform, pixel).unwrap();
        traverse(&ray, max_height, &terrain_store, &mut gizmos);
    }
    
    //temporary "rendering"
    for (_,quad) in &terrain_store.quad_map {
        let offset = IVec2::new(quad.coords.x as i32, quad.coords.y as i32);
        let v1 = offset;
        let v2 = offset + IVec2::new(0, VOXEL_SIZE as i32);
        let v3 = offset + IVec2::new(VOXEL_SIZE as i32, 0);
        let v4 = offset + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32);
        

        for edge in [(v1,v2),(v3,v4),(v1,v4),(v1,v3),(v2,v4)] {
            let start = Vec3::new(edge.0.x as f32, 0.0, edge.0.y as f32);
            let end = Vec3::new(edge.1.x as f32, 0.0, edge.1.y as f32);
            gizmos.line(start, end, Color::BLACK);
        }
    }

    //fastest method
    /*
    (0..window.width() as i32)
        .into_par_iter()
        .for_each(|x| {
            for y in 0..window.height() as i32 {
                let pixel = Vec2::new(x as f32, y as f32);
                let ray = camera.viewport_to_world(camera_transform, pixel).unwrap();
                traverse(&ray, render_distance, max_height);
            }
        });
    */
}

fn traverse(ray: &Ray3d, max_height: f32, terrain_store: &TerrainStore, gizmos: &mut Gizmos) {
    let mut current_voxel: IVec2 = coord(ray.origin);
    let ray_origin_y = ray.origin.y;
    let mut ray_end_y = ray_origin_y;
    let ray_dir_y = ray.direction.y;
    let ray_dir_xz = ray.direction.xz();
    let tilted_up = if ray_dir_y > 0.0 { true } else { false };

    if ray_dir_xz.length_squared() < EPSILON {
        //if y is postive, no collision will happen and skip entirely
        if ray_dir_y.is_sign_positive() { return; }

        
        //if y is negative, simply find terrain height in the starting voxel and use that as our final value
        //TODO: IMPLIMENT THIS
        info!("Vertical ray");
        return;
    }

    //collision offest in voxel coordinates
    let offset = Vec2::new(
        if ray.direction.x > 0.0 { 1.0 } else { 0.0 },
        if ray.direction.z > 0.0 { 1.0 } else { 0.0 },
    );

    //the next boundary for collison testing in world coordinates
    let next_boundary = Vec2::new(
        (current_voxel.x as f32 + offset.x) * VOXEL_SIZE,
        (current_voxel.y as f32 + offset.y) * VOXEL_SIZE,
    );

    //must be initalized per component so we can insert INFINITY to avoid NaN
    let mut t = Vec2::ZERO;
    let mut delta = Vec2::ZERO;

    // X axis
    if ray_dir_xz.x != 0.0 {
        let inv = 1.0 / ray_dir_xz.x;
        delta.x = VOXEL_SIZE * inv.abs();
        t.x = (next_boundary.x - ray.origin.x) * inv;
    } else {
        delta.x = f32::INFINITY;
        t.x = f32::INFINITY;
    }

    // Z axis
    if ray_dir_xz.y != 0.0 {
        let inv = 1.0 / ray_dir_xz.y;
        delta.y = VOXEL_SIZE * inv.abs();
        t.y = (next_boundary.y - ray.origin.z) * inv;
    } else {
        delta.y = f32::INFINITY;
        t.y = f32::INFINITY;
    }

    //which way to step in voxel coordinates
    let step = IVec2::new(
        if ray.direction.x > 0.0 {
            1
        } else if ray.direction.x < 0.0 {
            -1
        } else {
            0
        },
        if ray.direction.z > 0.0 {
            1
        } else if ray.direction.z < 0.0 {
            -1
        } else {
            0
        },
    );

    let mut t_current = 0.0;
    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end_y > max_height {
            break;
        }

        //collision detection
        let t_next = t.x.min(t.y);

        let enter_point = ray.origin + ray.direction * t_current;
        let exit_point  = ray.origin + ray.direction * t_next;

        //TODO: OUTSIDE RENDER DISTANCE ERROR! WE ARE GETTING NONE WERE THE VOXEL SHOULD BE SOME
        if let Some(voxel) = terrain_store.quad_map.get(&current_voxel) {
            if let Some(point) = voxel.intersect(enter_point, exit_point, ray) {
                //ray has hit terrain
                gizmos.sphere(Isometry3d::from_translation(point), 0.25, Color::BLACK);
                break;
            }
        } else {
            //we are outside of currently rendered chunks (aka past render distance)
            gizmos.sphere(Isometry3d::from_translation(enter_point), 0.15, Color::WHITE);
            break;
        }



        //Traversal
        // see which plane is intersected first

        if t.x < t.y {
            //intersected with x plane first
            current_voxel.x += step.x;
            t.x += delta.x;
        } else {
            //intersected with z plane first
            current_voxel.y += step.y;
            t.y += delta.y;
            
        }

        t_current = t_next;
        ray_end_y = ray_origin_y + t_current * ray_dir_y;
    }
}

///used for identifying the player entity
#[derive(Component)]
pub struct Player {
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
}

///default material for terrain
#[derive(Resource, Clone)]
pub struct PlanetMaterial(pub Handle<StandardMaterial>);

///marker component for fps text
#[derive(Component)]
struct FpsText;

///Struct for holding all currently relevant terrain data
#[derive(Resource, Default)]
struct TerrainStore {
    quad_map: HashMap<IVec2, QuadInfo>,
}

impl TerrainStore {
    ///Tests if any of the recently added quads have a higher max height than the current value,
    /// and updates accordingly
    fn add_test(added: &[QuadInfo], max_height: &mut MaxHeight) {
        let max_added = added.iter().map(|q| q.y_max).reduce(f32::max).unwrap_or(0.0);
        if max_added > max_height.0 {
            max_height.0 = max_added;
        }
    }

    ///Tests if the quad with the largest max height was removed, and
    /// updates accordingly
    fn remove_test(&self, removed: &[QuadInfo], max_height: &mut MaxHeight) {
        for quad in removed {
            if quad.y_max == max_height.0 {
                max_height.0 = self
                    .quad_map
                    .iter()
                    .map(|(_, q)| q.y_max)
                    .reduce(f32::min)
                    .unwrap_or(0.0);
                break;
            }
        }

    }
}

///Plane struct optimized for ray-plane intersection
struct SimplePlane {
    n: Vec3, // normalized normal
    d: f32,  // plane constant
}

///Stores all relevant information of a quad for ray intersection calculations
struct QuadInfo {
    coords: Vec2, //coordinates of the vertex with the lowest (closest to negative infinity) x and z values in world space
    upper: SimplePlane,
    lower: SimplePlane,
    y_max: f32,
    y_min: f32,
}

impl QuadInfo {
    ///Creates a new quad of a given size, with a given offset in voxel coordinates
    fn new_simple(offset: IVec2) -> Self {
        let offset = VOXEL_SIZE as i32 * offset;
        let v1 = offset.to_array();
        let v2 = (offset + IVec2::new(0, VOXEL_SIZE as i32)).to_array();
        let v3 = (offset + IVec2::new(VOXEL_SIZE as i32, 0)).to_array();
        let v4 = (offset + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32)).to_array();


        return QuadInfo::new([v1, v2, v3, v4]);
    }

    ///Creates a new quad from 4 vertexes (given as [[x,z]; 4])
    fn new(vertexes: [[i32; 2]; 4]) -> Self {
        // Find min and max X/Z
        let min_x = vertexes.iter().map(|v| v[0]).min().unwrap();
        let min_z = vertexes.iter().map(|v| v[1]).min().unwrap();
        let max_x = vertexes.iter().map(|v| v[0]).max().unwrap();
        let max_z = vertexes.iter().map(|v| v[1]).max().unwrap();

        let coords = Vec2::new(min_x as f32, min_z as f32);

        // Heights at the 4 corners
        let y0 = get_height(min_x, min_z);
        let y1 = get_height(max_x, min_z);
        let y2 = get_height(min_x, max_z);
        let y3 = get_height(max_x, max_z);

        // Compute min/max Y
        let y_max = y0.max(y1).max(y2).max(y3);
        let y_min = y0.min(y1).min(y2).min(y3);

        // Construct lower plane (right angle at min_x, min_z)
        let lower = {
            let nx = -(y1 - y0) / VOXEL_SIZE;
            let ny = 1.0;
            let nz = -(y2 - y0) / VOXEL_SIZE;
            let n = Vec3::new(nx, ny, nz).normalize();
            let d = -n.dot(Vec3::new(0.0, y0, 0.0)); // local 0,0 corner
            SimplePlane { n, d }
        };

        // Construct upper plane (right angle at max_x, max_z)
        let upper = {
            let nx = -(y3 - y2) / VOXEL_SIZE;
            let ny = 1.0;
            let nz = -(y3 - y1) / VOXEL_SIZE;
            let n = Vec3::new(nx, ny, nz).normalize();
            let d = -n.dot(Vec3::new(VOXEL_SIZE, y3, VOXEL_SIZE)); // local VOXEL_SIZE,VOXEL_SIZE
            SimplePlane { n, d }
        };

        QuadInfo {
            coords,
            upper,
            lower,
            y_max,
            y_min,
        }
    }

    ///Returns the point where a 3d ray intersects with terrain generated by a heightmap stored as quads (if it exists)
    fn intersect(&self, enter_point: Vec3, exit_point: Vec3, ray: &Ray3d) -> Option<Vec3> {
        
        if (enter_point.y > self.y_max && exit_point.y > self.y_max)
            || (enter_point.y < self.y_min && exit_point.y < self.y_min)
        {
            return None; //fully above terrain or below terrain
        }
        

        //convert to local
        let local_enter_x = enter_point.x - self.coords.x;
        let local_enter_z = enter_point.z - self.coords.y;

        //does the ray enter from the bottom or left?
        //if so test the lower triangle first
        if local_enter_x.abs() < f32::EPSILON || local_enter_z.abs() < f32::EPSILON {
            if let Some(point) = self.test_lower(ray) {
                return Some(point);
            }

            if let Some(point) = self.test_upper(ray) {
                return Some(point);
            }
        } else {
            //passing through upper triangle first
            if let Some(point) = self.test_upper(ray) {
                return Some(point);
            }

            if let Some(point) = self.test_lower(ray) {
                return Some(point);
            };
        }

        None
    }

    ///Returns the 3d point where the ray intersects with the lower triangle (if it exists)
    fn test_lower(&self, ray: &Ray3d) -> Option<Vec3> {
        let point = self.lower.ray_plane(ray);

        if let Some(p) = point {
            //if the ray does intersect the plane
            let point_2d = Vec2::new(p.x, p.z) - self.coords; //convert to local

            if let Some(classification) = classify(point_2d)
                && classification
            {
                return point; //if it does indeed intersect the plane within the lower triangle
            } else {
                return None; //either it intersects withint he upper triangle (wrong plane) or doesn't intersect at al
            }
        }
        None
    }

    ///Returns the 3d point where the ray intersects with the upper triangle (if it exists)
    fn test_upper(&self, ray: &Ray3d) -> Option<Vec3> {
        let point = self.upper.ray_plane(ray);

        if let Some(p) = point {
            //if the ray does intersect the plane
            let point_2d = Vec2::new(p.x, p.z) - self.coords; //convert to local

            if let Some(classification) = classify(point_2d)
                && !classification
            {
                return point; //if it does indeed intersect the plane within the upper triangle
            } else {
                return None; //either it intersects withint he lower triangle (wrong plane) or doesn't intersect at all
            }
        }
        None
    }
}

impl SimplePlane {
    ///Returns the 3d point where the ray intersects the plane, or None if near parallel
    fn ray_plane(&self, ray: &Ray3d) -> Option<Vec3> {
        // denominator = dot(n, ray direction)
        let denom = self.n.dot(ray.direction.into());

        // Parallel (or extremely close to parallel)
        if denom.abs() < f32::EPSILON {
            return None;
        }

        let t = -(self.n.dot(ray.origin) + self.d) / denom;

        // Optional: reject intersections behind the ray
        if t < 0.0 {
            return None;
        }

        Some(ray.origin + ray.direction * t)
    }
}

///Classifies points as outide (none) lower (true) or upper (false)
/// With the lower triangle being bound by the local x and z axis
/// and the upper triangle being bound by the local x = VOXEL_SIZE and z = VOXEL_SIZE
fn classify(point: Vec2) -> Option<bool> {
    let x = point.x;
    let z = point.y;

    if x < 0.0 || z < 0.0 || x > VOXEL_SIZE || z > VOXEL_SIZE {
        return None; // outside quad
    }

    Some(x + z <= VOXEL_SIZE) // true = lower, false = upper
}

#[derive(Resource)]
///The highest currently relevant terrain
struct MaxHeight(f32);
