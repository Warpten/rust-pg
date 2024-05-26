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

pub struct RenderPassCreateInfo {
    color_images   : Vec<(vk::Format, vk::SampleCountFlags, vk::AttachmentLoadOp, vk::AttachmentStoreOp, vk::ImageLayout)>,
    depth_images   : Vec<(vk::Format, vk::SampleCountFlags, vk::AttachmentLoadOp, vk::AttachmentStoreOp)>,
    resolve_images : Vec<(vk::Format, vk::ImageLayout)>,

    dependencies : Vec<vk::SubpassDependency>,
    subpasses : Vec<(vk::PipelineBindPoint, Vec<u32>, Vec<u32>, Option<u32>)>,
}

impl RenderPassCreateInfo {
    pub fn color_attachment(
        mut self,
        format : vk::Format,
        samples : vk::SampleCountFlags,
        load : vk::AttachmentLoadOp,
        store : vk::AttachmentStoreOp,
        final_layout : vk::ImageLayout
    ) -> Self {
        self.color_images.push((format, samples, load, store, final_layout));
        self
    }

    pub fn dependency(mut self,
        src_subpass : u32,
        dst_subpass : u32,
        src_stage_mask : vk::PipelineStageFlags,
        dst_stage_mask : vk::PipelineStageFlags,
        src_access_flags : vk::AccessFlags,
        dst_access_flags : vk::AccessFlags
    ) -> Self {
        self.dependencies.push(vk::SubpassDependency::default()
            .src_subpass(src_subpass)
            .dst_subpass(dst_subpass)
            .src_stage_mask(src_stage_mask)
            .dst_stage_mask(dst_stage_mask)
            .src_access_mask(src_access_flags)
            .dst_access_mask(dst_access_flags)
        );
        self
    }

    pub fn depth_attachment(
        mut self,
        format : vk::Format,
        samples : vk::SampleCountFlags,
        load : vk::AttachmentLoadOp,
        store : vk::AttachmentStoreOp
    ) -> Self {
        self.depth_images.push((format, samples, load, store));
        self
    }

    pub fn resolve_attachment(
        mut self,
        format : vk::Format,
        final_layout : vk::ImageLayout
    ) -> Self {
        self.resolve_images.push((format, final_layout));
        self
    }

    fn make_attachment_description(
        format : vk::Format,
        samples : vk::SampleCountFlags,
        load : vk::AttachmentLoadOp,
        store : vk::AttachmentStoreOp,
        initial_layout : vk::ImageLayout,
        final_layout : vk::ImageLayout
    ) -> vk::AttachmentDescription {
        vk::AttachmentDescription::default()
            .format(format)
            .samples(samples)
            .load_op(load)
            .store_op(store)
            .initial_layout(initial_layout)
            .final_layout(final_layout)
    }

    pub fn subpass(
        mut self,
        bind_point : vk::PipelineBindPoint,
        color_attachments : &[u32],
        resolve_attachments : &[u32],
        depth_attachment : Option<u32>
    ) -> Self {
        self.subpasses.push((bind_point, color_attachments.to_vec(), resolve_attachments.to_vec(), depth_attachment));
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> RenderPass {
        let mut descs = Vec::<vk::AttachmentDescription>::new();

        let mut attachment_index = 0;
        let mut color_attachment_refs = Vec::<vk::AttachmentReference>::new();
        for (format, samples, load, store, final_layout) in self.color_images {
            descs.push(Self::make_attachment_description(
                format,
                samples,
                load,
                store,
                vk::ImageLayout::UNDEFINED,
                final_layout
            ));
            color_attachment_refs.push(vk::AttachmentReference::default()
                .attachment(attachment_index)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            );

            attachment_index += 1;
        }

        let mut depth_attachment_refs = Vec::<vk::AttachmentReference>::new();
        for (format, samples, load, store) in self.depth_images {
            descs.push(Self::make_attachment_description(
                format,
                samples,
                load,
                store,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
            ));

            depth_attachment_refs.push(vk::AttachmentReference::default()
                .attachment(attachment_index)
                .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            );

            attachment_index += 1;
        }
    
        let mut resolve_attachment_refs = Vec::<vk::AttachmentReference>::new();
        for (format, final_layout) in self.resolve_images {
            descs.push(Self::make_attachment_description(
                format,
                vk::SampleCountFlags::TYPE_1,
                vk::AttachmentLoadOp::DONT_CARE,
                vk::AttachmentStoreOp::STORE,
                vk::ImageLayout::UNDEFINED,
                final_layout
            ));
            resolve_attachment_refs.push(vk::AttachmentReference::default()
                .attachment(attachment_index)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            );
            
            attachment_index += 1;
        }

        // This exists because the mapped arrays need to exist outside of the loop to satisfy the borrow checker.
        let subpass_data = self.subpasses.into_iter().map(|tuple| {
            let (bind_point, color_refs, resolve_refs, depth) = tuple;

            let colors = color_refs.into_iter()
                .map(|i| color_attachment_refs[i as usize])
                .collect::<Vec<_>>();
            let resolve = resolve_refs.into_iter()
                .map(|i| resolve_attachment_refs[i as usize])
                .collect::<Vec<_>>();

            (bind_point, colors, resolve, depth)
        }).collect::<Vec<_>>();

        let mut subpasses = vec![];
        for (bind_point, colors, resolve, depth) in &subpass_data {
            let mut subpass_description = vk::SubpassDescription::default()
                .pipeline_bind_point(*bind_point)
                .color_attachments(&colors)
                .resolve_attachments(&resolve);

            if let Some(depth) = depth {
                subpass_description = subpass_description.depth_stencil_attachment(&depth_attachment_refs[*depth as usize]);
            }

            subpasses.push(subpass_description);
        }

        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&descs)
            .subpasses(&subpasses)
            .dependencies(&self.dependencies);

        unsafe {
            let handle = device.handle()
                .create_render_pass(&create_info, None)
                .expect("Failed to create a render pass");
            
            RenderPass::new(device, handle)
        }
    }
}

impl Default for RenderPassCreateInfo {
    fn default() -> Self {
        Self {
            color_images: Default::default(),
            depth_images: Default::default(),
            resolve_images: Default::default(),
            
            dependencies: Default::default(),
            subpasses: Default::default()
        }
    }
}

impl RenderPass {
    pub fn find_supported_format(device : &Arc<LogicalDevice>, formats : &[vk::Format], tiling : vk::ImageTiling, flags : vk::FormatFeatureFlags) -> Option<vk::Format> {
        for &format in formats {
            let properties = device.physical_device.get_format_properties(format);
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

    pub fn new(device : &Arc<LogicalDevice>, handle : vk::RenderPass) -> RenderPass {
        Self {
            device : device.clone(),
            handle,
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