use std::ops::Range;

use ash::vk;
use ash::prelude::VkResult;

use crate::orchestration::rendering::RenderingContext;
use crate::{make_handle, window::Window};
use crate::vk::context::Context;
use crate::vk::image::Image;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::queue::QueueFamily;
use crate::vk::render_pass::RenderPass;

use super::{image::ImageCreateInfo, render_pass::RenderPassCreateInfo};

/// Options that are used when creating a [`Swapchain`].
pub trait SwapchainOptions {
    /// Determines if the provided surface_format is the preferred format for the swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `format` - The format to test.
    /// 
    /// # Returns
    /// 
    /// This function should return `true` in one exact case; if it doesn't, whatever format is tested
    /// `true` first will be selected. If no format returns true, the first available format for will
    /// be selected.
    fn select_surface_format(&self, format : &vk::SurfaceFormatKHR) -> bool;

    /// Returns the width of the swapchain's images.
    fn width(&self) -> u32;

    /// Returns the height of the swapchain's images.
    fn height(&self) -> u32;

    /// Returns the composite flags to be used by the swapchain's images.
    fn composite_alpha(&self) -> vk::CompositeAlphaFlagsKHR { vk::CompositeAlphaFlagsKHR::OPAQUE }

    /// Returns the presentation mode of this swapchain.
    fn select_present_mode(&self, modes : Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
        for present_mode in modes {
            if present_mode == vk::PresentModeKHR::MAILBOX {
                return present_mode;
            }
        }

        vk::PresentModeKHR::FIFO
    }

    /// Returns the amount of layers of each texture of this swapchain.
    /// By default, there is only one layer.
    fn layers(&self) -> Range<u32> {
        return Range { start : 0, end : 1 }
    }

    /// Returns the swapchain's mip layout range.
    /// By default, there is only one layer.
    fn mip_range(&self) -> Range<u32> {
        return Range { start : 0, end : 1 }
    }

    fn depth(&self) -> bool;
    fn stencil(&self) -> bool;

    fn multisampling(&self) -> vk::SampleCountFlags { vk::SampleCountFlags::TYPE_1 }
}

pub struct SwapchainImage {
    pub present : Image,
    pub depth : Option<Image>,
    pub resolve : Option<Image>,
}

pub struct Swapchain {
    // Surface
    handle : vk::SwapchainKHR,
    pub loader : ash::khr::swapchain::Device,
    pub surface_format : vk::SurfaceFormatKHR,
    
    // Images
    pub extent : vk::Extent2D,
    pub images : Vec<SwapchainImage>,
    pub sample_count : vk::SampleCountFlags,
    layer_count : u32,

    // Queues
    pub queue_families : Vec<QueueFamily>,
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.images.clear();
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

impl Swapchain {
    pub fn new<T : SwapchainOptions>(
        context : &RenderingContext,
        options : &T,
        queue_families : Vec<QueueFamily>,
    ) -> Swapchain {
        let surface_format = Self::select_format(options, context.window.get_surface_formats(&context.device.physical_device));
        let surface_capabilities = context.window.get_surface_capabilities(&context.device.physical_device);
        let extent = Self::get_extent(surface_capabilities, options);

        let image_count = surface_capabilities.min_image_count + 1;
        let image_count = if surface_capabilities.max_image_count != 0 {
            image_count.min(surface_capabilities.max_image_count)
        } else {
            image_count
        };

        let present_modes = context.window.get_present_modes(&context.device.physical_device);

        let mut queue_family_indices = queue_families.iter().map(QueueFamily::index).collect::<Vec<_>>();
        queue_family_indices.dedup();
        let sharing_mode = if queue_family_indices.len() == 1 {
            vk::SharingMode::EXCLUSIVE
        } else {
            vk::SharingMode::CONCURRENT
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.window.surface())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            // Number of views in a multiview/stereo surface. For non-stereoscopic-3D applications, this value is 1.
            .image_array_layers(1)
            // A bitmask of VkImageUsageFlagBits describing the intended usage of the (acquired) swapchain images.
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::INPUT_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(if surface_capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            })
            // Indicates the alpha compositing mode to use when this surface is composited together with other
            // surfaces on certain window systems.
            .composite_alpha(options.composite_alpha())
            // Presentation mode the swapchain will use. A swapchain’s present mode determines how incoming present
            // requests will be processed and queued internally.
            .present_mode(options.select_present_mode(present_modes))
            // Specifies whether the Vulkan implementation is allowed to discard rendering operations that affect
            // regions of the surface that are not visible.
            .clipped(true);

        let loader = ash::khr::swapchain::Device::new(context.context.handle(), context.device.handle());
        let handle = unsafe {
            loader.create_swapchain(&create_info, None)
                .expect(format!("Failed to create swapchain with options {:?}", create_info).as_str())
        };

        let present_images = unsafe {
            let swapchain_images = loader.get_swapchain_images(handle)
                .expect("Failed to get swapchain images");

            Image::from_swapchain(&extent, context, surface_format.format, swapchain_images)
        };

        let mut images = vec![];
        for (i, present) in present_images.into_iter().enumerate() {
            let depth = Self::make_depth_image(context, sharing_mode, extent, format!("Swapchain/Depth[{}]", i), options);
            let resolve = Self::make_resolve_image(context, surface_format, sharing_mode, extent, format!("Swapchain/Resolve[{}]", i), options);

            images.push(SwapchainImage {
                present,
                depth,
                resolve
            })
        }

        Swapchain {
            handle,
            loader,
            surface_format,
            extent,
            images,
            sample_count : options.multisampling(),
            layer_count : options.layers().len() as _,
            queue_families : queue_families.clone(),
        }
    }


    fn make_depth_image<T : SwapchainOptions>(
        context : &RenderingContext,
        sharing_mode : vk::SharingMode,
        extent : vk::Extent2D,
        name : String,
        options : &T,
    ) -> Option<Image> {
        let depth_format = RenderPass::find_supported_format(context,
            &[
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
        ).expect("Failed to find an usable depth format");

        if !options.depth() {
            return None;
        }

        let mut depth_aspect_flags = vk::ImageAspectFlags::DEPTH;
        if depth_format == vk::Format::D32_SFLOAT_S8_UINT || depth_format == vk::Format::D24_UNORM_S8_UINT {
            depth_aspect_flags |= vk::ImageAspectFlags::STENCIL;
        }

        ImageCreateInfo::default()
            .aspect(depth_aspect_flags)
            .name(name)
            .image_type(vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D)
            .format(depth_format)
            .levels(0, 1)
            .layers(0, 1)
            .samples(options.multisampling())
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(sharing_mode)
            .extent(vk::Extent3D {
                width : extent.width,
                height : extent.height,
                depth : 1
            })
            .build(context)
            .into()
    }

    pub fn make_resolve_image<T : SwapchainOptions>(
        context : &RenderingContext,
        surface_format : vk::SurfaceFormatKHR,
        sharing_mode : vk::SharingMode,
        extent : vk::Extent2D,
        name : String,
        options : &T,
    ) -> Option<Image> {
        if options.multisampling() > vk::SampleCountFlags::TYPE_1 {
            ImageCreateInfo::default()
                .aspect(vk::ImageAspectFlags::COLOR)
                .name(name)
                .image_type(vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .levels(0, 1)
                .layers(0, 1)
                .samples(options.multisampling())
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .sharing_mode(sharing_mode)
                .extent(vk::Extent3D {
                    width : extent.width,
                    height : extent.height,
                    depth : 1
                })
                .build(context)
                .into()
        } else {
            None
        }
    }


    fn select_format<T : SwapchainOptions>(options : &T, formats : Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        for format in &formats {
            if options.select_surface_format(format) {
                return *format;
            }
        }

        formats[0]
    }

    fn get_extent<T : SwapchainOptions>(capabilities : vk::SurfaceCapabilitiesKHR, options : &T) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: options.width()
                    .clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: options.height()
                    .clamp(capabilities.max_image_extent.height, capabilities.min_image_extent.height),
            }
        }
    }

    pub fn color_format(&self) -> vk::Format { self.images[0].present.format() }

    pub fn create_render_pass(&self, is_presenting : bool) -> RenderPassCreateInfo {
        // TODO: Fix this for cases where multisampling is not active

        // Rely on the first image to deduce image formats for the render pass attachments.
        // What we do here doesn't really matter, we just need a way to get attachments and all
        // images should be in the same state at the point this function is called.
        let first_image = unsafe { self.images.get(0).unwrap_unchecked() };

        let color_format   = first_image.present.format();
        let depth_format   = match &first_image.depth {
            Some(depth) => depth.format(),
            None => unreachable!()
        };
        let resolve_format = match &first_image.resolve {
            Some(resolve) => resolve.format(),
            None => unreachable!()
        };

        let final_format = if is_presenting {
            vk::ImageLayout::PRESENT_SRC_KHR
        } else {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };

        RenderPassCreateInfo::default()
            .color_attachment(color_format, self.sample_count, vk::AttachmentLoadOp::CLEAR, vk::AttachmentStoreOp::STORE, vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .depth_attachment(depth_format, self.sample_count, vk::AttachmentLoadOp::CLEAR, vk::AttachmentStoreOp::STORE)
            .resolve_attachment(resolve_format, final_format)
    }

    /// Acquires the next image. Returns the image index, and wether the swapchain is suboptimal for the surface.
    pub(in crate) fn acquire_image(&self, semaphore : vk::Semaphore, fence : vk::Fence, timeout : u64) -> VkResult<(u32, bool)> {
        unsafe {
            self.loader.acquire_next_image(self.handle, timeout, semaphore, fence)
        }
    }

    pub fn format(&self) -> vk::Format { self.surface_format.format }
    pub fn color_space(&self) -> vk::ColorSpaceKHR { self.surface_format.color_space}
    pub fn layer_count(&self) -> u32 { self.layer_count }
    pub fn image_count(&self) -> usize { self.images.len() }
}

make_handle! { Swapchain, vk::SwapchainKHR }