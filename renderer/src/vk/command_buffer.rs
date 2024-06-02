use std::{ffi::CString, sync::Arc};

use ash::vk::{self, ClearValue};

use crate::traits::handle::Handle;
use crate::vk::buffer::Buffer;
use crate::vk::command_pool::CommandPool;
use crate::vk::framebuffer::Framebuffer;
use crate::vk::image::Image;
use crate::vk::logical_device::LogicalDevice;
use crate::vk::pipeline::Pipeline;
use crate::vk::render_pass::RenderPass;

pub struct CommandBuffer {
    device : Arc<LogicalDevice>,
    handle : vk::CommandBuffer,
    level : vk::CommandBufferLevel,
}

impl CommandBuffer {
    pub fn builder() -> CommandBufferBuilder {
        CommandBufferBuilder { pool : vk::CommandPool::null(), level : vk::CommandBufferLevel::PRIMARY }
    }

    pub fn pipeline_barrier(&self,
        src_stage_mask : vk::PipelineStageFlags,
        dst_stage_mask : vk::PipelineStageFlags,
        dependency_flags : vk::DependencyFlags,
        memory_barriers : &[vk::MemoryBarrier],
        buffer_memory_barriers : &[vk::BufferMemoryBarrier],
        image_memory_barriers : &[vk::ImageMemoryBarrier]
    ) {
        unsafe {
            self.device.handle()
                .cmd_pipeline_barrier(self.handle,
                    src_stage_mask,
                    dst_stage_mask,
                    dependency_flags, memory_barriers, buffer_memory_barriers, image_memory_barriers
                );
        }
    }

    pub fn begin_label(&self, label : String, color : [f32; 4]) {
        unsafe {
            if let Some(debug_utils) = &self.device.debug_utils {
                let name = CString::new(label).unwrap();

                let info = vk::DebugUtilsLabelEXT::default()
                    .label_name(name.as_c_str())
                    .color(color);

                debug_utils.cmd_begin_debug_utils_label(self.handle, &info);
            }
        }
    }

    pub fn end_label(&self) {
        unsafe {
            if let Some(debug_utils) = &self.device.debug_utils {
                debug_utils.cmd_end_debug_utils_label(self.handle);
            }
        }
    }

    pub fn label(&self, label : String, color : [f32; 4], cb : impl Fn()) {
        unsafe {
            if let Some(debug_utils) = &self.device.debug_utils {
                let name = CString::new(label).unwrap();

                let info = vk::DebugUtilsLabelEXT::default()
                    .label_name(name.as_c_str())
                    .color(color);

                debug_utils.cmd_begin_debug_utils_label(self.handle, &info);
                cb();
                debug_utils.cmd_end_debug_utils_label(self.handle);
            } else {
                cb();
            }
        }
    }

    pub fn insert_label(&self, label : &'static str, color : [f32; 4]) {
        unsafe {
            if let Some(debug_utils) = &self.device.debug_utils {
                let name = CString::new(label).unwrap();

                let info = vk::DebugUtilsLabelEXT::default()
                    .label_name(name.as_c_str())
                    .color(color);

                debug_utils.cmd_insert_debug_utils_label(self.handle, &info);
            }
        }
    }

    /// Begins recording this command buffer.
    pub fn begin(&self, flags : vk::CommandBufferUsageFlags) {
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(flags);

            self.device.handle()
                .begin_command_buffer(self.handle, &begin_info)
                .expect("Failed to begin recording the command buffer.");
        }
    }

    pub fn image_memory_barrier(&self,
        image : &mut Image,
        src : BarrierPhase,
        dst : BarrierPhase,
        dependency : vk::DependencyFlags,
        new_layout : vk::ImageLayout
    ) {
        let barrier = vk::ImageMemoryBarrier::default()
            .dst_access_mask(dst.1)
            .src_access_mask(src.1)
            .dst_queue_family_index(dst.0)
            .src_queue_family_index(src.0)
            .old_layout(image.layout())
            .new_layout(new_layout)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(image.aspect())
                .base_array_layer(image.base_array_layer())
                .layer_count(image.layer_count())
                .base_mip_level(image.base_mip_level())
                .level_count(image.level_count())
            )
            .image(image.handle());

        unsafe {
            self.device.handle()
                .cmd_pipeline_barrier(self.handle, src.2, dst.2, dependency, &[], &[], &[barrier]);

            image.layout = new_layout;
        }
    }

    /// Begins a new render pass.
    pub fn begin_render_pass(&self, render_pass : &RenderPass, framebuffer : &Framebuffer, render_area : vk::Rect2D, clear_values : &[ClearValue], contents : vk::SubpassContents) {
        unsafe {
            let begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(render_pass.handle())
                .framebuffer(framebuffer.handle())
                .render_area(render_area)
                .clear_values(clear_values);

            self.device.handle()
                .cmd_begin_render_pass(self.handle, &begin_info, contents);
        }
    }

    /// Transitions to the next subpass of the current render pass.
    pub fn next_subpass(&self, contents : vk::SubpassContents) {
        unsafe {
            self.device.handle().cmd_next_subpass(self.handle, contents);
        }
    }

    /// Executes commands from a given array of command buffers.
    pub fn execute_commands(&self, commands : &[CommandBuffer]) {
        unsafe {
            let handles = commands.iter()
                .map(CommandBuffer::handle)
                .collect::<Vec<_>>();

            self.device.handle().cmd_execute_commands(self.handle, &handles);
        }
    }

    /// Executes commands from a given command buffer.
    pub fn execute_command(&self, command : &CommandBuffer) {
        unsafe {
            let handle = [command.handle()];

            self.device.handle().cmd_execute_commands(self.handle, &handle);
        }
    }

    /// Binds a pipeline object to this command buffer.
    pub fn bind_pipeline(&self, point : vk::PipelineBindPoint, pipeline : &Pipeline) {
        unsafe {
            self.device.handle().cmd_bind_pipeline(self.handle, point, pipeline.handle());
        }
    }

    /// Sets the viewport dynamically for this command buffer.
    pub fn set_viewport(&self, first_viewport : u32, viewports : &[vk::Viewport]) {
        unsafe {
            self.device.handle().cmd_set_viewport(self.handle, first_viewport, viewports);
        }
    }

    /// Sets the scissors dynamically for this command buffer.
    pub fn set_scissors(&self, first_scissor : u32, scissors : &[vk::Rect2D]) {
        unsafe {
            self.device.handle().cmd_set_scissor(self.handle, first_scissor, scissors);
        }
    }

    pub fn draw_indexed(&self, index_count : u32, instance_count : u32, first_index : u32, vertex_offset : i32, first_instance : u32) {
        unsafe {
            self.device.handle()
                .cmd_draw_indexed(self.handle, index_count, instance_count, first_index, vertex_offset, first_instance)
        }
    }

    /// Binds vertex buffers to this command buffer.
    pub fn bind_vertex_buffers(&self, first_binding : u32, buffers : &[(&Buffer, vk::DeviceSize)]) {
        let mut handles = Vec::<vk::Buffer>::with_capacity(buffers.len());
        let mut offsets = Vec::<vk::DeviceSize>::with_capacity(buffers.len());
        for (buffer, offset) in buffers {
            handles.push(buffer.handle());
            offsets.push(*offset);
        }

        unsafe {
            self.device.handle().cmd_bind_vertex_buffers(self.handle, first_binding, &handles, &offsets)
        }
    }

    /// Binds an index buffer to this command buffer.
    pub fn bind_index_buffer(&self, buffer : &Buffer, offset : vk::DeviceSize) {
        unsafe {
            self.device.handle().cmd_bind_index_buffer(self.handle, buffer.handle(), offset, buffer.index_type())
        }
    }
    
    /// Ends the current render pass.
    pub fn end_render_pass(&self) {
        unsafe {
            self.device.handle().cmd_end_render_pass(self.handle);
        }
    }

    /// Draws primitives.
    pub fn draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.handle().cmd_draw(self.handle, vertex_count, instance_count, first_vertex, first_instance)
        }
    }

    /// Copies data between buffer regions.
    pub fn copy_buffer(&self, source : &Buffer, dest : &Buffer, regions : &[vk::BufferCopy]) {
        unsafe {
            self.device.handle().cmd_copy_buffer(self.handle, source.handle(), dest.handle(), regions);
        }
    }

    /// Copies data from a buffer to an image.
    pub fn copy_buffer_to_image(&self, source : &Buffer, dest : &Image, dst_layout : vk::ImageLayout, regions : &[vk::BufferImageCopy]) {
        unsafe {
            self.device.handle().cmd_copy_buffer_to_image(self.handle, source.handle(), dest.handle(), dst_layout, regions);
        }
    }

    /// Updates the values of push constants.
    pub fn push_constants(&self, pipeline : &Pipeline, stage : vk::ShaderStageFlags, offset : u32, constants : &[u8]) {
        unsafe {
            self.device.handle()
                .cmd_push_constants(self.handle, pipeline.layout(), stage, offset, constants);
        }
    }

    pub fn bind_descriptor_sets(&self, point : vk::PipelineBindPoint, pipeline : &Pipeline, first_set : u32, descriptor_sets : &[vk::DescriptorSet], dynamic_offsets : &[u32]) {
        unsafe {
            self.device.handle()
                .cmd_bind_descriptor_sets(self.handle, point, pipeline.layout(), first_set, descriptor_sets, dynamic_offsets)
        }
    }

    /// Copies regions of an image, potentially performing format conversion.
    pub fn blit_image(&self, source : &Image, dest : &mut Image, blit : &[vk::ImageBlit], filter : vk::Filter) {
        debug_assert!(source.sample_count() == vk::SampleCountFlags::TYPE_1 && dest.sample_count() == vk::SampleCountFlags::TYPE_1,
            "blit_image must not be used for multisampled source or destination images. Use resolve_image for this purpose."
        );

        unsafe {
            self.device.handle().cmd_blit_image(self.handle,
                source.handle(),
                source.layout(),
                dest.handle(),
                dest.layout(),
                blit,
                filter);

            dest.layout = source.layout;
        }
    }

    /// Finishes recording this command buffer.
    pub fn end(&self) {
        unsafe {
            self.device.handle().end_command_buffer(self.handle)
                .expect("Failed to finish recording the command buffer.");
        }
    }

    pub fn record<F>(&self, flags : vk::CommandBufferUsageFlags, callback : F) where F : FnOnce(&CommandBuffer) {
        self.begin(flags);

        callback(&self);

        self.end();
    }
}

impl Handle<vk::CommandBuffer> for CommandBuffer {
    fn handle(&self) -> vk::CommandBuffer { self.handle }
}

pub struct CommandBufferBuilder {
    pool   : vk::CommandPool,
    level  : vk::CommandBufferLevel,
}

impl CommandBufferBuilder {
    pub fn pool(mut self, pool : &CommandPool) -> Self {
        self.pool = pool.handle();
        self
    }

    pub fn build_one(self, device : &Arc<LogicalDevice>) -> CommandBuffer {
        let create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(self.level)
            .command_buffer_count(1);

        unsafe {
            let handles = device.handle().allocate_command_buffers(&create_info)
                .expect("Unable to allocate a command buffer");

            CommandBuffer { handle : handles[0], level : self.level, device : device.clone() }
        }
    }

    pub fn build(self, device : &Arc<LogicalDevice>, count : u32) -> Vec<CommandBuffer> {
        let create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(self.level)
            .command_buffer_count(count);

        unsafe {
            device.handle().allocate_command_buffers(&create_info)
                .expect("Unable to allocate a command buffer")
                .into_iter()
                .map(|handle| {
                    CommandBuffer { handle, level : self.level, device : device.clone() }
                })
                .collect()
        }
    }

    value_builder! { level, vk::CommandBufferLevel }
}

pub struct BarrierPhase(pub u32, pub vk::AccessFlags, pub vk::PipelineStageFlags);

impl BarrierPhase {
    pub fn ignore_queue(access_flags : vk::AccessFlags, stage : vk::PipelineStageFlags) -> Self {
        Self(vk::QUEUE_FAMILY_IGNORED, access_flags, stage)
    }
}