use crate::graph::attachment::{Attachment, AttachmentID};
use crate::graph::buffer::{Buffer, BufferID};
use crate::graph::manager::Manager;
use crate::graph::pass::Pass;
use crate::graph::resource::{Resource, ResourceID};
use crate::graph::texture::{Texture, TextureID};

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
        unimplemented!("Fix me")
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
    use crate::graph::resource::{ResourceID, ResourceAccessFlags};
    use crate::graph::texture::{Texture, TextureOptions};

    #[test]
    pub fn test_graph() {
        let mut graph = crate::graph::Graph::default();

        let tex = Texture::new("Texture 1", 1, 1, ash::vk::Format::A1R5G5B5_UNORM_PACK16)
            .register(&mut graph);

        let attachment = Attachment::new("Attachment 1")
            .register(&mut graph);

        let a = Pass::new("A")
            .add_texture("A[1]", &ResourceID::Texture(tex), ResourceAccessFlags::Read | ResourceAccessFlags::Write, TextureOptions {
                usage_flags : ash::vk::ImageUsageFlags::COLOR_ATTACHMENT
            })
            .register(&mut graph);

        let b = Pass::new("B")
            .add_texture("B[1]", a.get(&graph).texture("A[1]").unwrap(), ResourceAccessFlags::Read, TextureOptions {
                usage_flags : ash::vk::ImageUsageFlags::COLOR_ATTACHMENT
            })
            .register(&mut graph);
    }
}
