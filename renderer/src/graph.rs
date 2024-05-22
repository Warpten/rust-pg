use std::sync::Arc;
use crate::graph::attachment::{Attachment, AttachmentID, AttachmentOptions};
use crate::graph::buffer::{Buffer, BufferID, BufferOptions};
use crate::graph::manager::Manager;
use crate::graph::pass::{Pass, PassID};
use crate::graph::resource::{Identifiable, PhysicalResourceID, Resource, ResourceID};
use crate::graph::texture::{Texture, TextureID, TextureOptions};
use crate::traits::handle::BorrowHandle;
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
    pub fn build(&mut self) {
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
                    sorted.iter()
                        .map(|id| id.get(self))
                        .collect::<Vec<_>>()
                },
                Err(_) => panic!("Cyclic graph detected"),
            }
        };

        // Walk the topology and process resources
        for pass in topology {
            for resource in pass.resources() {
                let physical_resource = resource.devirtualize();
                match physical_resource {
                    PhysicalResourceID::Texture(texture) => {
                        let options = texture.get_options(pass).unwrap();
                        let texture = texture.get(self).unwrap();
                        self.process_texture(pass, texture, options);
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
        }
    }

    fn process_texture(&mut self, pass: &Pass, texture: &Texture, options: &TextureOptions) {
        // TODO: this needs to persist across calls and is specific to the texture
        let mut state = TextureState {
            id: Default::default(),
            layout: Default::default(),
            command_buffer : self.command_pool.rent(ash::vk::CommandBufferLevel::SECONDARY, 1)[0],
            // This one is tricky, this is where aliasing happens - we need a pool of images
            // and their associated memory and rent/return from/to it.
            handle : Default::default(),
        };

        // If a layout was requested and it differs from the current one, update the state and
        // record a layout transition command.
        // TODO: Accumulate barriers and schedule the layout transitions as late as possible ?
        //       Wouldn't this still stall if the transitions get flushed before a new transition happen?
        //       In that case, the first transition could be considered redundant and we could effectively
        //       collapse the intermediary layout, I guess...
        //       Something to keep in mind for V2.
        if let Some(new_layout) = options.layout {
            if new_layout != state.layout {
                state.handle.layout_transition(state.command_buffer, state.layout, new_layout);
                state.layout = new_layout;
            }
        }
    }

    fn process_buffer(&mut self, pass: &Pass, buffer: &Buffer, options: &BufferOptions) {}

    fn process_attachment(&mut self, pass : &Pass, attachment : &Attachment, options : &AttachmentOptions) {}
}

impl Graph { // Public API
    pub fn new(device : &Arc<LogicalDevice>) -> Self {
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
struct TextureState {
    pub id : TextureID,
    pub layout : ash::vk::ImageLayout,
    pub command_buffer : ash::vk::CommandBuffer,
    pub handle : Image,
}

impl TextureState {
    pub fn new(texture : &Texture, device : &Arc<LogicalDevice>, pool : &CommandPool) -> Self {
        Self {
            id : texture.id(),
            layout : texture.layout(),
            handle : Image::new(texture.name(),
                device,
                ash::vk::ImageCreateInfo::default(),
                ash::vk::ImageAspectFlags::empty(),
                texture.levels()),
            command_buffer : pool.rent_one(ash::vk::CommandBufferLevel::SECONDARY)
        }
    }
}
