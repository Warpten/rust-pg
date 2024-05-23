use std::sync::{Arc, Mutex};

use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator};

use crate::{traits::handle::BorrowHandle, vk::LogicalDevice};

pub struct Image {
    device : Arc<LogicalDevice>,
    handle : ash::vk::Image,
    allocation : Option<Allocation>,
    view : ash::vk::ImageView,

    levels : u32,
    layers : u32,
    layout : ash::vk::ImageLayout,
    format : ash::vk::Format,
    extent : ash::vk::Extent3D,
}

impl Image { // Construction
    /// Creates a new image.
    ///
    /// # Arguments
    ///
    /// * `device` - The logical device owning this image.
    /// * `allocator` - A GPU allocator.
    /// * `create_info` - Describes the format and type of the texel blocks that will be contained in the image.
    /// * `aspect_mask` - Number of data elements in each dimension of the image.
    /// * `levels` - Number of levels of detail available for minified sampling of the image.
    /// * `layers` - 
    pub fn new(
        name : &'static str,
        device : &Arc<LogicalDevice>,
        create_info : ash::vk::ImageCreateInfo,
        aspect_mask : ash::vk::ImageAspectFlags,
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
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                // TODO: Figure this out
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
            })
            .expect("Memory allocation failed");

        unsafe { device.handle().bind_image_memory(image, allocation.memory(), allocation.offset()).expect("Memory binding failed") };

        let image_view_info = ash::vk::ImageViewCreateInfo::default()
            .view_type(ash::vk::ImageViewType::TYPE_2D)
            .subresource_range(ash::vk::ImageSubresourceRange::default()
                .aspect_mask(aspect_mask)
                .level_count(create_info.mip_levels)
                .layer_count(create_info.array_layers))
            .image(image)
            .format(create_info.format);

        let image_view = unsafe {
            device.handle().create_image_view(&image_view_info, None)
                .expect("Creating an image view failed")
        };

        Self {
            device: device.clone(),
            allocation: Some(allocation),
            handle: image,
            layout: create_info.initial_layout,
            format: create_info.format,
            view: image_view,
            extent: create_info.extent,
            levels : create_info.mip_levels,
            layers : create_info.array_layers,
        }
    }

    pub fn from_swapchain(extent: &ash::vk::Extent2D, device: &Arc<LogicalDevice>, format: ash::vk::Format, images: Vec<ash::vk::Image>) -> Vec<Image> {
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
                    device: device.clone(),
                    handle: image,
                    extent: ash::vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1
                    },
                    allocation: None,
                    view: image_view,
                    format,
                    layout: ash::vk::ImageLayout::UNDEFINED,
                    levels : 1,
                    layers : 1,
                }
            }
        }).collect::<Vec<_>>()
    }
}

impl Image { // Getters
    #[inline]
    pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.device }
    #[inline]
    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { self.logical_device().allocator() }

    #[inline]
    pub fn layout(&self) -> ash::vk::ImageLayout { self.layout }
    #[inline]
    pub fn extent(&self) -> &ash::vk::Extent3D { &self.extent }

    pub fn view(&self) -> ash::vk::ImageView { self.view }

    pub fn format(&self) -> ash::vk::Format { self.format }
}

impl Image { // Utilities
    pub fn derive_aspect_flags(layout : ash::vk::ImageLayout, format : ash::vk::Format) -> ash::vk::ImageAspectFlags {
        let mut aspect_flags = ash::vk::ImageAspectFlags::COLOR;
        if layout == ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            aspect_flags = ash::vk::ImageAspectFlags::DEPTH;
            match format {
                ash::vk::Format::D32_SFLOAT_S8_UINT => aspect_flags |= ash::vk::ImageAspectFlags::STENCIL,
                ash::vk::Format::D24_UNORM_S8_UINT => aspect_flags |= ash::vk::ImageAspectFlags::STENCIL,
                _ => ()
            }
        }
        aspect_flags
    }
    
    /// Records a layout transition for this image.
    ///
    /// # Arguments
    ///
    /// * `command_buffer` - The command buffer on which the command will be recorded.
    /// * `from` - The old layout.
    /// * `to` - The new layout.
    /// * `mip_levels` - The mipmap levels that should be transitioned.
    pub fn layout_transition(&self, command_buffer : ash::vk::CommandBuffer, from : ash::vk::ImageLayout, to : ash::vk::ImageLayout) {
        let aspect_flags = Image::derive_aspect_flags(to, self.format);

        let src_access_mask = match from {
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL => ash::vk::AccessFlags::TRANSFER_WRITE,
            ash::vk::ImageLayout::PREINITIALIZED => ash::vk::AccessFlags::HOST_WRITE,
            ash::vk::ImageLayout::GENERAL => ash::vk::AccessFlags::MEMORY_WRITE | ash::vk::AccessFlags::SHADER_WRITE,
            _ => ash::vk::AccessFlags::default(),
        };

        let dst_access_mask = match to {
            ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                => ash::vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | ash::vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ash::vk::ImageLayout::GENERAL => ash::vk::AccessFlags::empty(),
            ash::vk::ImageLayout::PRESENT_SRC_KHR => ash::vk::AccessFlags::empty(),
            ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => ash::vk::AccessFlags::SHADER_READ,
            ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL => ash::vk::AccessFlags::TRANSFER_READ,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL => ash::vk::AccessFlags::TRANSFER_WRITE,
            _ => panic!("Incomprehensible layout transition"),
        };

        let src_stage = match from {
            ash::vk::ImageLayout::GENERAL => ash::vk::PipelineStageFlags::ALL_COMMANDS,
            ash::vk::ImageLayout::PREINITIALIZED => ash::vk::PipelineStageFlags::HOST,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL => ash::vk::PipelineStageFlags::TRANSFER,
            ash::vk::ImageLayout::UNDEFINED => ash::vk::PipelineStageFlags::TOP_OF_PIPE,
            _ => ash::vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let dst_stage = match to {
            ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => ash::vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ash::vk::ImageLayout::GENERAL => ash::vk::PipelineStageFlags::HOST,
            ash::vk::ImageLayout::PRESENT_SRC_KHR => ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => ash::vk::PipelineStageFlags::FRAGMENT_SHADER,
            ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL => ash::vk::PipelineStageFlags::TRANSFER,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL => ash::vk::PipelineStageFlags::TRANSFER,
            _ => ash::vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let barrier = ash::vk::ImageMemoryBarrier::default()
            .image(self.handle)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .new_layout(to)
            .old_layout(from)
            .subresource_range(ash::vk::ImageSubresourceRange::default()
                .aspect_mask(aspect_flags)
                .layer_count(self.layers)
                .level_count(self.levels));

        unsafe {
            self.device.handle().cmd_pipeline_barrier(command_buffer,
                src_stage,
                dst_stage,
                ash::vk::DependencyFlags::empty(), // No idea
                &[],
                &[],
                &[barrier]);
        }
    }
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