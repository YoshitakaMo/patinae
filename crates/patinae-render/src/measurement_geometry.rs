use std::collections::HashMap;
use std::sync::LazyLock;

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::pipelines::measurement::{MeasurementParams, MeasurementParamsLayout};
use crate::render_input::RenderMeasurementInput;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeasurementVertex {
    pub p0: [f32; 3],
    pub p1: [f32; 3],
    /// x selects p0/p1; y selects the negative/positive screen normal.
    pub corner: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeasurementLabelVertex {
    pub anchor: [f32; 3],
    pub offset_px: [f32; 2],
    pub uv: [f32; 2],
    /// 0 glyph, 1 background, 2 shadow glyph, 3 connector.
    pub kind: u32,
}

impl MeasurementLabelVertex {
    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Uint32];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

/// Raster size of the shared atlas. Label quads scale this to `label_size`.
pub(crate) const MEASUREMENT_ATLAS_FONT_SIZE_PX: f32 = 64.0;
const FONT_BYTES: &[u8] = include_bytes!("../../../patinae/ui/fonts/Inter-Medium.ttf");
pub(crate) const MEASUREMENT_ATLAS_WIDTH: u32 = 1024;
pub(crate) const MEASUREMENT_ATLAS_HEIGHT: u32 = 96;
const ATLAS_PADDING: u32 = 1;
const SUPPORTED_GLYPHS: &str = "0123456789.-";

#[derive(Debug, Clone, Copy)]
struct AtlasGlyph {
    advance: f32,
    offset_min: [f32; 2],
    offset_max: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

pub(crate) struct MeasurementFontAtlas {
    pub pixels: Vec<u8>,
    glyphs: HashMap<char, AtlasGlyph>,
    baseline_offset: f32,
}

pub(crate) static MEASUREMENT_FONT_ATLAS: LazyLock<MeasurementFontAtlas> =
    LazyLock::new(MeasurementFontAtlas::build);

impl MeasurementFontAtlas {
    fn build() -> Self {
        let font = FontRef::try_from_slice(FONT_BYTES)
            .expect("bundled Inter Medium font must remain valid");
        let scale = PxScale::from(MEASUREMENT_ATLAS_FONT_SIZE_PX);
        let scaled = font.as_scaled(scale);
        let baseline_offset = (scaled.ascent() + scaled.descent()) * 0.5;
        let mut pixels = vec![0; (MEASUREMENT_ATLAS_WIDTH * MEASUREMENT_ATLAS_HEIGHT) as usize];
        let mut glyphs = HashMap::new();
        let mut cursor_x = ATLAS_PADDING;

        for character in SUPPORTED_GLYPHS.chars() {
            let glyph_id = font.glyph_id(character);
            let advance = scaled.h_advance(glyph_id);
            let glyph = glyph_id.with_scale_and_position(scale, point(0.0, 0.0));
            let Some(outlined) = font.outline_glyph(glyph) else {
                glyphs.insert(
                    character,
                    AtlasGlyph {
                        advance,
                        offset_min: [0.0; 2],
                        offset_max: [0.0; 2],
                        uv_min: [0.0; 2],
                        uv_max: [0.0; 2],
                    },
                );
                continue;
            };
            let bounds = outlined.px_bounds();
            let width = bounds.width().ceil().max(0.0) as u32;
            let height = bounds.height().ceil().max(0.0) as u32;
            assert!(
                cursor_x + width + ATLAS_PADDING <= MEASUREMENT_ATLAS_WIDTH
                    && height + 2 * ATLAS_PADDING <= MEASUREMENT_ATLAS_HEIGHT,
                "measurement font atlas is too small"
            );
            let atlas_x = cursor_x;
            let atlas_y = ATLAS_PADDING;
            outlined.draw(|x, y, coverage| {
                let index = ((atlas_y + y) * MEASUREMENT_ATLAS_WIDTH + atlas_x + x) as usize;
                pixels[index] = (coverage * 255.0).round() as u8;
            });
            glyphs.insert(
                character,
                AtlasGlyph {
                    advance,
                    offset_min: [bounds.min.x, bounds.min.y],
                    offset_max: [bounds.max.x, bounds.max.y],
                    uv_min: [
                        atlas_x as f32 / MEASUREMENT_ATLAS_WIDTH as f32,
                        atlas_y as f32 / MEASUREMENT_ATLAS_HEIGHT as f32,
                    ],
                    uv_max: [
                        (atlas_x + width) as f32 / MEASUREMENT_ATLAS_WIDTH as f32,
                        (atlas_y + height) as f32 / MEASUREMENT_ATLAS_HEIGHT as f32,
                    ],
                },
            );
            cursor_x += width + 2 * ATLAS_PADDING;
        }

        Self {
            pixels,
            glyphs,
            baseline_offset,
        }
    }
}

impl MeasurementVertex {
    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

pub struct MeasurementEntry {
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub vertex_count: u32,
    pub label_buffer: Option<wgpu::Buffer>,
    pub label_vertex_count: u32,
    pub params_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub params: MeasurementParams,
    pub segments: Vec<crate::render_input::MeasurementSegment>,
    pub labels: Vec<crate::render_input::RenderMeasurementLabel>,
    pub color: [f32; 4],
    pub label_color: [f32; 4],
    pub label_outline_color: [f32; 4],
    pub label_bg_color: [f32; 4],
    pub label_connector_color: [f32; 4],
    pub label_size: f32,
    pub label_padding: [f32; 3],
    pub label_connector_width: f32,
    pub label_connector_ext_length: f32,
    pub dash_width: f32,
    pub is_opaque: bool,
}

impl MeasurementEntry {
    pub fn new(
        input: &RenderMeasurementInput,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &MeasurementParamsLayout,
    ) -> Self {
        let params = measurement_params(input);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("patinae.measurement.params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("patinae.measurement.params_bg"),
            layout: &layout.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&layout.font_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&layout.font_sampler),
                },
            ],
        });
        let mut entry = Self {
            vertex_buffer: None,
            vertex_count: 0,
            label_buffer: None,
            label_vertex_count: 0,
            params_buffer,
            bind_group,
            params,
            segments: Vec::new(),
            labels: Vec::new(),
            color: input.color,
            label_color: input.label_color,
            label_outline_color: input.label_outline_color,
            label_bg_color: input.label_bg_color,
            label_connector_color: input.label_connector_color,
            label_size: input.label_size,
            label_padding: input.label_padding,
            label_connector_width: input.label_connector_width,
            label_connector_ext_length: input.label_connector_ext_length,
            dash_width: input.dash_width,
            is_opaque: input.color[3] >= 0.999,
        };
        entry.sync(input, device, queue);
        entry
    }

    pub fn sync(
        &mut self,
        input: &RenderMeasurementInput,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> bool {
        let geometry_changed = self.segments != input.segments;
        if geometry_changed {
            self.segments.clone_from(&input.segments);
            let vertices: Vec<MeasurementVertex> = input
                .segments
                .iter()
                .flat_map(|segment| {
                    let vertex = |along, side| MeasurementVertex {
                        p0: segment.p0,
                        p1: segment.p1,
                        corner: [along, side],
                    };
                    [
                        vertex(0.0, -1.0),
                        vertex(1.0, -1.0),
                        vertex(1.0, 1.0),
                        vertex(0.0, -1.0),
                        vertex(1.0, 1.0),
                        vertex(0.0, 1.0),
                    ]
                })
                .collect();
            self.vertex_count = vertices.len() as u32;
            self.vertex_buffer = (!vertices.is_empty()).then(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("patinae.measurement.vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
        }
        let labels_changed = self.labels != input.labels;
        if labels_changed {
            self.labels.clone_from(&input.labels);
            let vertices = label_vertices(input);
            self.label_vertex_count = vertices.len() as u32;
            self.label_buffer = (!vertices.is_empty()).then(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("patinae.measurement.labels"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
        }
        let label_layout_changed = self.label_size != input.label_size
            || self.label_padding != input.label_padding
            || self.label_connector_width != input.label_connector_width
            || self.label_connector_ext_length != input.label_connector_ext_length;
        if label_layout_changed && !labels_changed {
            let vertices = label_vertices(input);
            self.label_vertex_count = vertices.len() as u32;
            self.label_buffer = (!vertices.is_empty()).then(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("patinae.measurement.labels"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
        }
        let params = measurement_params(input);
        let material_changed = self.params != params;
        if material_changed {
            self.color = input.color;
            self.label_color = input.label_color;
            self.label_outline_color = input.label_outline_color;
            self.label_bg_color = input.label_bg_color;
            self.label_connector_color = input.label_connector_color;
            self.dash_width = input.dash_width;
            self.params = params;
            self.is_opaque = input.color[3] >= 0.999;
            queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
        }
        self.label_size = input.label_size;
        self.label_padding = input.label_padding;
        self.label_connector_width = input.label_connector_width;
        self.label_connector_ext_length = input.label_connector_ext_length;
        geometry_changed || labels_changed || label_layout_changed || material_changed
    }

    pub fn record<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(vertices) = self.vertex_buffer.as_ref() else {
            return;
        };
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    pub fn record_labels<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(vertices) = self.label_buffer.as_ref() else {
            return;
        };
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.draw(0..self.label_vertex_count, 0..1);
    }
}

fn connector_color(input: &RenderMeasurementInput) -> [f32; 4] {
    let mut color = input.label_connector_color;
    if !input.label_connector {
        color[3] = 0.0;
    }
    color
}

fn measurement_params(input: &RenderMeasurementInput) -> MeasurementParams {
    MeasurementParams {
        color: input.color,
        label_color: input.label_color,
        label_outline_color: input.label_outline_color,
        label_bg_color: input.label_bg_color,
        label_connector_color: connector_color(input),
        dash_width: input.dash_width,
        label_bg_outline: u32::from(input.label_bg_outline),
        label_shadow_mode: input.label_shadow_mode.max(0) as u32,
        label_z_target: input.label_z_target.max(0) as u32,
    }
}

fn label_vertices(input: &RenderMeasurementInput) -> Vec<MeasurementLabelVertex> {
    let atlas = &*MEASUREMENT_FONT_ATLAS;
    let font_scale = input.label_size.max(1.0) / MEASUREMENT_ATLAS_FONT_SIZE_PX;
    let mut vertices = Vec::new();
    for label in &input.labels {
        let width: f32 = label
            .text
            .chars()
            .filter_map(|character| atlas.glyphs.get(&character))
            .map(|glyph| glyph.advance * font_scale)
            .sum();
        let mut pen_x = -width * 0.5;
        let pad_x = input.label_padding[0].max(0.0) * input.label_size;
        let pad_y = input.label_padding[1].max(0.0) * input.label_size;
        let left = label.offset_px[0] - width * 0.5 - pad_x;
        let right = label.offset_px[0] + width * 0.5 + pad_x;
        let top = label.offset_px[1] - input.label_size * 0.5 - pad_y;
        let bottom = label.offset_px[1] + input.label_size * 0.5 + pad_y;
        append_label_quad(
            &mut vertices,
            label.anchor,
            [left, top],
            [right, bottom],
            [0.0, 0.0],
            [1.0, 1.0],
            1,
        );
        append_connector_quad(
            &mut vertices,
            label.anchor,
            label.offset_px,
            input.label_connector_width,
            input.label_connector_ext_length,
        );
        for character in label.text.chars() {
            let Some(glyph) = atlas.glyphs.get(&character) else {
                continue;
            };
            let x0 = label.offset_px[0] + pen_x + glyph.offset_min[0] * font_scale;
            let x1 = label.offset_px[0] + pen_x + glyph.offset_max[0] * font_scale;
            let y0 =
                label.offset_px[1] + (atlas.baseline_offset + glyph.offset_min[1]) * font_scale;
            let y1 =
                label.offset_px[1] + (atlas.baseline_offset + glyph.offset_max[1]) * font_scale;
            if x1 > x0 && y1 > y0 {
                append_label_quad(
                    &mut vertices,
                    label.anchor,
                    [x0 + 1.5, y0 + 1.5],
                    [x1 + 1.5, y1 + 1.5],
                    glyph.uv_min,
                    glyph.uv_max,
                    2,
                );
                append_label_quad(
                    &mut vertices,
                    label.anchor,
                    [x0, y0],
                    [x1, y1],
                    glyph.uv_min,
                    glyph.uv_max,
                    0,
                );
            }
            pen_x += glyph.advance * font_scale;
        }
    }
    vertices
}

fn append_label_quad(
    vertices: &mut Vec<MeasurementLabelVertex>,
    anchor: [f32; 3],
    min: [f32; 2],
    max: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    kind: u32,
) {
    let make = |offset_px, uv| MeasurementLabelVertex {
        anchor,
        offset_px,
        uv,
        kind,
    };
    vertices.extend([
        make(min, uv_min),
        make([max[0], min[1]], [uv_max[0], uv_min[1]]),
        make(max, uv_max),
        make(min, uv_min),
        make(max, uv_max),
        make([min[0], max[1]], [uv_min[0], uv_max[1]]),
    ]);
}

fn append_connector_quad(
    vertices: &mut Vec<MeasurementLabelVertex>,
    anchor: [f32; 3],
    endpoint: [f32; 2],
    width: f32,
    extension: f32,
) {
    let length = endpoint[0].hypot(endpoint[1]);
    if length < 0.01 {
        return;
    }
    let direction = [endpoint[0] / length, endpoint[1] / length];
    let normal = [-direction[1] * width * 0.5, direction[0] * width * 0.5];
    let end = [
        endpoint[0] + direction[0] * extension,
        endpoint[1] + direction[1] * extension,
    ];
    let p0 = [normal[0], normal[1]];
    let p1 = [-normal[0], -normal[1]];
    let p2 = [end[0] - normal[0], end[1] - normal[1]];
    let p3 = [end[0] + normal[0], end[1] + normal[1]];
    let make = |offset_px| MeasurementLabelVertex {
        anchor,
        offset_px,
        uv: [0.0, 0.0],
        kind: 3,
    };
    vertices.extend([make(p0), make(p1), make(p2), make(p0), make(p2), make(p3)]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_input::RenderMeasurementLabel;

    #[test]
    fn bundled_inter_medium_atlas_contains_measurement_glyphs() {
        let atlas = &*MEASUREMENT_FONT_ATLAS;
        assert_eq!(MEASUREMENT_ATLAS_FONT_SIZE_PX, 64.0);
        assert!(SUPPORTED_GLYPHS
            .chars()
            .all(|character| atlas.glyphs.contains_key(&character)));
        assert!(atlas.pixels.iter().any(|alpha| *alpha > 0));
    }

    #[test]
    fn inter_label_geometry_uses_one_quad_per_visible_character() {
        let vertices = label_vertices(&RenderMeasurementInput {
            object_id: crate::ObjectId(1),
            segments: Vec::new(),
            labels: vec![RenderMeasurementLabel {
                anchor: [1.0, 2.0, 3.0],
                text: "2.893".into(),
                offset_px: [0.0, 0.0],
            }],
            color: [1.0; 4],
            label_color: [1.0; 4],
            label_outline_color: [0.0; 4],
            label_bg_color: [0.0; 4],
            label_connector_color: [1.0; 4],
            label_size: 24.0,
            label_padding: [0.2, 0.2, 0.0],
            label_bg_outline: false,
            label_connector: false,
            label_connector_width: 2.0,
            label_connector_ext_length: 2.5,
            label_shadow_mode: 0,
            label_z_target: 0,
            dash_width: 2.5,
        });
        assert_eq!(vertices.len(), (1 + 5 * 2) * 6);
        assert!(vertices
            .iter()
            .all(|vertex| vertex.anchor == [1.0, 2.0, 3.0]));
    }
}
