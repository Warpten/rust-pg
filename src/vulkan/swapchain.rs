use super::{Instance, LogicalDevice, Queue, QueueFamily, Surface};

pub struct Swapchain<'instance, 'surface : 'device, 'device : 'instance> {
    pub handle : ash::vk::SwapchainKHR,
    pub device : &'device LogicalDevice<'device, 'instance>,
    pub surface : &'surface Surface<'instance>,
    pub loader : ash::khr::swapchain::Device,
    pub extent : ash::vk::Extent2D,
    pub images : Vec<ash::vk::Image>,
}

impl Drop for Swapchain<'_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

impl Swapchain<'_, '_, '_> {
    pub fn new<'device, 'instance, 'surface>(
        instance : &Instance,
        device : &LogicalDevice<'device, 'instance>,
        surface : &'surface Surface<'instance>,
        queue_family : &QueueFamily,
        width : u32,
        height : u32
    ) -> Self {
        let surface_format = {
            let surface_formats = unsafe {
                surface.loader
                    .get_physical_device_surface_formats(device.physical_device.handle, surface.handle)
                    .expect("Failed to get physical device surface formats")
            };

            surface_formats.iter()
                .find(|format| {
                    format.format == ash::vk::Format::B8G8R8A8_UNORM
                        || format.format == ash::vk::Format::R8G8B8A8_UNORM
                })
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
                width: width
                    .max(surface_capabilities.min_image_extent.width)
                    .min(surface_capabilities.max_image_extent.width),
                height: height
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
        let queue_family_indices = [queue_family.index as u32];

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
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
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

        Self {
            device : device,
            extent : surface_extent,
            handle,
            surface,
            loader : swapchain_loader,
            images : swapchain_images
        }
    }
}