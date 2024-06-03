use std::{borrow::Borrow, ops::Range, sync::Arc};

use ash::vk;
use ash::prelude::VkResult;

use crate::make_handle;
use crate::traits::handle::Handle;
use crate::vk::context::Context;
use crate::vk::image::Image;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::queue::QueueFamily;
use crate::vk::render_pass::RenderPass;
use crate::vk::surface::Surface;

use super::{image::ImageCreateInfo, render_pass::RenderPassCreateInfo, renderer::RendererOptions};

pub struct SwapchainImage {
    pub present : Image,
    pub depth : Option<Image>,
    pub resolve : Option<Image>,
}

pub struct Swapchain {
    device : Arc<LogicalDevice>,
    pub surface : Arc<Surface>,

    handle : vk::SwapchainKHR,
    pub loader : ash::khr::swapchain::Device,
    
    pub extent : vk::Extent2D,
    pub images : Vec<SwapchainImage>,
    // pub present_images : Vec<Image>,
    // pub depth_images   : Vec<Image>,
    // pub resolve_images : Vec<Image>,
    pub sample_count   : vk::SampleCountFlags,

    pub queue_families : Vec<QueueFamily>,
    layer_count : u32,

    pub surface_format : vk::SurfaceFormatKHR,
}

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

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

impl Swapchain {
    #[inline] pub fn device(&self) -> &Arc<LogicalDevice> { &self.device }

    /// Creates a new swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The global Vulkan [`Instance`].
    /// * `device` - The [`LogicalDevice`] for which to create a swapchain.
    /// * `surface` - The [`Surface`] for which to create a swapchain.
    /// * `options` - An implementation of the [`SwapchainOptions`] trait that defines how the swapchain should be created.
    /// * `queue_families` - All queue families that will have access to the swapchain's images.
    /// 
    /// # Panics
    ///
    /// * Panics if [`vkGetPhysicalDeviceSurfaceFormats`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetPhysicalDeviceSurfaceFormatsKHR.html) fails.
    /// * Panics if [`vkGetPhysicalDeviceSurfaceCapabilities`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetPhysicalDeviceSurfaceCapabilitiesKHR.html) fails.
    /// * Panics if [`vkCreateSwapchainKHR`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateSwapchainKHR.html) fails.
    /// * Panics if [`vkGetSwapchainImagesKHR`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetSwapchainImagesKHR.html) fails.
    pub fn new<T : SwapchainOptions>(
        instance : &Arc<Context>,
        device : &Arc<LogicalDevice>,
        surface : &Arc<Surface>,
        options : &T,
        queue_families : Vec<QueueFamily>,
    ) -> Arc<Self> {
        let surface_format = {
            let surface_formats = unsafe {
                surface.loader
                    .get_physical_device_surface_formats(device.physical_device.handle(), surface.handle())
                    .expect("Failed to get physical device surface formats")
            };

            surface_formats.iter()
                .find(|&v| options.select_surface_format(v))
                .unwrap_or(&surface_formats[0])
                .clone()
        };

        let surface_capabilities = unsafe {
            surface.loader
                .get_physical_device_surface_capabilities(device.physical_device.handle(), surface.handle())
                .expect("Failed to get physical device surface capabilities")
        };
        let extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            vk::Extent2D {
                width: options.width()
                    .clamp(surface_capabilities.min_image_extent.width, surface_capabilities.max_image_extent.width),
                height: options.height()
                    .clamp(surface_capabilities.max_image_extent.height, surface_capabilities.min_image_extent.height),
            }
        };

        let image_count = surface_capabilities.min_image_count + 1;
        let image_count = if surface_capabilities.max_image_count != 0 {
            image_count.min(surface_capabilities.max_image_count)
        } else {
            image_count
        };

        let present_modes = unsafe {
            surface.loader.get_physical_device_surface_present_modes(device.physical_device.handle(), surface.handle())
                .expect("Failed to get physical device surface present modes")
        };

        let queue_family_indices = queue_families.iter().map(QueueFamily::index).collect::<Vec<_>>();

        let sharing_mode = if queue_family_indices.len() == 1 {
            vk::SharingMode::EXCLUSIVE
        } else {
            vk::SharingMode::CONCURRENT
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            // Number of views in a multiview/stereo surface. For non-stereoscopic-3D applications, this value is 1.
            .image_array_layers(1)
            // A bitmask of VkImageUsageFlagBits describing the intended usage of the (acquired) swapchain images.
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
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

        let swapchain_loader = ash::khr::swapchain::Device::new(instance.handle(), device.handle());
        let handle = unsafe {
            swapchain_loader.create_swapchain(&swapchain_create_info, None)
                .expect(format!("Failed to create swapchain with options {:?}", swapchain_create_info).borrow())
        };

        let present_images = unsafe {
            let swapchain_images = swapchain_loader
                .get_swapchain_images(handle)
                .expect("Failed to get swapchain images");

            Image::from_swapchain(&extent, &device, surface_format.format, swapchain_images)
        };

        let mut images = vec![];
        for (i, present) in present_images.into_iter().enumerate() {
            let depth = Self::make_depth_image(device,
                format!("Swapchain/Depth[{}]", i), options, sharing_mode, extent);
            let resolve = Self::make_resolve_image(device,
                format!("Swapchain/Resolve[{}]", i), options, surface_format, sharing_mode, extent);

            images.push(SwapchainImage {
                present,
                depth,
                resolve
            })
        }

        Arc::new(Self {
            device : device.clone(),
            extent,
            handle,
            surface : surface.clone(),
            layer_count : options.layers().len() as _,
            loader : swapchain_loader,
            images,
            queue_families,
            sample_count : options.multisampling(),

            surface_format
        })
    }

    fn make_depth_image<T : SwapchainOptions>(
        device : &Arc<LogicalDevice>,
        name : String,
        options : &T,
        image_sharing_mode : vk::SharingMode,
        extent : vk::Extent2D
    ) -> Option<Image> {
        let depth_format = RenderPass::find_supported_format(device,
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
            .sharing_mode(image_sharing_mode)
            .extent(vk::Extent3D {
                width : extent.width,
                height : extent.height,
                depth : 1
            })
            .build(device)
            .into()
    }

    pub fn make_resolve_image<T : SwapchainOptions>(
        device : &Arc<LogicalDevice>,
        name : String,
        options : &T,
        surface_format : vk::SurfaceFormatKHR,
        sharing_mode : vk::SharingMode,
        extent : vk::Extent2D
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
                .build(device)
                .into()
        } else {
            None
        }
    }

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