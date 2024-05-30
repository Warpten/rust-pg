use std::{ops::Range, sync::{Arc, Mutex}};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator};

use crate::{graph::buffer::Buffer, make_handle, vk::logical_device::LogicalDevice};

use super::{command_buffer::CommandBuffer, renderer::Renderer};

pub struct Image {
    device : Arc<LogicalDevice>,
    handle : vk::Image,
    allocation : Option<Allocation>,
    view : vk::ImageView,

    levels : Range<u32>,
    layers : Range<u32>,
    pub(in crate) layout : vk::ImageLayout,
    format : vk::Format,
    extent : vk::Extent3D,
    aspect : vk::ImageAspectFlags,
    sample_count : vk::SampleCountFlags,
}

pub struct ImageCreateInfo {
    aspect : vk::ImageAspectFlags,
    levels : [u32; 2],
    layers : [u32; 2],
    format : vk::Format,
    image_type : vk::ImageType,
    image_view_type : vk::ImageViewType,
    extent : vk::Extent3D,
    samples : vk::SampleCountFlags,
    tiling : vk::ImageTiling,
    usage : vk::ImageUsageFlags,
    sharing_mode : vk::SharingMode,
    name : String,
    initial_layout : vk::ImageLayout,
}

impl Default for ImageCreateInfo {
    fn default() -> Self {
        Self {
            aspect: vk::ImageAspectFlags::empty(),
            levels: [0, 1],
            layers: [0, 1],
            format: vk::Format::UNDEFINED,
            image_type: vk::ImageType::TYPE_2D,
            image_view_type: vk::ImageViewType::TYPE_2D,
            extent: vk::Extent3D::default(),
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::empty(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            name: "Unnamed image".to_owned(),
            initial_layout: vk::ImageLayout::UNDEFINED
        }
    }
}

impl ImageCreateInfo {
    value_builder! { aspect, vk::ImageAspectFlags }
    value_builder! { initial_layout, vk::ImageLayout }
    value_builder! { name, String }
    value_builder! { format, vk::Format }
    value_builder! { extent, vk::Extent3D }
    value_builder! { samples, vk::SampleCountFlags }
    value_builder! { tiling, vk::ImageTiling }
    value_builder! { usage, vk::ImageUsageFlags }
    value_builder! { sharing_mode, vk::SharingMode }
    
    #[inline] pub fn image_type(mut self, image_type : vk::ImageType, view_image_type : vk::ImageViewType) -> Self {
        self.image_type = image_type;
        self.image_view_type = view_image_type;
        self
    }

    #[inline] pub fn color(mut self) -> Self {
        self.aspect |= vk::ImageAspectFlags::COLOR;
        self
    }

    #[inline] pub fn depth(mut self) -> Self {
        self.aspect |= vk::ImageAspectFlags::DEPTH;
        self
    }

    #[inline] pub fn stencil(mut self) -> Self {
        self.aspect |= vk::ImageAspectFlags::STENCIL;
        self
    }

    #[inline] pub fn levels(mut self, base : u32, count : u32) -> Self {
        self.levels = [base, count];
        self
    }

    #[inline] pub fn layers(mut self, base : u32, count : u32) -> Self {
        self.layers = [base, count];
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> Image {
        unsafe {
            let image = vk::ImageCreateInfo::default()
                .image_type(self.image_type)
                .format(self.format)
                .extent(self.extent)
                .mip_levels(self.levels[1])
                .array_layers(self.layers[1])
                .samples(self.samples)
                .tiling(self.tiling)
                .usage(self.usage)
                .sharing_mode(self.sharing_mode);

            let image = device.handle()
                .create_image(&image, None)
                .expect("Image creation failed");

            let requirements = device.handle()
                .get_image_memory_requirements(image);

            let allocation = device.allocator()
                .lock()
                .expect("Failed to obtain allocator")
                .allocate(&AllocationCreateDesc {
                    name : format!("Allocation/{}", self.name).as_str(),
                    requirements,
                    location: gpu_allocator::MemoryLocation::GpuOnly,
                    linear: false,
                    // TODO: Figure this out
                    allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged
                })
                .expect("Memory allocation failed");

            device.handle()
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .expect("Memory binding failed");

            let image_view = vk::ImageViewCreateInfo::default()
                .format(self.format)
                .view_type(self.image_view_type)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(self.aspect)
                    .base_mip_level(self.levels[0])
                    .level_count(self.levels[1])
                    .base_array_layer(self.layers[0])
                    .layer_count(self.layers[1])
                );

            let image_view = device.handle()
                .create_image_view(&image_view, None)
                .expect("Image view creation failed");
            device.set_handle_name(image_view, &format!("View/{}", self.name));
            
            Image {
                device: device.clone(),
                handle: image,
                allocation : Some(allocation),
                view: image_view,
                levels: Range { start : self.levels[0], end : self.levels[0] + self.levels[1] },
                layers: Range { start : self.layers[0], end : self.layers[0] + self.layers[1] },
                layout: self.initial_layout,
                format: self.format,
                extent: self.extent,
                aspect: self.aspect,
                sample_count : self.samples,
            }
        }
    }
}

impl Image { // Construction
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
                    levels : Range { start : 0, end : 1 },
                    layers : Range { start : 0, end : 1 },
                    aspect : vk::ImageAspectFlags::COLOR,
                    sample_count : vk::SampleCountFlags::TYPE_1,
                }
            }
            
        }).collect::<Vec<_>>()
    }
}

impl Image { // Getters
    #[inline] pub fn logical_device(&self) -> &Arc<LogicalDevice> { &self.device }
    #[inline] pub fn allocator(&self) -> &Arc<Mutex<Allocator>> { self.logical_device().allocator() }

    #[inline] pub fn layout(&self) -> vk::ImageLayout { self.layout }
    #[inline] pub fn extent(&self) -> &vk::Extent3D { &self.extent }
    #[inline] pub fn view(&self) -> vk::ImageView { self.view }
    #[inline] pub fn format(&self) -> vk::Format { self.format }
    #[inline] pub fn aspect(&self) -> vk::ImageAspectFlags { self.aspect }
    #[inline] pub fn base_array_layer(&self) -> u32 { self.layers.start }
    #[inline] pub fn layer_count(&self) -> u32 { self.layers.end - self.layers.start }
    #[inline] pub fn base_mip_level(&self) -> u32 { self.levels.start }
    #[inline] pub fn level_count(&self) -> u32 { self.levels.end - self.levels.start }
    #[inline] pub fn sample_count(&self) -> vk::SampleCountFlags { self.sample_count }
    
    /// Returns a structure specifying access to one of this image's subresource layers.
    /// 
    /// # Arguments
    /// 
    /// * `mip_level` - The mimap level to identify.
    /// * `layers`- The base layer and the amount of layers to access. If this parameter is not specified, all available layers will be accessed.
    /// * `aspect_mask` - An optional parameter specifying which components of this image to access.
    pub fn make_subresource_layer(&self, mip_level : u32, layers : Option<Range<u32>>, aspect_mask : Option<vk::ImageAspectFlags>) -> vk::ImageSubresourceLayers {
        let base_array_layer = match layers {
            Some(ref layers) => u32::clamp(layers.start, self.layers.start, self.layers.end),
            None => self.layers.start,
        };

        let end_array_layer = match layers {
            Some(ref layers) => u32::clamp(layers.end, base_array_layer + 1, self.layers.end),
            None => self.layers.end
        };
        let layer_count = end_array_layer - base_array_layer;
        
        assert_ne!(layer_count, 0, "Impossible layers requested or invalid image setup");

        vk::ImageSubresourceLayers::default()
            .aspect_mask(self.aspect)
            .mip_level(mip_level.clamp(self.base_mip_level(), self.base_mip_level() + self.level_count()))
            .base_array_layer(base_array_layer)
            .layer_count(layer_count)
    }
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

    pub fn from_buffer(&self, renderer : &Renderer, buffer : Buffer, cb : impl FnOnce(&CommandBuffer)) {
        let cmd = CommandBuffer::builder()
            .pool(&renderer.transfer_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .build_one(&renderer.device);

        {
            let image_memory_barrier = vk::ImageMemoryBarrier::default()
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_access_mask(vk::AccessFlags::NONE_KHR)
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(self.layers.start)
                    .layer_count(self.layer_count())
                    .base_mip_level(self.levels.start)
                    .level_count(self.level_count())
                );

            cmd.pipeline_barrier(
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::BY_REGION,
                &[], &[], &[image_memory_barrier]
            );
        }

        cb(&cmd);

        {
            let image_memory_barrier = vk::ImageMemoryBarrier::default()
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(self.layers.start)
                    .layer_count(self.layer_count())
                    .base_mip_level(self.levels.start)
                    .level_count(self.level_count())
                );

            cmd.pipeline_barrier(
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_SHADER,
                vk::DependencyFlags::BY_REGION,
                &[], &[], &[image_memory_barrier]
            );
        }
    }
    
    /// Records a layout transition for this image.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command buffer on which the command will be recorded.
    /// * `from` - The old layout.
    /// * `to` - The new layout.
    /// * `mip_levels` - The mipmap levels that should be transitioned.
    pub fn layout_transition(&self, cmd : &CommandBuffer, from : vk::ImageLayout, to : vk::ImageLayout, flags : vk::DependencyFlags) {
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
                .base_array_layer(self.layers.start)
                .layer_count(self.layer_count())
                .base_mip_level(self.levels.start)
                .level_count(self.level_count()));

        unsafe {
            cmd.pipeline_barrier(src_stage,
                dst_stage,
                flags,
                &[],
                &[],
                &[barrier]);
        }
    }
}

make_handle! { Image, vk::Image }

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