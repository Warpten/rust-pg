use std::sync::Arc;

use super::{Instance, LogicalDevice, QueueFamily, Surface};

pub struct Swapchain {
    pub handle : ash::vk::SwapchainKHR,
    pub device : Arc<LogicalDevice>,
    pub surface : Arc<Surface>,
    pub loader : ash::khr::swapchain::Device,
    pub extent : ash::vk::Extent2D,
    pub images : Vec<ash::vk::Image>,
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
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

impl Swapchain {
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Creates a new swapchain.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The global Vulkan instance.
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
                    .get_physical_device_surface_formats(device.physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface formats")
            };

            surface_formats.iter()
                .find(|&v| options.select_surface_format(v))
                .unwrap_or(&surface_formats[0])
                .clone()
        };

        let surface_capabilities = unsafe {
            surface.loader
                .get_physical_device_surface_capabilities(device.physical_device.handle, surface.handle)
                .expect("Failed to get physical device surface capabilities")
        };
        let surface_extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            ash::vk::Extent2D {
                width: options.width()
                    .max(surface_capabilities.min_image_extent.width)
                    .min(surface_capabilities.max_image_extent.width),
                height: options.height()
                    .max(surface_capabilities.min_image_extent.height)
                    .min(surface_capabilities.max_image_extent.height),
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
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_extent)
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(options.composite_alpha())
            .present_mode(ash::vk::PresentModeKHR::FIFO)
            .clipped(true);

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance.handle, &device.handle);
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

        Arc::new(Self {
            device : device,
            extent : surface_extent,
            handle,
            surface,
            loader : swapchain_loader,
            images : swapchain_images,
            queue_families : options.queue_families().to_vec()
        })
    }
}