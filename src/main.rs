use bevy::anti_alias::fxaa::*;
use bevy::{
    anti_alias::fxaa::Fxaa,
    core_pipeline::{
        FullscreenShader,
        core_3d::graph::{Core3d, Node3d},
    },
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::query::QueryItem,
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BlendState, Buffer, BufferInitDescriptor, BufferUsages, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, PipelineCache, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, ShaderType, TextureFormat, UniformBuffer,
            binding_types::{storage_buffer_read_only, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ViewTarget,
    },
    window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowResolution},
};
use bytemuck::{NoUninit, cast_slice};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;

//INPUTS
const DESIRED_VOXEL_SIZE: u32 = 15;
const RENDER_DISTANCE: f32 = 5000.;
const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;
const MOVE_SPEED: f32 = 10.0;

//CONSTANTS
const FRAC_PI_4: f32 = std::f32::consts::FRAC_PI_4;
const PITCH_LIMIT: f32 = FRAC_PI_4;
const SHADER_ASSET_PATH: &str = "shaders/my_shader.wgsl";

//PRECALCULATIONS
const TOTAL_SPAN: f32 = RENDER_DISTANCE * 2.;
const DESIRED_CHUNKS_PER_EDGE: u32 = (TOTAL_SPAN / DESIRED_VOXEL_SIZE as f32) as u32;
const BUFFER_SIZE: i32 = nearest_power_of_two(DESIRED_CHUNKS_PER_EDGE) as i32;
const RENDER_DIST_VOXELS: i32 = BUFFER_SIZE >> 1;
const VOXEL_SIZE: f32 = ((TOTAL_SPAN / BUFFER_SIZE as f32) as u32) as f32; //must be whole number, but type f32 to avoid casting u32 -> f32 frequently
const INV_VOXEL_SIZE: f32 = 1.0 / VOXEL_SIZE;
const BUFFER_MASK:  usize = (BUFFER_SIZE - 1) as usize;
const BUFFER_SHIFT: usize = BUFFER_SIZE.trailing_zeros() as usize;


const fn nearest_power_of_two(x: u32) -> u32 {
    if x.is_power_of_two() { return x; }

    let next = x.next_power_of_two();
    let prev = next >> 1;

    // Compare which is closer
    if x - prev <= next - x {
        prev
    } else {
        next
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    title: "Bevy".into(),
                    resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resizable: true,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_plugins(ShaderPlugin)
        //.insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .init_resource::<TerrainStore>()
        .add_systems(Startup, (setup /*init_terrain.after(setup)*/,))
        .add_systems(
            Update,
            (
                grab_mouse,
                update_cam,
                player_move,
                update_fps_text,
                //update_terrain,
            ),
        )
        /*
        .add_systems(PostUpdate,
            march_rays.after(TransformSystems::Propagate)
        )
        */
        .run();
}
//Reference
//https://bevy.org/examples/shaders/custom-post-processing/
struct ShaderPlugin;
impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            //for components that live in the main world but will be extracted to the render world every frame
            ExtractComponentPlugin::<Uniform>::default(),
            UniformComponentPlugin::<Uniform>::default(),
        ));
        app.add_systems(Update, stage_terrain_updates);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_custom_pipeline);
        render_app.add_systems(ExtractSchedule, extract_data);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
        );

        render_app
            .add_render_graph_node::<ViewNodeRunner<CustomNode>>(Core3d, MyPassLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    MyPassLabel, //run before the main pass
                    Node3d::StartMainPass,
                ),
            );
    }
}

fn init_custom_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
) {
    //initialize materials
    let planet_material = GpuMaterial::from_standardmaterial(
        StandardMaterial {
            base_color: Color::srgb(0.34, 0.49, 0.22),
            metallic: 0.0,
            perceptual_roughness: 0.90,
            reflectance: 0.04,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        },
        0.0,
    );

    let materials = [planet_material];

    let mut material_buffer = UniformBuffer::default();
    material_buffer.set(materials);
    material_buffer.write_buffer(&render_device, &render_queue);

    let static_layout = BindGroupLayoutDescriptor::new(
        "static_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (uniform_buffer::<[GpuMaterial; 1]>(false),),
        ),
    );

    let static_bind_group = render_device.create_bind_group(
        "static_bind_group",
        &pipeline_cache.get_bind_group_layout(&static_layout),
        &BindGroupEntries::sequential((material_buffer.binding().unwrap(),)),
    );

    //initialize terrain
    let quads = TerrainStore::default();
    let quad_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("quad_buffer"),
        contents: cast_slice(&quads.quad_buffer),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let layout = BindGroupLayoutDescriptor::new(
        "per_frame_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<Uniform>(false),
                storage_buffer_read_only::<GpuQuadInfo>(false),
            ),
        ),
    );

    let shader = asset_server.load(SHADER_ASSET_PATH);

    //allows us to skip writing a vertex shader
    let vertex_state = fullscreen_shader.to_vertex_state();

    // This will add the pipeline to the cache and queue its creation
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("custom_pass_pipeline".into()),
        layout: vec![layout.clone(), static_layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    commands.insert_resource(MyPipeline {
        pipeline_id,
        layout,
        static_bind_group,
        quad_buffer,
    });
}

#[derive(Resource)]
struct MyPipeline {
    pipeline_id: CachedRenderPipelineId,
    layout: BindGroupLayoutDescriptor,
    static_bind_group: BindGroup,
    quad_buffer: Buffer,
}

#[derive(Default)]
struct CustomNode;

fn extract_data(
    mut commands: Commands,
    window: Extract<Single<&Window, With<PrimaryWindow>>>,
    camera: Extract<Single<(&Camera, &GlobalTransform)>>,
    pending: Extract<Option<Res<PendingTerrainChanges>>>,
    max_height: Extract<Res<MaxHeight>>,
) {
    let (camera, transform) = (camera.0, camera.1);
    let clip_matrix = camera.clip_from_view();
    let transform_matrix = transform.to_matrix();
    let world_from_clip = transform_matrix * clip_matrix.inverse();

    commands.insert_resource(Uniform {
        resolution: Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        ),
        world_from_clip,
        buffer_mask: BUFFER_MASK as u32,
        buffer_shift: BUFFER_SHIFT as u32,
        render_distance: RENDER_DISTANCE,
        voxel_size: VOXEL_SIZE,
        inv_voxel_size: INV_VOXEL_SIZE,
        buffer_size: BUFFER_SIZE as u32,
        max_height: max_height.0,
    });
    //reinsert for the render world
    if let Some(pending) = pending.as_ref() {
        commands.insert_resource(PendingTerrainChanges {
            changes: pending.changes.clone(),
        });
    } else {
        //clear if no new changes needed
        commands.insert_resource(PendingTerrainChanges {
            changes: Vec::new(),
        });
    }
}

///Updates group of information sent to the render world
fn prepare_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    world: &World,
    //q_window: Single<&Window, With<PrimaryWindow>>
) {
    //println!("1.0");
    let Some(pipeline) = world.get_resource::<MyPipeline>() else {
        return;
    };
    //println!("1.1");
    let Some(uniform) = world.get_resource::<Uniform>() else {
        return;
    };
    //println!("1.2");
    let Some(pipeline_cache) = world.get_resource::<PipelineCache>() else {
        return;
    };

    let staged_changes = world.get_resource::<PendingTerrainChanges>();

    let layout = pipeline_cache.get_bind_group_layout(&pipeline.layout);

    let mut u_buffer = UniformBuffer::default();
    u_buffer.set(*uniform);
    u_buffer.write_buffer(&render_device, &render_queue);

    //split into chunks that are runs
    if let Some(staged_changes) = staged_changes
        && !staged_changes.changes.is_empty()
    {
        let changes = &staged_changes.changes;
        //println!("updating {} quads", changes.len());

        let mut chunks = vec![vec![&changes[0]]];
        let mut last = changes[0].0;
        for change in changes.iter().skip(1) {
            if change.0 == last + 1 {
                chunks.last_mut().unwrap().push(change);
                last += 1;
            } else {
                chunks.push(vec![change]);
                last = change.0;
            }
        }

        for run in chunks {
            let start = run[0].0;
            let data: Vec<GpuQuadInfo> = run.iter().map(|g| g.1).collect();
            let byte_offset = (start * std::mem::size_of::<GpuQuadInfo>()) as u64;

            render_queue.write_buffer(&pipeline.quad_buffer, byte_offset, cast_slice(&data));
        }
    }

    let bind_group = render_device.create_bind_group(
        "my_pipeline_bind_group",
        &layout,
        &BindGroupEntries::sequential((
            u_buffer.binding().unwrap(),
            pipeline.quad_buffer.as_entire_binding(),
        )),
    );

    commands.insert_resource(MyBindGroup { bind_group });
}

#[derive(Resource)]
struct MyBindGroup {
    bind_group: BindGroup,
}

impl ViewNode for CustomNode {
    type ViewQuery = &'static ViewTarget;

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        //println!("0");
        let Some(pipeline_cache) = world.get_resource::<PipelineCache>() else {
            return Ok(());
        };
        //println!("1");

        let Some(my_pipeline) = world.get_resource::<MyPipeline>() else {
            return Ok(());
        };
        //println!("2");

        let Some(bind_group) = world.get_resource::<MyBindGroup>() else {
            return Ok(());
        };
        //println!("3");

        let Some(pipeline) = pipeline_cache.get_render_pipeline(my_pipeline.pipeline_id) else {
            return Ok(());
        };
        //println!("4");

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("my_custom_pass".into()),
            color_attachments: &[Some(view_target.get_color_attachment())],
            ..default()
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group.bind_group, &[]);
        render_pass.set_bind_group(1, &my_pipeline.static_bind_group, &[]);
        render_pass.draw(0..3, 0..1); //fullscreen triangle

        Ok(())
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct MyPassLabel;

// This is the component that will get passed to the shader
#[derive(Resource, Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
struct Uniform {
    world_from_clip: Mat4,
    resolution: Vec2,
    buffer_mask: u32,
    buffer_shift: u32,
    render_distance: f32,
    voxel_size: f32,
    inv_voxel_size: f32,
    buffer_size: u32,
    max_height: f32,
}

#[derive(Resource)]
struct PendingTerrainChanges {
    changes: Vec<(usize, GpuQuadInfo)>,
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]

struct GpuMaterial {
    base_color: Vec3,
    reflectance: f32,
    emissive: Vec3,
    roughness: f32,
    specular_tint: Vec3,
    metallic: f32,
    attenuation_color: Vec3,
    diffuse_transmission: f32,
    specular_transmission: f32,
    ior: f32,
    thickness: f32,
    attenuation_distance: f32,
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    clearcoat: f32,
    clearcoat_roughness: f32,
    emissive_strength: f32,
    _pad: i32,    // 4 bytes
    _pad2: IVec2, // 8 bytes — reaches 112
}

impl GpuMaterial {
    fn from_standardmaterial(material: StandardMaterial, emissive_strength: f32) -> Self {
        GpuMaterial {
            base_color: material.base_color.to_linear().to_vec3(),
            reflectance: material.reflectance,
            emissive: material.emissive.to_vec3(),
            roughness: material.perceptual_roughness,
            specular_tint: material.specular_tint.to_linear().to_vec3(),
            metallic: material.metallic,
            attenuation_color: material.attenuation_color.to_linear().to_vec3(),
            diffuse_transmission: material.diffuse_transmission,
            specular_transmission: material.specular_transmission,
            ior: material.ior,
            thickness: material.thickness,
            attenuation_distance: material.attenuation_distance,
            anisotropy_strength: material.anisotropy_strength,
            anisotropy_rotation: material.anisotropy_rotation,
            clearcoat: material.clearcoat,
            clearcoat_roughness: material.clearcoat_perceptual_roughness,
            emissive_strength: emissive_strength,
            _pad: 0,
            _pad2: IVec2::default(),
        }
    }
}
#[repr(C, align(16))]
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, NoUninit)]
///Information about a single quad
struct GpuQuadInfo {
    ///coordinates of the vertex with the lowest (closest to negative infinity) x and z values in world space
    world_coords: Vec2,
    voxel_coords: IVec2,
    upper: GpuSimplePlane,
    lower: GpuSimplePlane,
    y_max: f32,
    y_min: f32,
    _pad: IVec2,
    ///vertex normal of the vertex at world_coords (x_min, z_min). 4th value is unused
    n1: [f32; 4],
    ///The 3d position of the vertex at world_coords (x_min, z_min). 4th value is unused
    pos_1: [f32; 4],
    ///vertex normal of the vertex at world_coords (x_max, z_min). 4th value is unused
    n2: [f32; 4],
    ///The 3d position of the vertex at world_coords (x_max, z_min). 4th value is unused
    pos_2: [f32; 4],
}
#[repr(C, align(16))]
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, NoUninit)]
///information about a single plane
struct GpuSimplePlane {
    ///first 3 is normal, last entry is plane constant d
    n_and_d: [f32; 4],
    ///only first entry is valid
    material_id: [i32; 4],
}

impl GpuQuadInfo {
    ///Creates a new quad at the gixen voxel coordinates
    fn new_simple(coords: IVec2, noise: &NoiseStore) -> Self {
        let coords = VOXEL_SIZE as i32 * coords;
        let v0 = coords.to_array();
        let v1 = (coords + IVec2::new(VOXEL_SIZE as i32, 0)).to_array();
        let v2 = (coords + IVec2::new(0, VOXEL_SIZE as i32)).to_array();
        let v3 = (coords + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32)).to_array();

        return GpuQuadInfo::new([v0, v1, v2, v3], noise);
    }

    ///Creates a new quad from 4 vertexes (given as [[x,z]; 4])
    fn new(vertexes: [[i32; 2]; 4], noise: &NoiseStore) -> Self {
        // Find min and max X/Z
        let x_min = vertexes.iter().map(|v| v[0]).min().unwrap();
        let z_min = vertexes.iter().map(|v| v[1]).min().unwrap();
        let x_max = vertexes.iter().map(|v| v[0]).max().unwrap();
        let z_max = vertexes.iter().map(|v| v[1]).max().unwrap();

        let world_coords = Vec2::new(x_min as f32, z_min as f32);
        let voxel_coords = coord(Vec3::new(world_coords.x, 0.0, world_coords.y));

        // Heights at the 4 corners
        let y0 = get_height(x_min, z_min, noise);
        let y1 = get_height(x_max, z_min, noise);
        let y2 = get_height(x_min, z_max, noise);
        let y3 = get_height(x_max, z_max, noise);

        //Extra heights for normal calculations
        let y4 = get_height(x_min - VOXEL_SIZE as i32, z_max, noise);
        let y5 = get_height(x_min - VOXEL_SIZE as i32, z_min, noise);
        //let y6 = get_height(x_min - VOXEL_SIZE as i32, z_min - VOXEL_SIZE as i32, noise);
        let y7 = get_height(x_min, z_min - VOXEL_SIZE as i32, noise);
        let y8 = get_height(x_max, z_min - VOXEL_SIZE as i32, noise);
        let y9 = get_height(x_max + VOXEL_SIZE as i32, z_min - VOXEL_SIZE as i32, noise);
        let y10 = get_height(x_max + VOXEL_SIZE as i32, z_min, noise);
        //let y11 = get_height(x_max + VOXEL_SIZE as i32, z_max, noise);

        // 3D positions
        let p0 = Vec3::new(x_min as f32, y0, z_min as f32);
        let p1 = Vec3::new(x_max as f32, y1, z_min as f32);
        let p2 = Vec3::new(x_min as f32, y2, z_max as f32);
        let p3 = Vec3::new(x_max as f32, y3, z_max as f32);
        let p4 = Vec3::new(x_min as f32 - VOXEL_SIZE, y4, z_max as f32);
        let p5 = Vec3::new(x_min as f32 - VOXEL_SIZE, y5, z_min as f32);
        let p7 = Vec3::new(x_min as f32, y7, z_min as f32 - VOXEL_SIZE);
        let p8 = Vec3::new(x_max as f32, y8, z_min as f32 - VOXEL_SIZE);
        let p9 = Vec3::new(x_max as f32 + VOXEL_SIZE, y9, z_min as f32 - VOXEL_SIZE);
        let p10 = Vec3::new(x_max as f32 + VOXEL_SIZE, y10, z_min as f32);
        //let p11 = Vec3::new(x_max as f32 + VOXEL_SIZE,   y11, z_max as f32);

        //current voxel normals
        let n_lower = Vec3::new(
            -(y1 - y0) * INV_VOXEL_SIZE,
            1.0,
            -(y2 - y0) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_upper = Vec3::new(
            -(y3 - y2) * INV_VOXEL_SIZE,
            1.0,
            -(y3 - y1) * INV_VOXEL_SIZE,
        )
        .normalize();

        //normals of all surrouding triangles
        let n_1 = Vec3::new(
            -(y2 - y4) * INV_VOXEL_SIZE,
            1.0,
            -(y2 - y0) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_2 = Vec3::new(
            -(y0 - y5) * INV_VOXEL_SIZE,
            1.0,
            -(y4 - y5) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_3 = Vec3::new(
            -(y0 - y5) * INV_VOXEL_SIZE,
            1.0,
            -(y0 - y7) * INV_VOXEL_SIZE,
        )
        .normalize();
        //let n_4 = Vec3::new(-(y7 - y6) * INV_VOXEL_SIZE, 1.0, -(y5 - y6) * INV_VOXEL_SIZE).normalize();
        let n_5 = Vec3::new(
            -(y1 - y0) * INV_VOXEL_SIZE,
            1.0,
            -(y1 - y8) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_6 = Vec3::new(
            -(y8 - y7) * INV_VOXEL_SIZE,
            1.0,
            -(y0 - y7) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_7 = Vec3::new(
            -(y10 - y1) * INV_VOXEL_SIZE,
            1.0,
            -(y10 - y9) * INV_VOXEL_SIZE,
        )
        .normalize();
        let n_8 = Vec3::new(
            -(y9 - y8) * INV_VOXEL_SIZE,
            1.0,
            -(y1 - y8) * INV_VOXEL_SIZE,
        )
        .normalize();
        //let n_9 = Vec3::new(-(y11 - y3) * INV_VOXEL_SIZE, 1.0, -(y11 - y10) * INV_VOXEL_SIZE).normalize();
        let n_10 = Vec3::new(
            -(y10 - y1) * INV_VOXEL_SIZE,
            1.0,
            -(y3 - y1) * INV_VOXEL_SIZE,
        )
        .normalize();

        let n1 = (n_lower * angle_at(p0, p1, p2)
            + n_1 * angle_at(p0, p2, p4)
            + n_2 * angle_at(p0, p4, p5)
            + n_3 * angle_at(p0, p5, p7)
            + n_6 * angle_at(p0, p7, p8)
            + n_5 * angle_at(p0, p8, p1))
        .normalize();

        // --- Vertex normal at p1 ---
        // 6 triangles: n_lower, n_upper, n_5, n_8, n_7, n_10
        let n2 = (n_lower * angle_at(p1, p2, p0)
            + n_upper * angle_at(p1, p3, p2)
            + n_5 * angle_at(p1, p0, p8)
            + n_8 * angle_at(p1, p8, p9)
            + n_7 * angle_at(p1, p9, p10)
            + n_10 * angle_at(p1, p10, p3))
        .normalize();

        // Compute min/max Y
        let y_max = y0.max(y1).max(y2).max(y3);
        let y_min = y0.min(y1).min(y2).min(y3);

        // Construct lower plane (right angle at min_x, min_z)
        let lower = {
            let world_point = Vec3::new(x_min as f32, y0, z_min as f32);
            let d = -n_lower.dot(world_point);
            GpuSimplePlane {
                n_and_d: [n_lower.x, n_lower.y, n_lower.z, d],
                material_id: [0, 0, 0, 0],
            }
        };

        // Construct upper plane (right angle at max_x, max_z)
        let upper = {
            let world_point = Vec3::new(x_max as f32, y3, z_max as f32);
            let d = -n_upper.dot(world_point);
            GpuSimplePlane {
                n_and_d: [n_upper.x, n_upper.y, n_upper.z, d],
                material_id: [0, 0, 0, 0],
            }
        };

        GpuQuadInfo {
            world_coords,
            voxel_coords,
            upper,
            lower,
            y_max,
            y_min,
            _pad: IVec2::default(),
            //placeholders
            n1: [n1.x, n1.y, n1.z, 0.],
            n2: [n2.x, n2.y, n2.z, 0.],
            pos_1: [x_min as f32, y0, z_min as f32, 0.],
            pos_2: [x_max as f32, y1, z_min as f32, 0.]
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    /*
    // lock mouse into window by default
    if let Ok((mut primary_window, mut cursor_options)) = q_window.single_mut() {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
        let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
        primary_window.set_cursor_position(Some(center));

        // camera

    }
    */

    println!("Actual voxel size: {}", VOXEL_SIZE);
    println!("Buffer size: {}", BUFFER_SIZE);
    println!("Actual render distance: {}", RENDER_DIST_VOXELS * VOXEL_SIZE as i32); 

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Fxaa {
            enabled: true,
            edge_threshold: Sensitivity::Low,
            edge_threshold_min: Sensitivity::Low,
        },
    ));

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
            moving: true,
        },
    ));

    //fps text
    commands
        .spawn((Text::new("Fps:"), TextColor(Color::BLACK)))
        .with_child((TextSpan::default(), TextColor(Color::BLACK), FpsText));

    //noise for terrain generation

    let seed = 9226;
    commands.insert_resource(NoiseStore {
        basic_perlin: Perlin::new(seed),
    });

    //placeholder
    commands.insert_resource(MaxHeight(8.));
}


///Takes the x and z voxel coordinates of a quad and returns the index within the terrain buffer
fn get_index(x: i32, z: i32) -> usize {
    let xi = x as usize & BUFFER_MASK;
    let zi = z as usize & BUFFER_MASK;
    (zi << BUFFER_SHIFT) | xi
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
    if let Ok((mut player, mut player_transform)) = player_q.single_mut() {
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
            player.moving = true;
        } else {
            player.moving = false;
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
            let _ = if let Ok(player_transform) = player_q.single() {
                player_transform.translation
            } else {
                Vec3::ZERO
            };
            //let coords = coord(pos);
            **span = format!("{value:.2}");
        }
    }
}

fn positive_mod(a: i32, b: i32) -> usize {
    (((a % b) + b) % b) as usize
}

fn get_height(x: i32, z: i32, noise: &NoiseStore) -> f32 {
    let o1 = noise
        .basic_perlin
        .get([x as f64 + FRAC_PI_4 as f64, z as f64 + FRAC_PI_4 as f64]) as f32;

    let o2 = 5.
        * noise.basic_perlin.get([
            (x as f64 + FRAC_PI_4 as f64) / 70.,
            (z as f64 + FRAC_PI_4 as f64) / 70.,
        ]) as f32;

    let o3 = 15.
        * noise.basic_perlin.get([
            (x as f64 + FRAC_PI_4 as f64) / 200.,
            (z as f64 + FRAC_PI_4 as f64) / 200.,
        ]) as f32;

    let o4 = 100.
        * noise.basic_perlin.get([
            (x as f64 + FRAC_PI_4 as f64) / 1000.,
            (z as f64 + FRAC_PI_4 as f64) / 1000.,
        ]) as f32;
    /* 
    let o_a = match (x+5) % 7 {
        0 => 15,
        _ => 0
    } as f32;

    let o_b = match (z+5) % 7 {
        0 => 15,
        _ => 0
    } as f32;
    */
    o1 + o2 + o3 +o4 //+ o_a + o_b
}

///marker component for fps text
#[derive(Component)]
struct FpsText;

///used for identifying the player entity
#[derive(Component)]
pub struct Player {
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub moving: bool,
}

#[derive(Resource)]
///Stores the max height of current rendered terrain
struct MaxHeight(f32);

///Struct for holding all currently relevant terrain data
#[derive(Resource)]
struct TerrainStore {
    quad_buffer: Box<[GpuQuadInfo]>,
    initialized: bool,
}

impl Default for TerrainStore {
    fn default() -> Self {
        TerrainStore {
            quad_buffer: (0..(BUFFER_SIZE * BUFFER_SIZE))
                .map(|_| GpuQuadInfo::default())
                .collect(),
            initialized: false,
        }
    }
}

///converts world position to horizontal voxel coordinates
fn coord(p: Vec3) -> IVec2 {
    (p.xz() * INV_VOXEL_SIZE).floor().as_ivec2()
}

///Changes the terrainstore and sends the changes to the gpu as well
fn stage_terrain_updates(
    mut commands: Commands,
    player_q: Single<(&Transform, &Player)>,
    terrain_store: Option<ResMut<TerrainStore>>,
    noise: Res<NoiseStore>,
    mut max_height: ResMut<MaxHeight>,
    time: Res<Time>,
    //mut gizmos: Gizmos
) {
    // Clear last frame's changes first
    commands.remove_resource::<PendingTerrainChanges>();

    //println!("Are we blind");
    let Some(mut terrain_store) = terrain_store else {
        return;
    };
    /*
    //temporary "rendering"
    for quad in &terrain_store.quad_buffer {
        let coords = IVec2::new(quad.world_coords.x as i32, quad.world_coords.y as i32);
        let v0 = coords;
        let v1 = coords + IVec2::new(VOXEL_SIZE as i32, 0);
        let v2 = coords + IVec2::new(0, VOXEL_SIZE as i32);
        let v3 = coords + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32);

        for edge in [(v0, v1), (v1, v2), (v2, v0), (v2, v3), (v3, v1)] {
            let start = Vec3::new(
                edge.0.x as f32,
                get_height(edge.0.x, edge.0.y, &noise),
                edge.0.y as f32,
            );
            let end = Vec3::new(
                edge.1.x as f32,
                get_height(edge.1.x, edge.1.y, &noise),
                edge.1.y as f32,
            );
            gizmos.line(start, end, Color::WHITE);
        }
    }
    */

    //println!("Hello?");

    let need_init = !terrain_store.initialized;

    let (player_transform, player) = player_q.into_inner();
    let player_voxel = coord(player_transform.translation);
    let mut max_added = 0.;

    let mut changes = Vec::new();

    let mut update_voxel = |x: i32, z: i32| {
        let idx = get_index(x, z);
        let voxel_coords = IVec2::new(x, z);

        // Only generate if stale
        if need_init || terrain_store.quad_buffer[idx].voxel_coords != voxel_coords {
            let new_quad = GpuQuadInfo::new_simple(voxel_coords, &noise);

            if new_quad.y_max > max_added {
                max_added = new_quad.y_max;
            }

            changes.push((idx, new_quad.clone()));

            terrain_store.quad_buffer[idx] = new_quad;
        }
    };

    //if just started, generate all voxels in parallel
    if need_init {
        let noise = noise.into_inner().to_owned();
        let new_quads: Vec<(usize, GpuQuadInfo)> = ((player_voxel.y - RENDER_DIST_VOXELS)
            ..(player_voxel.y + RENDER_DIST_VOXELS))
            .into_par_iter()
            .flat_map(|z| {
                ((player_voxel.x - RENDER_DIST_VOXELS)..(player_voxel.x + RENDER_DIST_VOXELS))
                    .map(move |x| {
                        (
                            get_index(x, z),
                            GpuQuadInfo::new_simple(IVec2::new(x, z), &noise),
                        )
                    })
                    .collect::<Vec<(usize, GpuQuadInfo)>>()
            })
            .collect();

        for (idx, quad) in new_quads {
            if quad.y_max > max_added {
                max_added = quad.y_max;
            }
            changes.push((idx, quad));
            terrain_store.quad_buffer[idx] = quad;
        }

        terrain_store.initialized = true;
        max_height.0 = max_added;
        //println!("Set max height to {}", max_height.0);

        //send changes to gpu
        //println!("Sorting started");
        changes.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for (i, quad) in changes.clone() {
            terrain_store.quad_buffer[i] = quad;
            //println!("A change was made");
        }
        //println!("Sorting finished");
        commands.insert_resource(PendingTerrainChanges { changes });

        return;
    }

    //only do this if player is moving
    if !player.moving {
        return;
    }

    //Only check the edges for updating
    let voxels_per_frame = ((MOVE_SPEED * time.delta_secs()) * INV_VOXEL_SIZE).ceil() as i32;

    // Top strip (outer edge)
    for z in (player_voxel.y + RENDER_DIST_VOXELS - voxels_per_frame)
        ..(player_voxel.y + RENDER_DIST_VOXELS)
    {
        for x in (player_voxel.x - RENDER_DIST_VOXELS)..(player_voxel.x + RENDER_DIST_VOXELS) {
            update_voxel(x, z);
        }
    }

    // Bottom strip (outer edge)
    for z in (player_voxel.y - RENDER_DIST_VOXELS)
        ..(player_voxel.y - RENDER_DIST_VOXELS + voxels_per_frame)
    {
        for x in (player_voxel.x - RENDER_DIST_VOXELS)..(player_voxel.x + RENDER_DIST_VOXELS) {
            update_voxel(x, z);
        }
    }

    // Left strip (exclude corners already covered by top/bottom)
    for z in (player_voxel.y - RENDER_DIST_VOXELS + voxels_per_frame)
        ..(player_voxel.y + RENDER_DIST_VOXELS - voxels_per_frame)
    {
        for x in (player_voxel.x - RENDER_DIST_VOXELS)
            ..(player_voxel.x - RENDER_DIST_VOXELS + voxels_per_frame)
        {
            update_voxel(x, z);
        }
    }

    // Right strip (exclude corners already covered by top/bottom)
    for z in (player_voxel.y - RENDER_DIST_VOXELS + voxels_per_frame)
        ..(player_voxel.y + RENDER_DIST_VOXELS - voxels_per_frame)
    {
        for x in (player_voxel.x + RENDER_DIST_VOXELS - voxels_per_frame)
            ..(player_voxel.x + RENDER_DIST_VOXELS)
        {
            update_voxel(x, z);
        }
    }

    if max_added > max_height.0 {
        max_height.0 = max_added;
    }

    //recompute (roughly) every 5(ish) seconds incase max was removed
    //due to floating point drift this is pretty inaccurate and will need to be replaced later
    if (time.elapsed_secs() * 1000.0) as u64 % 5000 == 0 {
        max_height.0 = terrain_store
            .quad_buffer
            .iter()
            .map(|q| q.y_max)
            .reduce(f32::max)
            .unwrap_or(0.0);
        //println!("Recomputed max height to {}", max_height.0);
    }

    changes.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (i, quad) in changes.clone() {
        terrain_store.quad_buffer[i] = quad;
    }

    commands.insert_resource(PendingTerrainChanges { changes });
}

#[derive(Resource)]
///Noise Resources for terrain generation
struct NoiseStore {
    basic_perlin: Perlin,
}

fn angle_at(center: Vec3, a: Vec3, b: Vec3) -> f32 {
    let e1 = (a - center).normalize();
    let e2 = (b - center).normalize();
    let dot = e1.dot(e2);
    let cross = e1.cross(e2).length();
    cross.atan2(dot)
}
