use std::sync::Arc;

use ash::vk::{self, ClearValue};

use crate::{traits::handle::Handle};

use super::{buffer::Buffer, command_pool::CommandPool, framebuffer::Framebuffer, logical_device::LogicalDevice, pipeline::Pipeline, queue::Queue, render_pass::RenderPass};

pub struct CommandBuffer {
    device : Arc<LogicalDevice>,
    handle : vk::CommandBuffer,
    level : vk::CommandBufferLevel,
}

impl CommandBuffer {
    pub fn builder() -> CommandBufferBuilder {
        CommandBufferBuilder { pool : vk::CommandPool::null(), level : vk::CommandBufferLevel::PRIMARY }
    }

    pub fn begin(&self, flags : vk::CommandBufferUsageFlags) {
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(flags);

            self.device.handle()
                .begin_command_buffer(self.handle, &begin_info)
                .expect("Failed to begin recording the command buffer.");
        }
    }

    pub fn begin_render_pass(&self, render_pass : &RenderPass, framebuffer : &Framebuffer, render_area : vk::Rect2D, clear_values : &[ClearValue], contents : vk::SubpassContents)
    {
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

    pub fn bind_pipeline(&self, point : vk::PipelineBindPoint, pipeline : &Pipeline) {
        unsafe {
            self.device.handle().cmd_bind_pipeline(self.handle, point, pipeline.handle());
        }
    }

    pub fn set_viewport(&self, first_viewport : u32, viewports : &[vk::Viewport]) {
        unsafe {
            self.device.handle().cmd_set_viewport(self.handle, first_viewport, viewports);
        }
    }

    pub fn set_scissors(&self, first_scissor : u32, scissors : &[vk::Rect2D]) {
        unsafe {
            self.device.handle().cmd_set_scissor(self.handle, first_scissor, scissors);
        }
    }

    pub fn bind_vertex_buffers(&self, first_binding : u32, buffers : &[&Buffer], offsets : &[vk::DeviceSize]) {
        let buffer_handles = buffers.iter()
            .map(|b| b.handle())
            .collect::<Vec<_>>();

        unsafe {
            self.device.handle().cmd_bind_vertex_buffers(self.handle, first_binding, &buffer_handles, offsets)
        }
    }
    
    pub fn end_render_pass(&self) {
        unsafe {
            self.device.handle().cmd_end_render_pass(self.handle);
        }
    }

    pub fn draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.handle().cmd_draw(self.handle, vertex_count, instance_count, first_vertex, first_instance)
        }
    }

    pub fn copy_buffer(&self, source : &Buffer, dest : &Buffer, regions : &[vk::BufferCopy]) {
        unsafe {
            self.device.handle().cmd_copy_buffer(self.handle, source.handle(), dest.handle(), regions);
        }
    }

    pub fn submit_to_queue(&self, queue : &Queue, fence : vk::Fence) {
        let handles = [self.handle];

        let submit_infos = [
            vk::SubmitInfo::default()
                .command_buffers(&handles)
        ];

        self.device.submit(queue, &submit_infos, fence);
    }

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