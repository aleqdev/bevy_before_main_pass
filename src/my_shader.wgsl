#import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;

@group(0) @binding(1)
var screen_sampler: sampler;

@group(0) @binding(2)
var<uniform> settings: MySettings;

struct MySettings {
    color: vec4<f32>,
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
  return mix(vec4(0.0), settings.color, in.uv.y);
}
