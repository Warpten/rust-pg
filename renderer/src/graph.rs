use std::collections::HashMap;
use std::sync::Arc;
use crate::graph::attachment::{Attachment, AttachmentID, AttachmentOptions};
use crate::graph::buffer::{Buffer, BufferID, BufferOptions};
use crate::graph::manager::Manager;
use crate::graph::pass::{Pass, PassID};
use crate::graph::resource::{Identifiable, PhysicalResourceID, Resource, ResourceID};
use crate::graph::texture::{Texture, TextureID, TextureOptions};
use crate::utils::topological_sort::TopologicalSorter;
use crate::vk::{CommandPool, Image, LogicalDevice};

pub mod attachment;
pub mod buffer;
pub mod manager;
pub mod resource;
pub mod pass;
pub mod texture;

pub struct Graph {
    pub(in crate) passes : Manager<Pass>,
    pub(in crate) textures : Manager<Texture>,
    pub(in crate) buffers : Manager<Buffer>,
    pub(in crate) attachments : Manager<Attachment>,

    device : Arc<LogicalDevice>,
    command_pool : CommandPool,
}

impl Graph { // Graph compilation functions
    /// Builds this graph into a render pass.
    pub fn build(&self) {
        let topology = {
            let mut sorter = TopologicalSorter::<PassID>::default();
            for pass in self.passes.iter() {
                for resource in pass.inputs() {
                    if let ResourceID::Virtual(resource, _) = resource {
                        sorter = sorter.add_edge(*resource, pass.id());
                    }
                }
            }

            match sorter.sort_kahn() {
                Ok(sorted) => {
                    let mut sorted_passes = Vec::with_capacity(sorted.len());
                    for sorted_id in sorted {
                        sorted_passes.push(self.passes.find(sorted_id));
                    }
                    sorted_passes
                },
                Err(_) => panic!("Cyclic graph detected"),
            }
        };

        // Walk the topology and process resources
        let mut texture_state_tracker = HashMap::<TextureID, TextureState>::new();
        for pass in topology {
            let pass = pass.unwrap();
            let command_buffer = self.command_pool.rent_one(ash::vk::CommandBufferLevel::SECONDARY);

            for resource in pass.resources() {
                let physical_resource = resource.devirtualize();
                match physical_resource {
                    PhysicalResourceID::Texture(texture) => {
                        let options = texture.get_options(pass).unwrap();
                        let texture = texture.get(self).unwrap();

                        // Create the tracking state for this resource if it doesn't exist yet.
                        let state = texture_state_tracker.entry(texture.id())
                            .or_insert_with(|| TextureState::new(texture, &self.device));

                        // Process the update.
                        self.process_texture(command_buffer, options, state);
                    },
                    PhysicalResourceID::Buffer(buffer) => {
                        let options = buffer.get_options(pass).unwrap();
                        let buffer = buffer.get(self).unwrap();

                        self.process_buffer(pass, buffer, options);
                    },
                    PhysicalResourceID::Attachment(attachment) => {
                        let options = attachment.get_options(pass).unwrap();
                        let attachment = attachment.get(self).unwrap();
                        self.process_attachment(pass, attachment, options);
                    }
                };
            }

            if let Some(pass_emitter) = pass.command_emitter {
                pass_emitter(command_buffer);
            }

            // Persist the command buffer here.
        }
    }

    fn process_texture(
        &self,
        command_buffer : ash::vk::CommandBuffer,
        options: &TextureOptions,
        state : &mut TextureState)
    {
        // If a layout was requested and it differs from the current one, update the state and
        // record a layout transition command.
        // TODO: Accumulate barriers and schedule the layout transitions as late as possible ?
        //       Wouldn't this still stall if the transitions get flushed before a new transition happen?
        //       In that case, the first transition could be considered redundant and we could effectively
        //       collapse the intermediary layout, I guess...
        //       Something to keep in mind for V2.

        if let Some(new_layout) = options.layout {
            state.emit_transition(command_buffer, new_layout);
        }
    }

    fn process_buffer(&self, pass: &Pass, buffer: &Buffer, options: &BufferOptions) {}

    fn process_attachment(&self, pass : &Pass, attachment : &Attachment, options : &AttachmentOptions) {}
}

impl Graph { // Public API
    pub fn new<'device>(device : &Arc<LogicalDevice>) -> Self {
        Self {
            passes: Default::default(),
            textures: Default::default(),
            buffers: Default::default(),
            attachments: Default::default(),

            device : device.clone(),
            command_pool : CommandPool::create(todo!(), device),
        }
    }

    pub fn find_texture(&self, texture : TextureID) -> Option<&Texture> { self.textures.find(texture) }

    pub fn find_buffer(&self, texture : BufferID) -> Option<&Buffer> { self.buffers.find(texture) }

    pub fn find_attachment(&self, attachment : AttachmentID) -> Option<&Attachment> { self.attachments.find(attachment) }

    pub fn find_resource<'a>(&'a self, resource : ResourceID) -> Option<Resource<'a>> {
        match resource.devirtualize() {
            PhysicalResourceID::Texture(texture) => {
                self.find_texture(*texture).map(|tex : &'a Texture| {
                    Resource::Texture(tex)
                })
            },
            PhysicalResourceID::Buffer(buffer) => {
                self.find_buffer(*buffer).map(|buf : &'a Buffer| {
                    Resource::Buffer(buf)
                })
            },
            PhysicalResourceID::Attachment(attachment) => {
                self.find_attachment(*attachment).map(|att : &'a Attachment| {
                    Resource::Attachment(att)
                })
            }
        }
    }
}

/// Tracks the properties of a texture during the compilation of a render graph.
struct TextureState<'a> {
    /// The current layout of the texture. This value is used to track necessary layout transitions
    /// when walking the graph topology.
    pub current_layout : ash::vk::ImageLayout,

    pub device : Arc<LogicalDevice>,
    /// The initial state of this texture, as defined in the graph.
    pub texture_info : &'a Texture,

    /// The actual image handle.
    pub handle : Option<Image>
}

impl TextureState<'_> {
    pub fn new<'a>(texture : &'a Texture, device : &Arc<LogicalDevice>) -> TextureState<'a> {
        TextureState {
            device : device.clone(),
            texture_info : texture,
            current_layout : texture.layout(),
            handle : None,
        }
    }

    /// Records a layout transition command on the provided command buffer from the current image layout
    /// to the given layout.
    /// 
    /// # Arguments
    /// 
    /// * `command_buffer` - The command buffer on which to record the layout transition.
    /// * `to` - The layout to transition to.
    pub fn emit_transition(&mut self, command_buffer : ash::vk::CommandBuffer , to : ash::vk::ImageLayout) {
        self.emit_layout_transition(command_buffer, self.current_layout, to);
    }

    /// Records a layout transition command on the provided command buffer between the given image layouts.
    /// 
    /// # Description
    /// 
    /// This function does nothing if both layouts are identical.
    /// 
    /// If the texture was not yet allocated on the device, it will be before recording the command.
    /// On top of this, if the initial state of the texture was not defined when it was added to the graph,
    /// the first layout transition is suppressed; the image is created with the final layout as its initial
    /// layout.
    /// 
    /// # Arguments
    /// 
    /// * `command_buffer` - The command buffer on which to record commands.
    /// * `from` - The layout to transition from.
    /// * `to` - The layout to transition to.
    pub fn emit_layout_transition(&mut self, command_buffer : ash::vk::CommandBuffer, from : ash::vk::ImageLayout, to : ash::vk::ImageLayout) {
        // Nothing to be done if the layout does not change.
        if from == to {
            return;
        }

        // Create the image now if it doesn't exist. This has the added benefit of not creating the image
        // if it is never used in the graph.
        let record_command_transition = if self.handle.is_none() {
            let mut create_info = self.texture_info.create_info();

            // Only record a layout transition if the image's layout wasn't undefined.
            // If it was undefined, we pretend the image was initially created with the
            // final layout of the transition.
            let mut record_command_transition = true;
            if self.current_layout == ash::vk::ImageLayout::UNDEFINED {
                create_info.initial_layout = to;

                record_command_transition = false;
            }

            let aspect_mask = Image::derive_aspect_flags(to, create_info.format);

            self.handle = Image::new(self.texture_info.name(),
                &self.device,
                create_info,
                aspect_mask).into();
            
            record_command_transition
        } else { true };

        if record_command_transition {
            // Record the layout transition command on the provided command buffer.
            self.handle.as_ref()
                .unwrap()
                .layout_transition(command_buffer, from, to);
        }
        self.current_layout = to;
    }
}
