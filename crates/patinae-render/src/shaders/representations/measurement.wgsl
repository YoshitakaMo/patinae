// {{INCLUDE_FRAME}}
// {{INCLUDE_WBOIT}}

struct MeasurementParams {
    color: vec4<f32>,
    label_color: vec4<f32>,
    label_outline_color: vec4<f32>,
    label_bg_color: vec4<f32>,
    label_connector_color: vec4<f32>,
    dash_width: f32,
    label_bg_outline: u32,
    label_shadow_mode: u32,
    label_z_target: u32,
};
@group(2) @binding(0) var<uniform> measurement: MeasurementParams;

struct VertexInput {
    @location(0) p0: vec3<f32>,
    @location(1) p1: vec3<f32>,
    @location(2) corner: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) view_depth: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let view0 = frame.view * vec4<f32>(input.p0, 1.0);
    let view1 = frame.view * vec4<f32>(input.p1, 1.0);
    let clip0 = frame.proj * view0;
    let clip1 = frame.proj * view1;
    let ndc0 = clip0.xy / clip0.w;
    let ndc1 = clip1.xy / clip1.w;
    let delta = ndc1 - ndc0;
    let delta_len = max(length(delta), 1e-6);
    let normal = vec2<f32>(-delta.y, delta.x) / delta_len;
    var clip = mix(clip0, clip1, input.corner.x);
    let half_width_ndc = measurement.dash_width / frame.viewport.xy;
    let shifted = clip.xy + normal * half_width_ndc * input.corner.y * clip.w;
    clip = vec4<f32>(shifted, clip.z, clip.w);
    let view = mix(view0, view1, input.corner.x);
    out.clip_position = clip;
    out.view_depth = -view.z;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> TranslucentOut {
    return write_translucent(measurement.color.rgb, measurement.color.a, input.view_depth, frame.clip.w);
}

@fragment
fn fs_opaque() -> @location(0) vec4<f32> {
    return vec4<f32>(measurement.color.rgb, 1.0);
}
