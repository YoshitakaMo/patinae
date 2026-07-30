// {{INCLUDE_FRAME}}

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
@group(2) @binding(1) var font_atlas: texture_2d<f32>;
@group(2) @binding(2) var font_sampler: sampler;

struct VertexInput {
    @location(0) anchor: vec3<f32>,
    @location(1) offset_px: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) kind: u32,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) kind: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var clip = frame.view_proj * vec4<f32>(input.anchor, 1.0);
    let pixel_to_clip = vec2<f32>(2.0 / frame.viewport.x, -2.0 / frame.viewport.y);
    let shifted = clip.xy + input.offset_px * pixel_to_clip * clip.w;
    clip = vec4<f32>(shifted, clip.z, clip.w);
    out.clip_position = clip;
    out.uv = input.uv;
    out.kind = input.kind;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.kind == 1u {
        if measurement.label_bg_outline != 0u {
            let edge = min(min(input.uv.x, 1.0 - input.uv.x), min(input.uv.y, 1.0 - input.uv.y));
            if edge < 0.06 {
                let outline = select(measurement.label_color, measurement.label_outline_color,
                    measurement.label_outline_color.a > 0.0);
                return outline;
            }
        }
        if measurement.label_bg_color.a <= 0.0 { discard; }
        return measurement.label_bg_color;
    }
    if input.kind == 3u {
        if measurement.label_connector_color.a <= 0.0 { discard; }
        return measurement.label_connector_color;
    }
    let alpha = textureSample(font_atlas, font_sampler, input.uv).r;
    if alpha <= 0.01 { discard; }
    if input.kind == 2u {
        if measurement.label_shadow_mode == 0u { discard; }
        return vec4<f32>(0.0, 0.0, 0.0, alpha * 0.55);
    }
    if measurement.label_outline_color.a > 0.0 && alpha < 0.65 {
        return vec4<f32>(
            measurement.label_outline_color.rgb,
            alpha * measurement.label_outline_color.a
        );
    }
    return vec4<f32>(measurement.label_color.rgb, alpha * measurement.label_color.a);
}
