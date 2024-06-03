use std::sync::Arc;

use ash::vk;

use crate::make_handle;
use crate::vk::logical_device::LogicalDevice;

use super::framebuffer::Framebuffer;
use super::swapchain::{Swapchain, SwapchainImage};

pub struct RenderPass {
    handle : vk::RenderPass,
    device : Arc<LogicalDevice>,

    spec : RenderPassAttachmentSpec,
}

impl RenderPass {
    pub fn builder() -> RenderPassCreateInfo {
        RenderPassCreateInfo::default()
    }

    /// Returns a framebuffer that is compatible with this render pass and the given swap chain.
    /// 
    /// # Arguments
    /// 
    /// * `swapchain` - The swapchain for which a framebuffer is created
    /// * `image` - An image from the swapchain.
    pub fn create_framebuffer(&self, swapchain : &Arc<Swapchain>, image : &SwapchainImage) -> Framebuffer {
        let mut attachments = vec![];

        // The attachments on this render pass dictates what we pull from the swapchain image
        let has_color = !self.spec.color_images.is_empty();
        let has_depth = !self.spec.depth_images.is_empty();
        let has_resolve = !self.spec.resolve_images.is_empty();

        let resolve = match &image.resolve {
            Some(resolve) => resolve.view(),
            None => vk::ImageView::null(),
        };
        let depth = match &image.depth {
            Some(depth) => depth.view(),
            None => vk::ImageView::null(),
        };
        let color = image.present.view();

        if has_resolve {
            attachments.push(resolve);
            if has_depth { attachments.push(depth); }
            attachments.push(color);
        } else {
            attachments.push(color);
            if has_depth { attachments.push(depth); }
        }
        
        Framebuffer::new(&self.device, vk::FramebufferCreateInfo::default()
            .width(swapchain.extent.width)
            .height(swapchain.extent.height)
            .render_pass(self.handle)
            .layers(swapchain.layer_count())
            .attachments(&attachments))
    }

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

    pub(in crate) fn new(device : &Arc<LogicalDevice>, handle : vk::RenderPass, spec : RenderPassAttachmentSpec) -> RenderPass {
        Self {
            device : device.clone(),
            handle,
            spec,
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

pub struct RenderPassAttachmentSpec {
    pub color_images   : Vec<(vk::Format, vk::SampleCountFlags, vk::AttachmentLoadOp, vk::AttachmentStoreOp, vk::ImageLayout, vk::ImageLayout)>,
    pub depth_images   : Vec<(vk::Format, vk::SampleCountFlags, vk::AttachmentLoadOp, vk::AttachmentStoreOp)>,
    pub resolve_images : Vec<(vk::Format, vk::ImageLayout)>,
}

pub struct RenderPassCreateInfo {
    spec : RenderPassAttachmentSpec,

    dependencies : Vec<vk::SubpassDependency>,
    subpasses : Vec<(vk::PipelineBindPoint, Vec<SubpassAttachment>, Option<SubpassAttachment>)>,
}

impl RenderPassCreateInfo {
    /// Adds a color attachment.
    /// 
    /// # Arguments
    /// 
    /// * `format` - The format of this attachment.
    /// * `samples` - The amount of samples to use.
    /// * `load` - The operation to use when this render pass begins.
    /// * `store` - The operation to use when this render pass finishes.
    /// * `final_layout` - The final layuout this attachment should be in when the render pass finished.
    pub fn color_attachment(
        mut self,
        format : vk::Format,
        samples : vk::SampleCountFlags,
        load : vk::AttachmentLoadOp,
        store : vk::AttachmentStoreOp,
        initial_layout : vk::ImageLayout,
        final_layout : vk::ImageLayout
    ) -> Self {
        self.spec.color_images.push((format, samples, load, store, initial_layout, final_layout));
        self
    }

    /// Expresses a dependency between two subpasses.
    /// 
    /// Arguments
    /// 
    /// * `src_subpass` - The subpass that is about to finish.
    /// * `dst_subpass` - The subpass that is about to begin.
    /// * `src_stage_mask` -
    /// * `dst_stage_mask` - 
    /// * `src_access_flags` -
    /// * `dst_access_flags` - 
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

    /// Adds a depth attachment.
    /// 
    /// # Arguments
    /// 
    /// * `format` - The format of this attachment.
    /// * `samples` - The amount of samples to use.
    /// * `load` - The operation to use when this render pass begins.
    /// * `store` - The operation to use when this render pass finishes.
    pub fn depth_attachment(
        mut self,
        format : vk::Format,
        samples : vk::SampleCountFlags,
        load : vk::AttachmentLoadOp,
        store : vk::AttachmentStoreOp
    ) -> Self {
        self.spec.depth_images.push((format, samples, load, store));
        self
    }

    /// Adds a multisampling resolve attachment.
    /// 
    /// # Arguments
    /// 
    /// * `format` - The format of this attachment.
    /// * `final_layout` - The final layuout this attachment should be in when the render pass finished.
    pub fn resolve_attachment(
        mut self,
        format : vk::Format,
        final_layout : vk::ImageLayout
    ) -> Self {
        self.spec.resolve_images.push((format, final_layout));
        self
    }

    fn make_attachment_description(
        format : vk::Format,
        samples : vk::SampleCountFlags,
        color_depth : (vk::AttachmentLoadOp, vk::AttachmentStoreOp),
        stencil : (vk::AttachmentLoadOp, vk::AttachmentStoreOp),
        initial_layout : vk::ImageLayout,
        final_layout : vk::ImageLayout
    ) -> vk::AttachmentDescription {
        vk::AttachmentDescription::default()
            .format(format)
            .samples(samples)
            .load_op(color_depth.0)
            .store_op(color_depth.1)
            .stencil_load_op(stencil.0)
            .stencil_store_op(stencil.1)
            .initial_layout(initial_layout)
            .final_layout(final_layout)
    }

    /// Declares a new subpass.
    /// 
    /// # Description
    /// 
    /// This function provides the ability to declare subpasses on the current render pass along with their attachments.
    /// It works by expecting an array of indices into the attachments registered via [`color_attachment`](Self::color_attachment),
    /// [`depth_attachment`](Self::depth_attachment) or [`resolve_attachment`](Self::resolve_attachment). Each one of this indices
    /// can then be interepreted as a color or resolve attachment for the subpass, allowing, for example, to alias the depth buffer
    /// as a color texture for a specific render pass.
    /// 
    /// It also takes in a single index as a depth attachment. In this case, the attachment must be referenced as a
    /// [`SubpassAttachment::depth`] attachment.
    /// 
    /// # Arguments
    /// 
    /// * `bind_point` - The pipeline type supported by this subpass.
    /// * `attachments` - An array of indices mapping to the attachments of this render pass.
    /// * `depth_attachment` - An indice mapping to one of the attachments of this render pass.
    pub fn subpass(
        mut self,
        bind_point : vk::PipelineBindPoint,
        attachments : &[SubpassAttachment],
        depth_attachment : Option<SubpassAttachment>
    ) -> Self {
        self.subpasses.push((bind_point, attachments.to_vec(), depth_attachment));
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> RenderPass {
        let mut descs = Vec::<vk::AttachmentDescription>::new();

        let mut attachment_index = 0;
        let mut color_attachment_refs = Vec::<vk::AttachmentReference>::new();
        for (format, samples, load, store, initial_layout, final_layout) in &self.spec.color_images {
            descs.push(Self::make_attachment_description(
                *format,
                *samples,
                (*load, *store),
                (vk::AttachmentLoadOp::DONT_CARE, vk::AttachmentStoreOp::DONT_CARE),
                *initial_layout,
                *final_layout
            ));
            color_attachment_refs.push(vk::AttachmentReference::default()
                .attachment(attachment_index)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            );

            attachment_index += 1;
        }

        let mut depth_attachment_refs = Vec::<vk::AttachmentReference>::new();
        for (format, samples, load, store) in &self.spec.depth_images {
            descs.push(Self::make_attachment_description(
                *format,
                *samples,
                (vk::AttachmentLoadOp::DONT_CARE, vk::AttachmentStoreOp::DONT_CARE),
                (*load, *store),
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
        for (format, final_layout) in &self.spec.resolve_images {
            descs.push(Self::make_attachment_description(
                *format,
                vk::SampleCountFlags::TYPE_1,
                (vk::AttachmentLoadOp::DONT_CARE, vk::AttachmentStoreOp::STORE),
                (vk::AttachmentLoadOp::DONT_CARE, vk::AttachmentStoreOp::DONT_CARE),
                vk::ImageLayout::UNDEFINED,
                *final_layout
            ));
            resolve_attachment_refs.push(vk::AttachmentReference::default()
                .attachment(attachment_index)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            );
            
            attachment_index += 1;
        }

        // This exists because the mapped arrays need to exist outside of the loop to satisfy the borrow checker.
        let subpass_data = self.subpasses.into_iter().map(|tuple| {
            let (bind_point, attachments, depth) = tuple;

            let mut colors = vec![];
            let mut resolves = vec![];

            for attachment in &attachments {
                match attachment {
                    SubpassAttachment::Color(index) => {
                        colors.push(color_attachment_refs[*index as usize])
                    },
                    SubpassAttachment::Resolve(index) => {
                        resolves.push(resolve_attachment_refs[*index as usize])
                    },
                    _ => panic!("Invalid subpass attachment"),
                };
            }

            (bind_point, colors, resolves, depth)
        }).collect::<Vec<_>>();

        let mut subpasses = vec![];
        for (bind_point, colors, resolve, depth) in &subpass_data {
            let mut subpass_description = vk::SubpassDescription::default()
                .pipeline_bind_point(*bind_point)
                .color_attachments(colors);

            if !resolve.is_empty() {
                subpass_description = subpass_description.resolve_attachments(resolve);
            }
            
            if let Some(depth) = depth {
                match depth {
                    SubpassAttachment::Depth(index) => {
                        subpass_description = subpass_description.depth_stencil_attachment(&depth_attachment_refs[*index as usize]);
                    },
                    _ => panic!("Invalid depth attachment"),
                }
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
            
            RenderPass::new(device, handle, self.spec)
        }
    }
}

impl Default for RenderPassCreateInfo {
    fn default() -> Self {
        Self {
            spec : RenderPassAttachmentSpec {
                color_images: vec![],
                depth_images: vec![],
                resolve_images: vec![],
            },
            
            dependencies: Default::default(),
            subpasses: Default::default()
        }
    }
}

#[derive(Copy, Clone)]
pub enum SubpassAttachment {
    Color(u32),
    Depth(u32),
    Resolve(u32),
}

impl SubpassAttachment {
    pub fn color(index : u32) -> Self { Self::Color(index) }
    pub fn depth(index : u32) -> Self { Self::Depth(index) }
    pub fn resolve(index : u32) -> Self { Self::Resolve(index) }
}