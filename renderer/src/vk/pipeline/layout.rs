use std::sync::Arc;
use ash::vk;
use crate::traits::handle::Handle;
use crate::vk::logical_device::LogicalDevice;

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

    pub fn layouts(mut self, layouts : &[vk::DescriptorSetLayout]) -> Self {
        self.descriptor_sets.extend_from_slice(layouts);
        self
    }

    pub fn register_push_constants(mut self, constants : &[vk::PushConstantRange]) -> Self {
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

impl Handle<vk::PipelineLayout> for PipelineLayout {
    fn handle(&self) -> vk::PipelineLayout { self.layout }
}