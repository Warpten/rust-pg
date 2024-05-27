use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator};

use crate::vk::logical_device::LogicalDevice;

pub struct Image {
    device : Arc<LogicalDevice>,
    handle : vk::Image,
    allocation : Option<Allocation>,
    view : vk::ImageView,

    levels : u32,
    layers : u32,
    layout : vk::ImageLayout,
    format : vk::Format,
    extent : vk::Extent3D,
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
        name : String,
        device : &Arc<LogicalDevice>,
        create_info : vk::ImageCreateInfo,
        aspect_mask : vk::ImageAspectFlags,
    ) -> Self
    {
        let image = unsafe {
            device.handle().create_image(&create_info, None)
                .expect("Creating the image failed")
        };
        device.set_handle_name(image, &name);

        let requirements = unsafe { device.handle().get_image_memory_requirements(image) };

        let allocation = device.allocator()
            .lock()
            .expect("Failed to obtain allocator")
            .allocate(&AllocationCreateDesc {
                name : name.as_str(),
                requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                // TODO: Figure this out
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
            })
            .expect("Memory allocation failed");

        unsafe { device.handle().bind_image_memory(image, allocation.memory(), allocation.offset()).expect("Memory binding failed") };

        let image_view_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .subresource_range(vk::ImageSubresourceRange::default()
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

    pub fn from_swapchain(extent: &vk::Extent2D, device: &Arc<LogicalDevice>, format: vk::Format, images: Vec<vk::Image>) -> Vec<Image> {
        let mut index = 0;
        images.iter().map(|&image| {
            device.set_handle_name(image, &format!("Swapchain/Image #{}", index));

            unsafe {
                let image_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
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

                device.set_handle_name(image_view, &format!("Swapchain/ImageView #{}", index));

                index += 1;

                Self {
                    device: device.clone(),
                    handle: image,
                    extent: vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1
                    },
                    allocation: None,
                    view: image_view,
                    format,
                    layout: vk::ImageLayout::UNDEFINED,
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
    pub fn layout(&self) -> vk::ImageLayout { self.layout }
    #[inline]
    pub fn extent(&self) -> &vk::Extent3D { &self.extent }

    pub fn view(&self) -> vk::ImageView { self.view }

    pub fn format(&self) -> vk::Format { self.format }
}

impl Image { // Utilities
    pub fn derive_aspect_flags(layout : vk::ImageLayout, format : vk::Format) -> vk::ImageAspectFlags {
        let mut aspect_flags = vk::ImageAspectFlags::COLOR;
        if layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            aspect_flags = vk::ImageAspectFlags::DEPTH;
            match format {
                vk::Format::D32_SFLOAT_S8_UINT => aspect_flags |= vk::ImageAspectFlags::STENCIL,
                vk::Format::D24_UNORM_S8_UINT => aspect_flags |= vk::ImageAspectFlags::STENCIL,
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
    pub fn layout_transition(&self, command_buffer : vk::CommandBuffer, from : vk::ImageLayout, to : vk::ImageLayout) {
        let aspect_flags = Image::derive_aspect_flags(to, self.format);

        let src_access_mask = match from {
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::AccessFlags::TRANSFER_WRITE,
            vk::ImageLayout::PREINITIALIZED => vk::AccessFlags::HOST_WRITE,
            vk::ImageLayout::GENERAL => vk::AccessFlags::MEMORY_WRITE | vk::AccessFlags::SHADER_WRITE,
            _ => vk::AccessFlags::default(),
        };

        let dst_access_mask = match to {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                => vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::ImageLayout::GENERAL => vk::AccessFlags::empty(),
            vk::ImageLayout::PRESENT_SRC_KHR => vk::AccessFlags::empty(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::AccessFlags::SHADER_READ,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => vk::AccessFlags::TRANSFER_READ,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::AccessFlags::TRANSFER_WRITE,
            _ => panic!("Incomprehensible layout transition"),
        };

        let src_stage = match from {
            vk::ImageLayout::GENERAL => vk::PipelineStageFlags::ALL_COMMANDS,
            vk::ImageLayout::PREINITIALIZED => vk::PipelineStageFlags::HOST,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::PipelineStageFlags::TRANSFER,
            vk::ImageLayout::UNDEFINED => vk::PipelineStageFlags::TOP_OF_PIPE,
            _ => vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let dst_stage = match to {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            vk::ImageLayout::GENERAL => vk::PipelineStageFlags::HOST,
            vk::ImageLayout::PRESENT_SRC_KHR => vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => vk::PipelineStageFlags::TRANSFER,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::PipelineStageFlags::TRANSFER,
            _ => vk::PipelineStageFlags::ALL_COMMANDS,
        };

        let barrier = vk::ImageMemoryBarrier::default()
            .image(self.handle)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .new_layout(to)
            .old_layout(from)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(aspect_flags)
                .layer_count(self.layers)
                .level_count(self.levels));

        unsafe {
            self.device.handle().cmd_pipeline_barrier(command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(), // No idea
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