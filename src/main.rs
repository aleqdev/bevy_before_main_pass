use bevy::{
    core_pipeline::{
        clear_color::ClearColorConfig, core_3d,
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::{
            ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingResource, BindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, Operations,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget},
        RenderApp,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MyPlugin)
        .add_systems(Startup, init)
        .run();
}

fn init(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera with gradient
    commands.spawn((
        Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            camera: Camera {
                order: -1,
                ..default()
            },
            ..default()
        },
        RenderBeforeMainPassSettings { color: Color::RED },
    ));

    // Camera with cube
    commands.spawn((Camera3dBundle {
        camera_3d: Camera3d {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(10.0, 10.0, 10.))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    },));

    // Cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Cube::new(3.0).into()),
        material: materials.add(Color::GREEN.into()),
        ..default()
    });
}

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<RenderBeforeMainPassSettings>::default(),
            UniformComponentPlugin::<RenderBeforeMainPassSettings>::default(),
        ));

        bevy::asset::load_internal_asset!(
            app,
            MY_SHADER_HANDLE,
            "./my_shader.wgsl",
            Shader::from_wgsl
        );

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app.add_render_graph_node::<MyNode>(core_3d::graph::NAME, MyNode::NAME);

        // Change this to switch behaviour
        if true {
            render_app.add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::PREPASS,
                    MyNode::NAME,
                    core_3d::graph::node::START_MAIN_PASS,
                ],
            );
        } else {
            render_app.add_render_graph_edges(
                // this shows gradient on top of cube
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::END_MAIN_PASS,
                    MyNode::NAME,
                    core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                ],
            );
        }
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app.init_resource::<MyPipeline>();
    }
}

const MY_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17571237424902107003);

#[derive(Clone, Component, ExtractComponent, ShaderType)]
struct RenderBeforeMainPassSettings {
    color: Color,
}

#[derive(Resource)]
struct MyPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for MyPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(RenderBeforeMainPassSettings::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });

        let shader = MY_SHADER_HANDLE.typed();

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: None,
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: shader.clone(),
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

struct MyNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
}

impl MyNode {
    const NAME: &str = "my_node";
}

impl FromWorld for MyNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MyNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph_context: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph_context.view_entity();

        let Ok(view_target) = self.query.get_manual(world, view_entity) else {
      return Ok(());
  };

        let post_process_pipeline = world.resource::<MyPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id) else {
        return Ok(());
    };

        let settings_uniforms = world.resource::<ComponentUniforms<RenderBeforeMainPassSettings>>();

        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
      return Ok(());
    };

        let post_process = view_target.post_process_write();

        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &post_process_pipeline.layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(post_process.source),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&post_process_pipeline.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: settings_binding.clone(),
                    },
                ],
            });

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
