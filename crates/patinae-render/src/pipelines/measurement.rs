use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use crate::context::RenderContext;
use crate::measurement_geometry::{
    MeasurementLabelVertex, MeasurementVertex, MEASUREMENT_ATLAS_HEIGHT, MEASUREMENT_ATLAS_WIDTH,
    MEASUREMENT_FONT_ATLAS,
};
use crate::pipelines::build_draw_pair;
use crate::shader_source::MEASUREMENT_WGSL;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeasurementParams {
    pub color: [f32; 4],
    pub label_color: [f32; 4],
    pub label_outline_color: [f32; 4],
    pub label_bg_color: [f32; 4],
    pub label_connector_color: [f32; 4],
    pub dash_width: f32,
    pub label_bg_outline: u32,
    pub label_shadow_mode: u32,
    pub label_z_target: u32,
}

pub struct MeasurementParamsLayout {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub font_view: wgpu::TextureView,
    pub font_sampler: wgpu::Sampler,
}

impl MeasurementParamsLayout {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("patinae.measurement.params_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<MeasurementParams>() as u64
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let atlas = &*MEASUREMENT_FONT_ATLAS;
        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("patinae.measurement.inter_medium_14px"),
            size: wgpu::Extent3d {
                width: MEASUREMENT_ATLAS_WIDTH,
                height: MEASUREMENT_ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(MEASUREMENT_ATLAS_WIDTH),
                rows_per_image: Some(MEASUREMENT_ATLAS_HEIGHT),
            },
            wgpu::Extent3d {
                width: MEASUREMENT_ATLAS_WIDTH,
                height: MEASUREMENT_ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        let font_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("patinae.measurement.inter_medium_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Self {
            bind_group_layout,
            font_view,
            font_sampler,
        }
    }
}

pub struct MeasurementPipeline {
    pub translucent: wgpu::RenderPipeline,
    pub opaque: wgpu::RenderPipeline,
    pub label: wgpu::RenderPipeline,
}

impl MeasurementPipeline {
    pub fn new(ctx: &RenderContext, layout: &MeasurementParamsLayout) -> Self {
        let pair = build_draw_pair(
            ctx,
            "patinae.measurement",
            MEASUREMENT_WGSL,
            &[
                &ctx.frame.bind_group_layout,
                &ctx.lighting.bind_group_layout,
                &layout.bind_group_layout,
            ],
            MeasurementVertex::vertex_layout(),
            wgpu::PrimitiveTopology::TriangleList,
        );
        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("patinae.measurement_label.shader"),
                source: wgpu::ShaderSource::Wgsl(
                    crate::shader_source::expand(crate::shader_source::MEASUREMENT_LABEL_WGSL)
                        .into(),
                ),
            });
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("patinae.measurement_label.layout"),
                bind_group_layouts: &[
                    Some(&ctx.frame.bind_group_layout),
                    Some(&ctx.lighting.bind_group_layout),
                    Some(&layout.bind_group_layout),
                ],
                immediate_size: 0,
            });
        let label = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("patinae.measurement_label.pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[MeasurementLabelVertex::vertex_layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.color_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::frame::DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        Self {
            translucent: pair.translucent,
            opaque: pair.opaque,
            label,
        }
    }
}
