use std::sync::Arc;

use crate::{traits::{BorrowHandle, Handle}, LogicalDevice, Swapchain};

pub struct RenderPass {
    handle : ash::vk::RenderPass,
    device : Arc<LogicalDevice>,
}

impl RenderPass {
    pub fn new(device : &Arc<LogicalDevice>, format : ash::vk::Format) -> RenderPass {
        let attachments = [
            ash::vk::AttachmentDescription::default()
                .format(format)
                .samples(ash::vk::SampleCountFlags::TYPE_1)
                .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                .store_op(ash::vk::AttachmentStoreOp::STORE)
                .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(ash::vk::ImageLayout::UNDEFINED)
                .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR),
            ash::vk::AttachmentDescription::default()
                .format(format)
                .samples(ash::vk::SampleCountFlags::TYPE_1)
                .load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
                .store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(ash::vk::AttachmentLoadOp::CLEAR)
                .stencil_store_op(ash::vk::AttachmentStoreOp::STORE)
                .initial_layout(ash::vk::ImageLayout::UNDEFINED)
                .final_layout(ash::vk::ImageLayout::STENCIL_ATTACHMENT_OPTIMAL)
        ];

        let attachment_references = [
            ash::vk::AttachmentReference::default()
                .attachment(0)
                .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            ash::vk::AttachmentReference::default()
                .attachment(1)
                .layout(ash::vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        ];

        let subpass_description = [
            ash::vk::SubpassDescription::default()
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&attachment_references[0..1])
                .depth_stencil_attachment(&attachment_references[1])
        ];

        let subpass_dependency = [
            ash::vk::SubpassDependency::default()
                .src_subpass(ash::vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | ash::vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .src_access_mask(ash::vk::AccessFlags::NONE)
                .dst_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | ash::vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE | ash::vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        ];

        let create_info = ash::vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpass_description)
            .dependencies(&subpass_dependency);

        let handle = unsafe {
            device.handle().create_render_pass(&create_info, None)
                .expect("Render pass creation failed")
        };

        Self {
            handle,
            device : device.clone()
        }
    }
}

impl Handle for RenderPass {
    type Target = ash::vk::RenderPass;

    fn handle(&self) -> Self::Target { self.handle }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_render_pass(self.handle, None);
        }
    }
}