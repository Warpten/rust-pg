use std::{ffi::CStr, fs};
use std::path::PathBuf;
use ash::vk;
use shaderc::{CompileOptions, Compiler, EnvVersion, ShaderKind};
use crate::make_handle;
use crate::orchestration::rendering::RenderingContext;

pub struct Shader {
    context : RenderingContext,
    module : vk::ShaderModule,
    flags : vk::ShaderStageFlags,
    path : PathBuf,
}

fn translate_shader_kind(stage : vk::ShaderStageFlags) -> ShaderKind {
    match stage {
        vk::ShaderStageFlags::VERTEX => ShaderKind::Vertex,
        vk::ShaderStageFlags::FRAGMENT => ShaderKind::Fragment,
        vk::ShaderStageFlags::COMPUTE => ShaderKind::Compute,
        vk::ShaderStageFlags::TESSELLATION_CONTROL => ShaderKind::TessControl,
        vk::ShaderStageFlags::TESSELLATION_EVALUATION => ShaderKind::TessEvaluation,
        vk::ShaderStageFlags::GEOMETRY => ShaderKind::Geometry,
        vk::ShaderStageFlags::RAYGEN_KHR => ShaderKind::RayGeneration,
        vk::ShaderStageFlags::ANY_HIT_KHR => ShaderKind::AnyHit,
        vk::ShaderStageFlags::CLOSEST_HIT_KHR => ShaderKind::ClosestHit,
        vk::ShaderStageFlags::MISS_KHR => ShaderKind::Miss,
        vk::ShaderStageFlags::INTERSECTION_KHR => ShaderKind::Intersection,
        _ => panic!("Unsupported shader stage"),
    }
}

impl Shader {
    pub fn new(context : &RenderingContext, path : PathBuf, flags : vk::ShaderStageFlags) -> Self {
        let compiler = Compiler::new().expect("Failed to initialize shader compiler");
        let mut options = CompileOptions::new().unwrap();
        #[cfg(debug_assertions)]
        options.set_generate_debug_info();
        options.set_target_spirv(shaderc::SpirvVersion::V1_6);
        options.set_target_env(shaderc::TargetEnv::Vulkan, EnvVersion::Vulkan1_3 as u32);
        options.set_include_callback(
            move |_requested_source, _include_type, _origin_source, _recursion_depth| {
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

        let shader_info = vk::ShaderModuleCreateInfo::default()
            .code(code.as_binary());

        let module = unsafe {
            context.device.handle().create_shader_module(&shader_info, None)
                .unwrap()
        };

        if let Some(path) = path.to_str() {
            context.device.set_handle_name(module, &path.to_owned());
        }
        
        Self {
            context : context.clone(),
            module,
            flags,
            path
        }
    }

    pub fn stage_info<'a>(&self, spec : Option<vk::SpecializationInfo>, name : &'a CStr) -> vk::PipelineShaderStageCreateInfo<'a> {
        let create_info = vk::PipelineShaderStageCreateInfo::default()
            .name(name)
            .stage(self.flags)
            .module(self.module);

        if spec.is_some() {
            _ = create_info.specialization_info(&spec.unwrap());
        }

        create_info
    }
}

make_handle! { Shader, vk::ShaderModule, module }

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.context.device.handle().destroy_shader_module(self.module, None);
        }
    }
}