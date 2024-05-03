use std::{ops::Range, sync::Arc};

use crate::traits::{BorrowHandle, Handle};

use super::{Instance, LogicalDevice, QueueFamily, Surface};

pub struct Swapchain {
    handle : ash::vk::SwapchainKHR,
    pub device : Arc<LogicalDevice>,
    pub surface : Arc<Surface>,
    pub loader : ash::khr::swapchain::Device,
    pub extent : ash::vk::Extent2D,
    pub images : Vec<ash::vk::Image>,
    pub image_views : Vec<ash::vk::ImageView>,
    layer_count : u32,
    pub queue_families : Vec<QueueFamily>,
}

/// Options that are used when creating a [`Swapchain`].
pub trait SwapchainOptions {
    /// Determines if the provided surface_format is eligible for the swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `format` - The format to test.
    /// 
    /// # Returns
    /// 
    /// This function should return `true` in one exact case; if it doesn't, whatever format is tested
    /// `true` first will be selected.
    fn select_surface_format(&self, format : &ash::vk::SurfaceFormatKHR) -> bool;

    /// Returns the [`QueueFamily`]ies that will have access to the swapchain's images.
    fn queue_families(&self) -> Vec<QueueFamily>;

    /// Returns the width of the swapchain's images.
    fn width(&self) -> u32;

    /// Returns the height of the swapchain's images.
    fn height(&self) -> u32;

    /// Returns the composite flags to be used by the swapchain's images.
    fn composite_alpha(&self) -> ash::vk::CompositeAlphaFlagsKHR;

    /// Returns the presentation mode of this swapchain.
    fn present_mode(&self) -> ash::vk::PresentModeKHR;

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
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.image_views.into_iter().for_each(|view| {
                self.device.handle().destroy_image_view(view, None);
            });

            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

impl Swapchain {
    /// Creates a new swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The global Vulkan [`Instance`].
    /// * `device` - The [`LogicalDevice`] for which to create a swapchain.
    /// * `surface` - The [`Surface`] for which to create a swapchain.
    /// * `options` - An implementation of the [`SwapchainOptions`] trait that defines how the swapchain should be created.
    /// 
    /// # Panics
    ///
    /// * Panics if [`vkGetPhysicalDeviceSurfaceFormats`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetPhysicalDeviceSurfaceFormatsKHR.html) fails.
    /// * Panics if [`vkGetPhysicalDeviceSurfaceCapabilities`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetPhysicalDeviceSurfaceCapabilitiesKHR.html) fails.
    /// * Panics if [`vkCreateSwapchainKHR`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateSwapchainKHR.html) fails.
    /// * Panics if [`vkGetSwapchainImagesKHR`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkGetSwapchainImagesKHR.html) fails.
    pub fn new(
        instance : Arc<Instance>,
        device : Arc<LogicalDevice>,
        surface : Arc<Surface>,
        options : impl SwapchainOptions,
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
        let surface_extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            ash::vk::Extent2D {
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

        let image_sharing_mode = ash::vk::SharingMode::EXCLUSIVE;
        let queue_family_indices = options.queue_families().iter()
            .map(|q| q.index as u32)
            .collect::<Vec<_>>();

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_extent)
            // Number of views in a multiview/stereo surface. For non-stereoscopic-3D applications, this value is 1.
            .image_array_layers(1)
            // A bitmask of VkImageUsageFlagBits describing the intended usage of the (acquired) swapchain images.
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(surface_capabilities.current_transform)
            // Indicates the alpha compositing mode to use when this surface is composited together with other
            // surfaces on certain window systems.
            .composite_alpha(options.composite_alpha())
            // Presentation mode the swapchain will use. A swapchain’s present mode determines how incoming present
            // requests will be processed and queued internally.
            .present_mode(ash::vk::PresentModeKHR::FIFO)
            // Specifies whether the Vulkan implementation is allowed to discard rendering operations that affect
            // regions of the surface that are not visible.
            .clipped(true);

        let swapchain_loader = ash::khr::swapchain::Device::new(instance.handle(), device.handle());
        let handle = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(handle)
                .expect("Failed to get swapchain images")
        };

        let swapchain_image_views = swapchain_images.iter().map(|&image| {
            let image_view_create_info = ash::vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(ash::vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(ash::vk::ComponentMapping::default()
                    .a(ash::vk::ComponentSwizzle::IDENTITY)
                    .r(ash::vk::ComponentSwizzle::IDENTITY)
                    .g(ash::vk::ComponentSwizzle::IDENTITY)
                    .b(ash::vk::ComponentSwizzle::IDENTITY))
                .subresource_range(ash::vk::ImageSubresourceRange::default()
                    // Aspects of the image that will be included in the view.
                    .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                    // The first mipmap level accessible to the view.
                    .base_mip_level(options.mip_range().start)
                    // The number of mipmap levels accessible to the view.
                    .level_count(options.mip_range().len() as _)
                    // The first array layer accessible to the view.
                    .base_array_layer(options.layers().start)
                    // The number of array layers (starting from base_array_layer) accessible to the view.
                    .layer_count(options.layers().len() as _));

            unsafe {   
                device.handle().create_image_view(&image_view_create_info, None)
                    .expect("Failed creating an image view to swapchain image")
            }
        }).collect::<Vec<_>>();

        Arc::new(Self {
            device : device,
            extent : surface_extent,
            handle,
            surface,
            layer_count : options.layers().len() as _,
            loader : swapchain_loader,
            images : swapchain_images,
            image_views : swapchain_image_views,
            queue_families : options.queue_families().to_vec()
        })
    }

    pub fn layer_count(&self) -> u32 { self.layer_count }

    pub fn image_count(&self) -> usize { self.images.len() }

    pub fn images<'a>(&'a self) -> impl Iterator<Item = (ash::vk::Image, ash::vk::ImageView)> + 'a {
        self.images.iter().cloned().zip(self.image_views.iter().cloned())
    }
}