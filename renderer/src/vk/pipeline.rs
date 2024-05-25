use std::{fs, ops::Range, path::PathBuf, sync::{Arc, Mutex}};

use ash::vk;
use gpu_allocator::vulkan::Allocator;
use crate::{traits::handle::Handle, vk::context::Context};
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
    fn format_offset() -> Vec<(vk::Format, u32)>;
}

pub struct PipelineInfo {
    layout : vk::PipelineLayout,
    render_pass : Option<vk::RenderPass>,
    shaders : Vec<(PathBuf, vk::ShaderStageFlags)>,
    depth : DepthOptions,
    cull_mode : vk::CullModeFlags,
    front_face : vk::FrontFace,

    specialization_data: Vec<u8>,
    specialization_entries: Vec<vk::SpecializationMapEntry>,

    vertex_format_offset : Vec<(vk::Format, u32)>,
    vertex_bindings : Vec<(u32, vk::VertexInputRate)>,
    samples : vk::SampleCountFlags,
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

impl PipelineInfo {
    #[inline] pub fn depth(&self) -> &DepthOptions { &self.depth }

    #[inline] pub fn layout(mut self, layout : vk::PipelineLayout) -> Self {
        self.layout = layout;
        self
    }

    #[inline] pub fn render_pass(mut self, render_pass : vk::RenderPass) -> Self {
        self.render_pass = Some(render_pass);
        self
    }

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

    #[inline] pub fn cull_mode(mut self, mode : vk::CullModeFlags) -> Self {
        self.cull_mode = mode;
        self
    }

    #[inline] pub fn vertex<T : Vertex>(mut self) -> Self {
        self.vertex_format_offset = T::format_offset();
        self.vertex_bindings = T::bindings();
        self
    }

    #[inline] pub fn samples(mut self, samples : vk::SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }

    #[inline] pub fn front_face(mut self, front : vk::FrontFace) -> Self {
        self.front_face = front;
        self
    }
}

impl Default for PipelineInfo {
    fn default() -> Self {
        Self {
            layout: vk::PipelineLayout::default(),
            render_pass: None,
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

            vertex_bindings : vec![],
            vertex_format_offset : vec![],
        }
    }
}

pub struct Pipeline {
    device : Arc<LogicalDevice>,
    info : PipelineInfo,
    handle : vk::Pipeline,
}

impl Pipeline {
    #[inline] pub fn device(&self) -> &Arc<LogicalDevice> { &self.device }
    #[inline] pub fn context(&self) -> &Arc<Context> { self.device().context() }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { self.device().allocator() }

    pub fn new(device : &Arc<LogicalDevice>, pool : Option<&Arc<PipelinePool>>, info : PipelineInfo) -> Self {
        let shaders = info.shaders.iter()
            .cloned() // TODO: remove this
            .map(|(path, flags)| Shader::new(device.clone(), path, flags))
            .collect::<Vec<_>>();

        let shader_stage_create_infos = shaders.iter().map(|shader| {
            if info.specialization_entries.is_empty() {
                shader.stage_info(None)
            } else {
                shader.stage_info(vk::SpecializationInfo::default()
                    .map_entries(&info.specialization_entries)
                    .data(&info.specialization_data)
                    .into())
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

        let vertex_attributes = {
            let mut descs = vec![];
            for (i, tpl) in info.vertex_format_offset.iter().enumerate() {
                descs.push(vk::VertexInputAttributeDescription::default()
                    .binding(0) // ?
                    .location(i as u32)
                    .format(tpl.0)
                    .offset(tpl.1)
                );
            }
            descs
        };
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
            .vertex_attribute_descriptions(&vertex_attributes)
            .vertex_binding_descriptions(&vertex_bindings);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // TODO: Allow for depth bias configuration
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .cull_mode(info.cull_mode)
            .line_width(1.0f32) // Any value larger than 1 requires a GPU feature
            .polygon_mode(vk::PolygonMode::FILL)
            .front_face(info.front_face);
        
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0f32)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let depth_stencil_state = info.depth().build();

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR);

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
            .render_pass(vk::RenderPass::default())
            .layout(info.layout);

        let pipelines = unsafe {
            let pool_handle = pool.map(|p| p.handle())
                .unwrap_or(vk::PipelineCache::null());

            device.handle().create_graphics_pipelines(pool_handle, &[create_info], None)
                .expect("Creating a graphics pipeline failed")
        };

        Self {
            device : device.clone(),
            handle : pipelines[0],
            info,
        }
    }
}

impl Handle<vk::Pipeline> for Pipeline {
    fn handle(&self) -> vk::Pipeline { self.handle }
}
