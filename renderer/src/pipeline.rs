use std::{fs, marker::PhantomData, ops::Range, path::PathBuf, sync::{Arc, Mutex}};

use ash::vk;
use gpu_allocator::vulkan::Allocator;
use shaderc::{CompileOptions, Compiler, EnvVersion, IncludeType, ResolvedInclude, ShaderKind};

use crate::{traits::BorrowHandle, Context, LogicalDevice};

pub struct Shader {
    device : Arc<LogicalDevice>,
    module : ash::vk::ShaderModule,
    flags : ash::vk::ShaderStageFlags,
    path : PathBuf,
}

fn translate_shader_kind(stage : ash::vk::ShaderStageFlags) -> ShaderKind {
    match stage {
        ash::vk::ShaderStageFlags::VERTEX => ShaderKind::Vertex,
        ash::vk::ShaderStageFlags::FRAGMENT => ShaderKind::Fragment,
        ash::vk::ShaderStageFlags::COMPUTE => ShaderKind::Compute,
        ash::vk::ShaderStageFlags::TESSELLATION_CONTROL => ShaderKind::TessControl,
        ash::vk::ShaderStageFlags::TESSELLATION_EVALUATION => ShaderKind::TessEvaluation,
        ash::vk::ShaderStageFlags::GEOMETRY => ShaderKind::Geometry,
        ash::vk::ShaderStageFlags::RAYGEN_KHR => ShaderKind::RayGeneration,
        ash::vk::ShaderStageFlags::ANY_HIT_KHR => ShaderKind::AnyHit,
        ash::vk::ShaderStageFlags::CLOSEST_HIT_KHR => ShaderKind::ClosestHit,
        ash::vk::ShaderStageFlags::MISS_KHR => ShaderKind::Miss,
        ash::vk::ShaderStageFlags::INTERSECTION_KHR => ShaderKind::Intersection,
        _ => panic!("Unsupported shader stage"),
    }
}

impl Shader {
    #[inline] pub fn device(&self) -> &Arc<LogicalDevice> { &self.device }

    pub fn new(device : Arc<LogicalDevice>, path : PathBuf, flags : ash::vk::ShaderStageFlags) -> Self {
        let compiler = Compiler::new().expect("Failed to initialize shader compiler");
        let mut options = CompileOptions::new().unwrap();
        #[cfg(debug_assertions)]
        options.set_generate_debug_info();
        options.set_target_spirv(shaderc::SpirvVersion::V1_6);
        options.set_target_env(shaderc::TargetEnv::Vulkan, EnvVersion::Vulkan1_3 as u32);
        options.set_include_callback(
            move |requested_source, include_type, origin_source, recursion_depth| {
                Err("Includes are not supported yet".to_owned())
            }
        );

        let source = fs::read_to_string(path.as_path()).unwrap();

        let shader_kind = translate_shader_kind(flags);
        let code = compiler.compile_into_spirv(&source,
            shader_kind,
            path.file_name().unwrap().to_str().unwrap(),
            "main",
            Some(&options)
        ).unwrap();

        let shader_info = ash::vk::ShaderModuleCreateInfo::default()
            .code(code.as_binary());

        let module = unsafe {
            device.handle().create_shader_module(&shader_info, None)
                .unwrap()
        };

        Self {
            device : device.clone(),
            module,
            flags,
            path
        }
    }

    pub fn stage_info(&self, spec : Option<ash::vk::SpecializationInfo>) -> ash::vk::PipelineShaderStageCreateInfo {
        let create_info = ash::vk::PipelineShaderStageCreateInfo::default()
            .stage(self.flags)
            .module(self.module);

        if spec.is_some() {
            _ = create_info.specialization_info(&spec.unwrap());
        }

        create_info
    }
}
 
impl BorrowHandle for Shader {
    type Target = ash::vk::ShaderModule;

    fn handle(&self) -> &Self::Target { &self.module }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_shader_module(self.module, None);
        }
    }
}

pub struct PipelineInfo {
    layout : ash::vk::PipelineLayout,
    render_pass : Option<ash::vk::RenderPass>,
    shaders : Vec<(PathBuf, ash::vk::ShaderStageFlags)>,
    depth : DepthOptions,
    cull_mode : ash::vk::CullModeFlags,
    front_face : ash::vk::FrontFace,

    specialization_data: Vec<u8>,
    specialization_entries: Vec<vk::SpecializationMapEntry>,
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

    pub fn build(&self) -> ash::vk::PipelineDepthStencilStateCreateInfo {
        let info = ash::vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(self.test)
            .depth_write_enable(self.write)
            .depth_compare_op(ash::vk::CompareOp::LESS);

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
    pub fn depth(&self) -> &DepthOptions { &self.depth }

    pub fn layout(mut self, layout : vk::PipelineLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn render_pass(mut self, render_pass: vk::RenderPass) -> Self {
        self.render_pass = Some(render_pass);
        self
    }

    pub fn add_shader(mut self, path : PathBuf, flags : vk::ShaderStageFlags) -> Self {
        self.shaders.push((path, flags));
        self
    }

    pub fn add_specialization<T>(mut self, data : &T, constant_id : u32) -> Self {
        let slice = unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of_val(data))
        };

        let offset = self.specialization_data.len();
        self.specialization_data.append(&mut slice.to_vec());
        self.specialization_entries.push(ash::vk::SpecializationMapEntry::default()
            .constant_id(constant_id)
            .offset(offset as _)
            .size(self.specialization_data.len()));
        self
    }
}

impl Default for PipelineInfo {
    fn default() -> Self {
        Self {
            layout: ash::vk::PipelineLayout::default(),
            render_pass: None,
            shaders: vec![],
            depth : DepthOptions {
                test : true,
                write : true,
                bounds : None
            },
            cull_mode: ash::vk::CullModeFlags::BACK,
            front_face: ash::vk::FrontFace::COUNTER_CLOCKWISE,

            specialization_data : vec![],
            specialization_entries : vec![],
        }
    }
}

struct Pipeline<V : VertexType> {
    device : Arc<LogicalDevice>,
    info : PipelineInfo,
    handle : ash::vk::Pipeline,

    _marker : PhantomData<V>,
}

pub trait VertexType {
    fn attributes() -> Vec<ash::vk::VertexInputAttributeDescription>;
    fn bindings() -> Vec<ash::vk::VertexInputBindingDescription>;
}

impl<V : VertexType> Pipeline<V> {
    #[inline] pub fn device(&self) -> &Arc<LogicalDevice> { &self.device }
    #[inline] pub fn context(&self) -> &Arc<Context> { self.device().context() }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { self.device().allocator() }

    pub fn new(device : Arc<LogicalDevice>, pool : Option<&Arc<PipelinePool>>, name : &'static str, info : PipelineInfo) -> Self {
        let shaders = info.shaders.iter()
            .cloned() // TODO: remove this
            .map(|(path, flags)| Shader::new(device.clone(), path, flags))
            .collect::<Vec<_>>();

        let shader_stage_create_infos = shaders.iter().map(|shader| {
            if info.specialization_entries.is_empty() {
                shader.stage_info(None)
            } else {
                shader.stage_info(ash::vk::SpecializationInfo::default()
                    .map_entries(&info.specialization_entries)
                    .data(&info.specialization_data)
                    .into())
            }
        }).collect::<Vec<_>>();

        let viewport_state = ash::vk::PipelineViewportStateCreateInfo::default()
            .scissor_count(1)
            .viewport_count(1);

        let dynamic_state = ash::vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[
                ash::vk::DynamicState::VIEWPORT,
                ash::vk::DynamicState::SCISSOR
            ]);

        let vertex_attributes = V::attributes();
        let vertex_bindings = V::bindings();
        let vertex_input_state = ash::vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_attributes)
            .vertex_binding_descriptions(&vertex_bindings);

        let input_assembly_state = ash::vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST);

        // TODO: Allow for depth bias configuration
        let rasterization_state = ash::vk::PipelineRasterizationStateCreateInfo::default()
            .cull_mode(info.cull_mode)
            .line_width(1.0f32) // Any value larger than 1 requires a GPU feature
            .polygon_mode(ash::vk::PolygonMode::FILL)
            .front_face(info.front_face);
        
        let multisample_state = ash::vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(ash::vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0f32)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let depth_stencil_state = info.depth().build();

        let color_blend_state = ash::vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(ash::vk::LogicOp::CLEAR);

        let create_info = ash::vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos[..])
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .render_pass(ash::vk::RenderPass::default())
            .layout(info.layout);

        let pipelines = unsafe {
            let pool_handle = pool.map(|p| p.handle())
                .copied()
                .unwrap_or(ash::vk::PipelineCache::null());

            device.handle().create_graphics_pipelines(pool_handle, &[create_info], None)
                .expect("Creating a graphics pipeline failed")
        };

        Self {
            device,
            handle : pipelines[0],
            info,

            _marker : PhantomData::default(),
        }
    }
}

pub struct PipelinePool {
    device : Arc<LogicalDevice>,
    cache : ash::vk::PipelineCache,

    path : PathBuf,
}

impl PipelinePool {
    pub fn new(device : Arc<LogicalDevice>, path : PathBuf) -> Self {
        let data = fs::read(path.as_path()).unwrap_or(vec![]);
        
        let create_info = ash::vk::PipelineCacheCreateInfo::default()
            .initial_data(&data);

        let cache = unsafe {
            device.handle().create_pipeline_cache(&create_info, None)
                .expect("An error occured while creating a pipeline cache")
        };

        Self { cache, path, device }
    }

    pub fn save(&self) {
        unsafe {
            let data = self.device.handle().get_pipeline_cache_data(self.cache).unwrap_or(vec![]);

            _ = fs::write(self.path.as_path(), data);
        }
    }
}

impl BorrowHandle for PipelinePool {
    type Target = ash::vk::PipelineCache;

    fn handle(&self) -> &Self::Target { &self.cache }
}

impl Drop for PipelinePool {
    fn drop(&mut self) { self.save() }
}