use ash::vk;
use egui::epaint::ImageDelta;

use crate::vk::image::Image;

pub fn prepare_buffer_image_copy(image : &Image, mip_level : u32) -> vk::BufferImageCopy {
    vk::BufferImageCopy::default()
        .image_subresource(vk::ImageSubresourceLayers::default()
            .aspect_mask(image.aspect())
            .base_array_layer(image.base_array_layer())
            .layer_count(image.layer_count())
            .mip_level(mip_level))
}

pub fn with_delta(delta : &ImageDelta, copy : vk::BufferImageCopy) -> vk::BufferImageCopy {
    copy.buffer_offset(0)
        .buffer_image_height(delta.image.height() as u32)
        .buffer_row_length(delta.image.width() as u32)
        .image_offset(vk::Offset3D::default())
        .image_extent(vk::Extent3D {
            width : delta.image.width() as u32,
            height : delta.image.height() as u32,
            depth : 1,
        })
}