use std::{ffi::CString, ops::Range, path::PathBuf, sync::Arc};

use ash::vk;
use crate::{make_handle, traits::handle::Handle};
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::shader::Shader;

use self::pool::PipelinePool;

pub mod layout;
pub mod pipeline;
pub mod pool;
pub mod shader;

pub trait Vertex {
    /// Returns bindings in the appropriate order.
    ///
    /// # Description
    ///
    /// This function returns an array of tuples consisting of the stride of the input, and the rate
    /// of the input.
    fn bindings() -> Vec<(u32, vk::VertexInputRate)>;

    /// Returns formats and offsets of elements in this vertex.
    fn format_offset() -> Vec<vk::VertexInputAttributeDescription>;
}

pub struct PipelineInfo {
    name : Option<&'static str>,

    layout : vk::PipelineLayout,
    render_pass : vk::RenderPass,
    subpass : u32,
    shaders : Vec<(PathBuf, vk::ShaderStageFlags)>,
    depth : DepthOptions,
    cull_mode : vk::CullModeFlags,
    front_face : vk::FrontFace,
    topology : vk::PrimitiveTopology,

    specialization_data: Vec<u8>,
    specialization_entries: Vec<vk::SpecializationMapEntry>,

    vertex_format_offset : Vec<vk::VertexInputAttributeDescription>,
    vertex_bindings : Vec<(u32, vk::VertexInputRate)>,
    samples : vk::SampleCountFlags,
    pool : Option<Arc<PipelinePool>>,
}

impl PipelineInfo {
    #[inline] pub fn pool(mut self, pool : &Arc<PipelinePool>) -> Self {
        self.pool = Some(pool.clone());
        self
    }

    #[inline] pub fn name(mut self, name : &'static str) -> Self {
        self.name = Some(name);
        self
    }

    #[inline] pub fn render_pass(mut self, render_pass : vk::RenderPass, subpass : u32) -> Self {
        self.render_pass = render_pass;
        self.subpass = subpass;
        self
    }

    value_builder! { depth, depth, DepthOptions }
    value_builder! { layout, layout, vk::PipelineLayout }
    value_builder! { cull_mode, mode, cull_mode, vk::CullModeFlags }
    value_builder! { samples, samples, vk::SampleCountFlags }
    value_builder! { front_face, front, front_face, vk::FrontFace }
    value_builder! { topology, topology, vk::PrimitiveTopology }

    #[inline] pub fn add_shader(mut self, path : PathBuf, flags : vk::ShaderStageFlags) -> Self {
        self.shaders.push((path, flags));
        self
    }

    #[inline] pub fn add_specialization<T>(mut self, data : &T, constant_id : u32) -> Self {
        let slice = unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of_val(data))
        };

        let offset = self.specialization_data.len();
        self.specialization_data.append(&mut slice.to_vec());
        self.specialization_entries.push(vk::SpecializationMapEntry::default()
            .constant_id(constant_id)
            .offset(offset as _)
            .size(self.specialization_data.len()));
        self
    }

    #[inline] pub fn vertex<T : Vertex>(mut self) -> Self {
        self.vertex_format_offset = T::format_offset();
        self.vertex_bindings = T::bindings();
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> Pipeline {
        Pipeline::new(device, self)
    }
}

impl Default for PipelineInfo {
    fn default() -> Self {
        Self {
            name : Some("Default Pipeline"),

            layout: vk::PipelineLayout::default(),
            shaders: vec![],
            depth : DepthOptions {
                test : true,
                write : true,
                bounds : None
            },
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,

            specialization_data : vec![],
            specialization_entries : vec![],

            samples : vk::SampleCountFlags::TYPE_1,
            topology : vk::PrimitiveTopology::TRIANGLE_LIST,

            vertex_bindings : vec![],
            vertex_format_offset : vec![],

            pool : None,

            render_pass : vk::RenderPass::null(),
            subpass : 0,
        }
    }
}

pub struct DepthOptions {
    test : bool,
    write : bool,
    bounds : Option<Range<f32>>,
}

impl DepthOptions {
    /// Returns a new instance of [`DepthOptions`] where depth testing will be disabled in the pipeline.
    pub fn disabled() -> Self {
        Self { test : false, write : false, bounds : None }
    }

    /// Returns a new instance of [`DepthOptions`] where depth testing will be enabled in the pipeline.
    pub fn enabled() -> Self {
        Self { test : true, write : false, bounds : None }
    }

    #[inline] pub fn write(mut self, write : bool) -> Self {
        self.write = write;
        self
    }

    #[inline] pub fn bounds(mut self, bounds : Range<f32>) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn build(&self) -> vk::PipelineDepthStencilStateCreateInfo {
        let info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(self.test)
            .depth_write_enable(self.write)
            .depth_compare_op(vk::CompareOp::LESS);

        match &self.bounds {
            Some(bounds) => {
                info.depth_bounds_test_enable(true)
                    .min_depth_bounds(bounds.start)
                    .max_depth_bounds(bounds.end)
            },
            None => {
                info.depth_bounds_test_enable(false)
            }
        }
    }
}

pub struct Pipeline {
    device : Arc<LogicalDevice>,
    info : PipelineInfo,
    handle : vk::Pipeline,
}

impl Pipeline {
    #[inline] pub fn layout(&self) -> vk::PipelineLayout { self.info.layout }

    pub(in self) fn new(device : &Arc<LogicalDevice>, info : PipelineInfo) -> Self {
        let shaders = info.shaders.iter()
            .cloned() // TODO: remove this
            .map(|(path, flags)| Shader::new(device, path, flags))
            .collect::<Vec<_>>();

        let shader_names = CString::new("main").unwrap();

        let shader_stage_create_infos = shaders.iter().map(|shader| {
            if info.specialization_entries.is_empty() {
                shader.stage_info(None, &shader_names)
            } else {
                shader.stage_info(vk::SpecializationInfo::default()
                    .map_entries(&info.specialization_entries)
                    .data(&info.specialization_data)
                    .into(), &shader_names)
            }
        }).collect::<Vec<_>>();

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .scissor_count(1)
            .viewport_count(1);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[
                vk::DynamicState::VIEWPORT,
                vk::DynamicState::SCISSOR
            ]);

        let vertex_bindings = {
            let mut bindings = vec![];
            for (stride, rate) in &info.vertex_bindings {
                bindings.push(vk::VertexInputBindingDescription::default()
                    .binding(0)
                    .input_rate(*rate)
                    .stride(*stride)
                );
            }
            bindings
        };
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&info.vertex_format_offset)
            .vertex_binding_descriptions(&vertex_bindings);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(info.topology);

        // TODO: Allow for depth bias configuration
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .cull_mode(info.cull_mode)
            // .depth_clamp_enable(false)
            // .rasterizer_discard_enable(false)
            // .depth_bias_enable(false)
            .line_width(1.0f32) // Any value larger than 1 requires a GPU feature
            .polygon_mode(vk::PolygonMode::FILL)
            .front_face(info.front_face);
        
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(info.samples)
            .min_sample_shading(1.0f32)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let depth_stencil_state = info.depth.build();

        // TODO: This array needs to be synced with render_pass.subpasses[all].colorAttachmentCount
        let color_blend_attachment_states = [
            vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
        ];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .blend_constants([0.0f32; 4])
            .attachments(&color_blend_attachment_states);

        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos[..])
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .render_pass(info.render_pass)
            .subpass(info.subpass)
            .layout(info.layout);

        let pipelines = unsafe {
            let pool_handle = match &info.pool {
                Some(handle) => handle.handle(),
                None => vk::PipelineCache::null(),
            };

            device.handle().create_graphics_pipelines(pool_handle, &[create_info], None)
                .expect("Creating a graphics pipeline failed")
        };

        if let Some(name) = info.name {
            device.set_handle_name(pipelines[0], &name.to_owned());
        }

        Self {
            device : device.clone(),
            handle : pipelines[0],
            info,
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_pipeline(self.handle, None);
        }
    }
}

make_handle! { Pipeline, vk::Pipeline }
