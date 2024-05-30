use std::sync::Arc;
use ash::vk;
use crate::make_handle;
use crate::traits::handle::{Handle, Handles};
use crate::vk::logical_device::LogicalDevice;
use crate::vk::renderer::Renderer;

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

    pub fn build(self, device : &Arc<LogicalDevice>) -> PipelineLayout {
        let create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&self.descriptor_sets)
            .push_constant_ranges(&self.push_constants);

        unsafe {
            let layout = device.handle()
                .create_pipeline_layout(&create_info, None)
                .expect("Pipeline layout creation failed");

            PipelineLayout {
                device : device.clone(),
                layout,
                info : self
            }
        }
    }
}

pub struct PipelineLayout {
    device : Arc<LogicalDevice>,
    layout : vk::PipelineLayout,
    info : PipelineLayoutInfo
}

make_handle! { PipelineLayout, vk::PipelineLayout, layout }

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_pipeline_layout(self.layout, None);
        }
    }
}