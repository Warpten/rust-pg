use ash::vk;
use crate::make_handle;
use crate::orchestration::rendering::RenderingContext;
use crate::traits::handle::{Handle, Handles};

#[derive(Default)]
pub struct PipelineLayoutInfo {
    pub flags : vk::PipelineLayoutCreateFlags,
    pub descriptor_sets : Vec<vk::DescriptorSetLayout>,
    pub push_constants : Vec<vk::PushConstantRange>,
}

impl PipelineLayoutInfo {
    pub fn layout<L>(mut self, layout : &L) -> Self
        where L : Handle<vk::DescriptorSetLayout>
    {
        self.descriptor_sets.push(layout.handle());
        self
    }

    pub fn layouts<L>(mut self, layout : &[L]) -> Self
        where L : Handle<vk::DescriptorSetLayout>
    {
        self.descriptor_sets.extend(layout.handles());
        self
    }

    pub fn push_constant(mut self, constant : vk::PushConstantRange) -> Self {
        self.push_constants.push(constant);
        self
    }

    pub fn push_constants(mut self, constants : &[vk::PushConstantRange]) -> Self {
        self.push_constants.extend_from_slice(constants);
        self
    }

    pub fn build(self, context : &RenderingContext) -> PipelineLayout {
        let create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&self.descriptor_sets)
            .push_constant_ranges(&self.push_constants);

        unsafe {
            let layout = context.device.handle()
                .create_pipeline_layout(&create_info, None)
                .expect("Pipeline layout creation failed");

            PipelineLayout {
                context : context.clone(),
                layout,
                info : self
            }
        }
    }
}

pub struct PipelineLayout {
    context : RenderingContext,
    layout : vk::PipelineLayout,
    info : PipelineLayoutInfo
}

make_handle! { PipelineLayout, vk::PipelineLayout, layout }

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.context.device.handle().destroy_pipeline_layout(self.layout, None);
        }
    }
}