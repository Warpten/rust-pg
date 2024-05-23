use std::sync::Arc;

use crate::{traits::handle::{BorrowHandle, Handle}, vk::LogicalDevice};

use super::Image;

pub struct RenderPass {
    handle : ash::vk::RenderPass,
    device : Arc<LogicalDevice>,
}

pub struct RenderPassInfo<'a> {
    pub color_images : Vec<&'a Image>,
    pub depth_images : Vec<&'a Image>,
    pub present : bool,
    pub final_layout : ash::vk::ImageLayout,
}

impl RenderPass {
    pub fn find_supported_format(device : &Arc<LogicalDevice>, formats : &[ash::vk::Format], tiling : ash::vk::ImageTiling, flags : ash::vk::FormatFeatureFlags) -> Option<ash::vk::Format> {
        for &format in formats {
            let properties = device.physical_device().get_format_properties(format);
            if let Some(properties) = properties {
                let supported = match tiling {
                    ash::vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(flags),
                    ash::vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(flags),
                    _ => panic!("Unsupported tiling mode")
                };

                if supported {
                    return Some(format);
                }
            }
        }

        None
    }

    pub fn new(device : &Arc<LogicalDevice>, info : RenderPassInfo) -> RenderPass {
        unsafe { // Everything is unsafe here, God has abandonned us.
            let mut descs = Vec::<ash::vk::AttachmentDescription>::new();

            let mut attachment_index = 0;
            let mut color_attachment_refs = Vec::<ash::vk::AttachmentReference>::new();
            for color_image in info.color_images {
                let mut layout = ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                // Fix this for multisampling
                // if info.present {
                //     layout = info.final_layout;
                // }

                descs.push(ash::vk::AttachmentDescription::default()
                    .format(color_image.format())
                    .samples(ash::vk::SampleCountFlags::TYPE_1)
                    .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                    .store_op(ash::vk::AttachmentStoreOp::STORE)
                    .final_layout(layout)
                );
                color_attachment_refs.push(ash::vk::AttachmentReference::default()
                    .attachment(attachment_index)
                    .layout(layout)
                );

                attachment_index += 1;
            }

            let mut depth_attachment_refs = Vec::<ash::vk::AttachmentReference>::new();
            for depth_image in info.depth_images {
                descs.push(ash::vk::AttachmentDescription::default()
                    .format(depth_image.format())
                    .samples(ash::vk::SampleCountFlags::TYPE_1)
                    .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                    .store_op(ash::vk::AttachmentStoreOp::STORE)
                    .initial_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .final_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                );
                depth_attachment_refs.push(ash::vk::AttachmentReference::default()
                    .attachment(attachment_index)
                    .layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                );
                
                attachment_index += 1;
            }

            let dependencies = [
                ash::vk::SubpassDependency::default()
                    .src_subpass(ash::vk::SUBPASS_EXTERNAL)
                    .src_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            ];

            let subpasses = [
                ash::vk::SubpassDescription::default()
                    .color_attachments(&color_attachment_refs)
                    .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                    .depth_stencil_attachment(&depth_attachment_refs[0])
            ];

            let create_info = ash::vk::RenderPassCreateInfo::default()
                .attachments(&descs)
                .subpasses(&subpasses)
                .dependencies(&dependencies);

            let render_pass = device.handle().create_render_pass(&create_info, None)
                .expect("Failed to create render pass");

            Self {
                device : device.clone(),
                handle : render_pass,
            }
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