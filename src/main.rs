mod terrain;
mod constructs;

use std::f32::consts::FRAC_PI_4;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin}, prelude::*, render::{
        render_resource::WgpuFeatures, renderer::RenderAdapterInfo, settings::{Backends, RenderCreation, WgpuSettings}, view::NoIndirectDrawing, RenderPlugin
    }, window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowResolution}
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
//use bevy_rich_text3d::Text3dPlugin;
use iyes_perf_ui::prelude::*;

use constructs::*;

use crate::terrain::PLANET_RADIUS;

const WINDOW_SCALE: f32 = 0.6;
const MOVE_SPEED: f32 = 1000.; // m/s


const WINDOW_WIDTH: f32 = 1920. * WINDOW_SCALE;
const WINDOW_HEIGHT: f32 = 1080. * WINDOW_SCALE;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                //for disabling fps cap (vsync)
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        title: "Planet Generator".into(),
                        resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                        position: WindowPosition::Centered(MonitorSelection::Primary),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                //for wireframe rendering
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        #[cfg(any(target_os = "windows", target_os = "linux"))]
                        backends: Some(Backends::VULKAN | Backends::DX12),
                        //backends: Some(Backends::DX12),
                        ..default()
                    }),
                    ..default()
                }),
            PanOrbitCameraPlugin,
            //Text3dPlugin {
            //    default_atlas_dimension: (1024, 1024),
            //    load_system_fonts: true,
                //load_font_directories: vec!["assets/fonts".to_owned()],
            //    ..Default::default()
            //},
        ))
        //stores chunks that are currently displayed
        .init_resource::<RenderedChunks>()
        //stores information of where the player is in simulated space (not rendered space)
        .init_resource::<PlayerInfo>()
        //sky color
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        //wireframe
        .add_plugins(WireframePlugin {..default()})
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })


        //diagnostics
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_systems(Startup, (
            setup, 
            enable_auto_indirect.after(setup),
            terrain::display_info,
            //terrain::show_coords,
        ))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                follow_cam,
                player_move,
                terrain::manage_chunks,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut player_info: ResMut<PlayerInfo>,
) {
    // camera
    commands.spawn((
        //Transform::from_translation(Vec3::new(terrain::PLANET_RADIUS*2., terrain::PLANET_RADIUS*2., terrain::PLANET_RADIUS*2.)),
        PanOrbitCamera {
            allow_upside_down: true,
            radius: Some(50.),
            axis: [Vec3::X, Vec3::Y, Vec3::NEG_Z],
            ..default()
        },
        //NoIndirectDrawing,
    ));

    // lock mouse into window by default
    if let Ok(mut primary_window) = q_windows.single_mut() {
        primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor_options.visible = false;
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

    // performance UI
    commands.spawn((
        PerfUiEntryFPS::default(),
        PerfUiEntryFPSAverage::default(),
        PerfUiEntryFrameTime::default(),
        PerfUiEntryRenderCpuTime::default(),
        PerfUiEntryRenderGpuTime::default(),
        PerfUiEntryEntityCount::default(),  

    ));

    //Player mockup
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 2., 1.))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.3))),
        //Transform::from_xyz(0., PLANET_RADIUS+1., 0.),
        Player{facing: Vec3::NEG_Z},
    ));
    player_info.position = Vec3::new(0., PLANET_RADIUS+1., 0.);
    player_info.offset = Vec3::new(0., -(PLANET_RADIUS+1.), 0.);
    player_info.facing = Vec3::NEG_Z;

    //setup global material handle
    commands.insert_resource(PlanetMaterial(materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.49, 0.22), // deep green
        metallic: 0.0,               // organic = dielectric
        perceptual_roughness: 0.90,  // very diffuse, kills sparkle
        reflectance: 0.04,           // ~4% F0 for leaves/plant matter
        alpha_mode: AlphaMode::Opaque,
        ..default()
    })));
}

fn grab_mouse(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut primary_window) = q_windows.single_mut() {
        if mouse.just_pressed(MouseButton::Left) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
            primary_window.cursor_options.visible = false;
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));
        }

        if key.just_pressed(KeyCode::Escape) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::None;
            primary_window.cursor_options.visible = true;
        }
    }
}

fn enable_auto_indirect(
    info: Res<RenderAdapterInfo>,
    mut commands: Commands,
    cameras: Query<Entity, With<PanOrbitCamera>>,
) {
     let is_intel = info.vendor == 0x8086 || info.vendor == 32902;
     if is_intel {
        println!("Enabling NoIndirectDrawing for Intel GPUs to avoid issues with indirect drawing.");
        for entity in &cameras {
            commands.entity(entity).insert(NoIndirectDrawing);
        }
     }
}

fn toggle_wireframe(
    key: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<WireframeConfig>,
) {
    if key.just_pressed(KeyCode::Space) {
        config.global = !config.global;
        //info!("Wireframe mode: {}", if config.global { "ON" } else { "OFF" });
    }
}

fn follow_cam(
    mut pan_orbit_q: Query<&mut PanOrbitCamera>,
    player_q: Query<&Transform, With<Player>>
) {
    if let Ok(mut pan_orbit) = pan_orbit_q.single_mut() {
        if let Ok(player_transform) = player_q.single() {
            let pos = player_transform.translation;
            
            //lock camera on player position
            pan_orbit.target_focus = pos;
            

            //rotation
            /* 
            let rot = player_transform.rotation;
            let forward =  rot * Vec3::NEG_Z;
            let up = rot * Vec3::Y;
            let right =  rot * Vec3::X;
            pan_orbit.axis = [right,up,forward];
            */

            pan_orbit.force_update = true;

        }
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &mut Transform)>,
    mut player_info: ResMut<PlayerInfo>,
    mut all_chunks: Query<&mut Transform, (With<Chunk>, Without<Player>)>,
) {

        
        let mut pos = player_info.position;

        let up = pos.normalize();
        //re-tangent and normalize (basically made sure player.facing is actually perpendicular to up)
        let forward = (player_info.facing - up * player_info.facing.dot(up)).normalize();
        let right = forward.cross(up).normalize();


        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) { input.y += 1.0; }
        if keys.pressed(KeyCode::KeyS) { input.y -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { input.x += 1.0; }
        if keys.pressed(KeyCode::KeyA) { input.x -= 1.0; }
        input = input.normalize();

        let move_direction = (right * input.x + forward * input.y).normalize();

        //rotate around this axis to simulate movement
        let axis = up.cross(move_direction);
        let axis_len = axis.length();

        //prevents NAN issues when no input is given
        if axis_len > 1e-6 {
            let axis_n = axis / axis_len;
            let angle = (MOVE_SPEED * time.delta_secs()) / (PLANET_RADIUS);
            let rotation = Quat::from_axis_angle(axis_n, angle);
            pos = rotation * pos;

            //make sure we remain on the surface of the planet (corrects any floating point errors)
            pos = pos.normalize() * (PLANET_RADIUS+1.);

            //update simulated position and rotation
            player_info.position = pos;

            let up = pos.normalize();
            let forward = (player_info.facing - up * player_info.facing.dot(up)).normalize();
            player_info.facing = forward;


            //update rendered position and rotation
            if let Ok((mut player, mut player_transform)) = player_q.single_mut() {
                player_transform.translation = pos+player_info.offset;
                player_transform.look_to(forward, up);
                player.facing = forward;
                //display new position in render space for debbugging
                //println!("Player Render Pos: {}", player_transform.translation);

                //println!("{}", player_transform.translation.length_squared());

                //println!("{}", player_info.position.length());


                //update offset if we've strayed too far from origin
                if player_transform.translation.length_squared() > 2_000_000_000. {
                    
                    //move all the chunks back to the origin
                    for mut chunk in all_chunks.iter_mut() {
                        chunk.translation -= player_transform.translation;
                    }

                    //update the offset and move the player back
                    player_info.offset = player_info.offset - player_transform.translation;
                    player_transform.translation = Vec3::ZERO;

                    println!("Offset updated: {}", player_info.offset);
            }


            
            //display height for debugging
            //println!("Height: {}", player_transform.translation.distance(player_info.offset) - PLANET_RADIUS);
        }
    }

    

}
