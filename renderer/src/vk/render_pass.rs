use std::sync::Arc;

use ash::vk;

use crate::make_handle;
use crate::traits::handle::Handle;
use crate::vk::image::Image;
use crate::vk::logical_device::LogicalDevice;

pub struct RenderPass {
    handle : vk::RenderPass,
    device : Arc<LogicalDevice>,
}

pub struct RenderPassInfo<'a> {
    pub color_images   : Vec<&'a Image>,
    pub depth_images   : Vec<&'a Image>,
    pub resolve_images : Vec<&'a Image>,

    pub present : bool,
    pub final_layout : vk::ImageLayout,
    pub sample_count : vk::SampleCountFlags,
}

impl RenderPass {
    pub fn find_supported_format(device : &Arc<LogicalDevice>, formats : &[vk::Format], tiling : vk::ImageTiling, flags : vk::FormatFeatureFlags) -> Option<vk::Format> {
        for &format in formats {
            let properties = device.physical_device().get_format_properties(format);
            if let Some(properties) = properties {
                let supported = match tiling {
                    vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(flags),
                    vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(flags),
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
            let mut descs = Vec::<vk::AttachmentDescription>::new();

            let mut attachment_index = 0;
            let mut color_attachment_refs = Vec::<vk::AttachmentReference>::new();
            for color_image in info.color_images {
                let mut layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
                // Fix this for multisampling
                if info.present && info.resolve_images.is_empty() {
                    layout = info.final_layout;
                }

                descs.push(vk::AttachmentDescription::default()
                    .format(color_image.format())
                    .samples(info.sample_count)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .final_layout(layout)
                );
                color_attachment_refs.push(vk::AttachmentReference::default()
                    .attachment(attachment_index)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // Not reusing layout here?
                );

                attachment_index += 1;
            }

            let mut depth_attachment_refs = Vec::<vk::AttachmentReference>::new();
            if let Some(depth_image) = info.depth_images.get(0) {
                descs.push(vk::AttachmentDescription::default()
                    .format(depth_image.format())
                    .samples(info.sample_count)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                );
                depth_attachment_refs.push(vk::AttachmentReference::default()
                    .attachment(attachment_index)
                    .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                );
                
                attachment_index += 1;
            }

            let mut resolve_attachment_refs = Vec::<vk::AttachmentReference>::new();
            for resolve_image in &info.resolve_images {
                descs.push(vk::AttachmentDescription::default()
                    .format(resolve_image.format())
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(info.final_layout)
                );
                resolve_attachment_refs.push(vk::AttachmentReference::default()
                    .attachment(attachment_index)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                );
                
                attachment_index += 1;
            }

            let dependencies = [
                vk::SubpassDependency::default()
                    .src_subpass(vk::SUBPASS_EXTERNAL)
                    .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            ];

            let subpasses = [
                {
                    let mut desc = vk::SubpassDescription::default()
                        .color_attachments(&color_attachment_refs)
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
                    
                    if !info.depth_images.is_empty() {
                        desc = desc.depth_stencil_attachment(&depth_attachment_refs[0]);
                    }

                    if !info.resolve_images.is_empty() {
                        desc = desc.resolve_attachments(&resolve_attachment_refs);
                    }
                    desc
                }
            ];

            let create_info = vk::RenderPassCreateInfo::default()
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

make_handle! { RenderPass, vk::RenderPass }

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_render_pass(self.handle, None);
        }
    }
}