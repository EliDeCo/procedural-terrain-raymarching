use std::f32::consts::FRAC_PI_4;

use bevy::{
    prelude::*,
    render::{
        render_resource::WgpuFeatures, renderer::RenderAdapterInfo, settings::{Backends, RenderCreation, WgpuSettings}, view::NoIndirectDrawing, RenderPlugin
    },
    window::{CursorGrabMode, PresentMode, PrimaryWindow},
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;
//use noise::{NoiseFn, Perlin};

const PLANET_RADIUS: f32 = 10.0; // in meters
//const CHUNK_SIZE: f32 = 10.0; // in meters
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                //for disabling fps cap (vsync)
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
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
        ))
        //sky color
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_systems(Startup, (
            setup, 
            enable_auto_indirect.after(setup), 
        ))
        .add_systems(
            Update,
            (
                grab_mouse,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 200.0, 0.0)),
        PanOrbitCamera::default(),
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

    // spawn basic cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(PLANET_RADIUS, PLANET_RADIUS, PLANET_RADIUS))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.5, 0.3))),
    ));

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
        for entity in &cameras {
            commands.entity(entity).insert(NoIndirectDrawing);
        }
     }
}