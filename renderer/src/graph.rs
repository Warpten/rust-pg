use crate::graph::attachment::{Attachment, AttachmentID};
use crate::graph::buffer::{Buffer, BufferID};
use crate::graph::manager::Manager;
use crate::graph::pass::{Pass, PassID};
use crate::graph::resource::{Identifiable, Resource, ResourceID};
use crate::graph::texture::{Texture, TextureID};
use crate::utils::topological_sort::TopologicalSorter;

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
}

impl Graph {
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
                    sorted.iter()
                        .map(|id| id.get(self))
                        .collect::<Vec<_>>()
                },
                Err(_) => panic!("Cyclic graph detected"),
            }
        };

        // Walk the topology.
        for pass in topology {
            for resource in pass.inputs() {
                let physical_resource = resource.devirtualize();
                match physical_resource {
                    ResourceID::Texture(texture) => {
                        let options = texture.get_options(pass);
                        let texture = texture.get(self);
                    },
                    ResourceID::Buffer(buffer) => {
                        let options = buffer.get_options(pass);
                        let buffer = buffer.get(self);
                    },
                    ResourceID::Attachment(attachment) => {
                        let options = attachment.get_options(pass);
                        let attachment = attachment.get(self);
                    },
                    _ => panic!("Unreachable code")
                };
            }
        }
    }

    pub fn find_texture(&self, texture : TextureID) -> Option<&Texture> { self.textures.find(texture) }

    pub fn find_buffer(&self, texture : BufferID) -> Option<&Buffer> { self.buffers.find(texture) }

    pub fn find_attachment(&self, attachment : AttachmentID) -> Option<&Attachment> { self.attachments.find(attachment) }

    pub fn find_resource<'a>(&'a self, resource : ResourceID) -> Option<Resource<'a>> {
        match resource {
            ResourceID::Texture(texture) => {
                self.find_texture(texture).map(|tex : &'a Texture| {
                    Resource::Texture(tex)
                })
            },
            ResourceID::Buffer(buffer) => {
                self.find_buffer(buffer).map(|buf : &'a Buffer| {
                    Resource::Buffer(buf)
                })
            },
            ResourceID::Attachment(attachment) => {
                self.find_attachment(attachment).map(|att : &'a Attachment| {
                    Resource::Attachment(att)
                })
            }
            ResourceID::Virtual(_, resource) => {
                self.find_resource(*resource)
            },
        }
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            passes : Default::default(),
            textures : Default::default(),
            buffers : Default::default(),
            attachments : Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::attachment::Attachment;
    use crate::graph::pass::Pass;
    use crate::graph::resource::ResourceID;
    use crate::graph::texture::{Texture, TextureOptions};

    #[test]
    pub fn test_graph() {
        let mut graph = crate::graph::Graph::default();

        let tex = Texture::new("Texture 1", 1, 1, ash::vk::Format::A1R5G5B5_UNORM_PACK16)
            .register(&mut graph);

        let attachment = Attachment::new("Attachment 1")
            .register(&mut graph);

        let a = Pass::new("A")
            .add_texture("A[1]", &ResourceID::Texture(tex), TextureOptions {
                usage_flags : ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..Default::default()
            })
            .register(&mut graph);

        let b = Pass::new("B")
            .add_texture("B[1]", a.get(&graph).texture("A[1]").unwrap(), TextureOptions {
                usage_flags : ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..Default::default()
            })
            .register(&mut graph);
    }
}
