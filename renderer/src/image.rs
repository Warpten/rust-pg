use std::sync::{Arc, Mutex};

use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator};

use crate::{traits::BorrowHandle, LogicalDevice};

pub struct Image {
    device : Arc<LogicalDevice>,
    handle : ash::vk::Image,
    allocation : Option<Allocation>,
    view : ash::vk::ImageView,

    layout : ash::vk::ImageLayout,
    format : ash::vk::Format,
    extent : ash::vk::Extent3D,
}

impl Image {
    #[inline] pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.device }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { self.logical_device().allocator() }

    /// Creates a new image.
    /// 
    /// # Arguments
    /// 
    /// * `device` - The logical device owning this image.
    /// * `allocator` - A GPU allocator.
    /// * `create_info` - Describes the format and type of the texel blocks that will be contained in the image.
    /// * `aspect_mask` - Number of data elements in each dimension of the image.
    /// * `levels` - Number of levels of detail available for minified sampling of the image.
    pub fn new(
        name : &'static str,
        device : &Arc<LogicalDevice>,
        create_info : ash::vk::ImageCreateInfo,
        aspect_mask : ash::vk::ImageAspectFlags,
        levels : u32,
    ) -> Self
    {
        let image = unsafe {
            device.handle().create_image(&create_info, None)
                .expect("Creating the image failed")
        };

        let requirements = unsafe { device.handle().get_image_memory_requirements(image) };

        let allocation = device.allocator()
            .lock()
            .expect("Failed to obtain allocator")
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location : gpu_allocator::MemoryLocation::GpuOnly,
                linear : false,
                // TODO: Figure this out
                allocation_scheme : gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
            })
            .expect("Memory allocation failed");

        unsafe { device.handle().bind_image_memory(image, allocation.memory(), allocation.offset()).expect("Memory binding failed") };

        let image_view_info = ash::vk::ImageViewCreateInfo::default()
            .view_type(ash::vk::ImageViewType::TYPE_2D)
            .subresource_range(ash::vk::ImageSubresourceRange::default()
                .aspect_mask(aspect_mask)
                .level_count(levels)
                .layer_count(1))
            .image(image)
            .format(create_info.format);

        let image_view = unsafe {
            device.handle().create_image_view(&image_view_info, None)
                .expect("Creating an image view failed")
        };

        Self {
            device : device.clone(),
            allocation : Some(allocation),
            handle : image,
            layout : create_info.initial_layout,
            format : create_info.format,
            view : image_view,
            extent : create_info.extent
        }
    }

    pub fn from_swapchain(extent : &ash::vk::Extent2D, device : &Arc<LogicalDevice>, format : ash::vk::Format, images : Vec<ash::vk::Image>) -> Vec<Image> {
        images.iter().map(|&image| {
            unsafe {
                let image_view_info = ash::vk::ImageViewCreateInfo::default()
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::R,
                        g: ash::vk::ComponentSwizzle::G,
                        b: ash::vk::ComponentSwizzle::B,
                        a: ash::vk::ComponentSwizzle::A,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);

                let image_view = device
                    .handle()
                    .create_image_view(&image_view_info, None)
                    .expect("Failed to create an image view on the swapchain image");

                Self {
                    device : device.clone(),
                    handle : image,
                    extent : ash::vk::Extent3D {
                        width : extent.width,
                        height : extent.height,
                        depth : 1
                    },
                    allocation : None,
                    view   : image_view,
                    format,
                    layout : ash::vk::ImageLayout::UNDEFINED
                }
            }
        }).collect::<Vec<_>>()
    }

    pub fn view(&self) -> ash::vk::ImageView { self.view }

    pub fn format(&self) -> ash::vk::Format { self.format }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_image_view(self.view, None);
            if self.allocation.is_some() {
                self.device.handle().destroy_image(self.handle, None);

                self.device.allocator()
                    .lock()
                    .unwrap()
                    .free(self.allocation.take().unwrap_unchecked())
                    .expect("Failed to free memory");
            }
        }
    }
}