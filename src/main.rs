use core::f32;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowResolution},
    input::mouse::AccumulatedMouseMotion
};
use rayon::prelude::*;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;
const FRAC_PI_4: f32 = std::f32::consts::FRAC_PI_4;
const MOVE_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = FRAC_PI_4;

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
        .add_systems(Startup, (setup /* enable_auto_indirect.after(setup) */,))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                update_cam,
                player_move,
                update_fps_text,
                march_rays
            ),
        )
        //.add_systems(PostUpdate, march_rays.after(TransformSystems::Propagate))
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
    ));

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
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(10.))),
        MeshMaterial3d(planet_material),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    //fps text
    commands
        .spawn((Text::new("Fps:"), TextColor(Color::BLACK)))
        .with_child((TextSpan::default(), TextColor(Color::BLACK), FpsText));
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

        let possible_pitch = ((camera_transform.rotation
            * pitch_rotation)
            * Vec3::NEG_Z)
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
    IVec2::new(
        p.x.floor() as i32,
        p.z.floor() as i32,
    )
}

//placeholder for a heightmap
fn get_height(x: i32, z: i32) -> f32 {
    if (x.pow(2) + z.pow(2)) < 5000*5000 {
        0.0
    } else {
        f32::NEG_INFINITY
    }
}


///Raymarches in 2D space along the xz plane, testing when the heightmap is collided with
fn march_rays(
    camera_query: Single<(&Camera, &GlobalTransform)>, 
    window: Single<&Window>,
) {
    let render_distance_squared = 5000.0 * 5000.0;


    let (camera, camera_transform) = *camera_query;

    let indicies: Vec<Vec2> = (0..window.width() as i32)
        .flat_map(|x| (0..window.height() as i32).map(move |y| Vec2::new(x as f32, y as f32)))
        .collect();

    let rays: Vec<Ray3d> = indicies
        .par_iter()
        .map(|pixel| camera.viewport_to_world(camera_transform, *pixel).unwrap())
        .collect();

    let testers: Vec<&Ray3d> = rays[0..100000].into_iter().collect();

    testers.into_par_iter().for_each(|ray| traverse(ray, render_distance_squared));



}


fn traverse(ray: &Ray3d, render_distance_squared: f32) {
    let start_pos = ray.origin.xz();
    let mut current_voxel: IVec2 = coord(ray.origin);
    let dir_inv = ray.direction.xz().recip(); 
    let mut ray_end = ray.origin; //a point at the tip of the ray

    //if the ray is pointing to the right, it will always intersect with the plane at x = voxel.x + 1, otherwise it will intersect with the plane at x = voxel.x
    // if the ray is pointing forward, it will always intersect with the plane at z = voxel.y + 1, otherwise it will intersect with the plane at z = voxel.y
    //-z is forward in Bevy
    let offset = Vec2::new(
        if ray.direction.x > 0.0 { 1.0 } else { 0.0 },
        if ray.direction.z < 0.0 { 1.0 } else { 0.0 },
    );

    let step = IVec2::new(
        if ray.direction.x > 0.0 { 1 } 
        else if ray.direction.x < 0.0 { -1 } 
        else { 0 },

        if ray.direction.z < 0.0 { 1 }
        else if ray.direction.z > 0.0 { -1 }
        else { 0 },
    );

    let mut t = (offset - start_pos) * dir_inv;
    let delta = ray.direction.xz().recip().abs();
    
    //println!("Orign: {:?}", ray.origin);

    //let angle = ray.direction.y.asin();
    //println!("Angle: {:.2} degrees", angle.to_degrees());
    loop {
        //Check for geometry within the current voxel
        let terrain_height = get_height(current_voxel.x, current_voxel.y);
        if ray_end.y <= terrain_height {
            //we have hit the terrain, so we can stop marching
            //println!("\nHit terrain at final position (  {:.2}, {:.2}, {:.2})", ray_end.x, ray_end.y, ray_end.z);
            //gizmos.arrow(ray_end, ray_end + Vec3::new(0.0, 2.0, 0.0), Color::BLACK);
            break;
        }
        //Traversal
        


        // see which plane is intersected first
        if t.x < t.y {
            //intersected with x plane first
            current_voxel.x += step.x;
            t.x += delta.x;
            ray_end = ray.origin + t.x * ray.direction;
        } else {
            //intersected with z plane first
            current_voxel.y += step.y;
            t.y += delta.y;
            ray_end = ray.origin + t.y * ray.direction;
        }

        //stop after reaching render distnace
        if ray_end.xz().distance_squared(start_pos) > render_distance_squared {
            //println!("\nReached render distance at final position ({:.2}, {:.2}, {:.2})", ray_end.x, ray_end.y, ray_end.z);
            //gizmos.arrow(ray_end, ray_end + Vec3::new(0.0, 2.0, 0.0), Color::BLACK);
            break;
        }
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
