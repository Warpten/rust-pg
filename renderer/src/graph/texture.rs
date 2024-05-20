use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::pass::Pass;
use crate::graph::resource::{Identifiable, ResourceAccessFlags, ResourceID, ResourceOptions};

pub struct Texture {
    id   : TextureID,
    name : &'static str,

    levels : u32,
    layers : u32,
    format : ash::vk::Format,
}

impl Texture {
    pub fn new(name : &'static str, levels : u32, layers : u32, format : ash::vk::Format) -> Texture {
        Self {
            name,
            id : TextureID(usize::MAX),

            levels,
            layers,
            format
        }
    }

    /// Registers this attachment on the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph on which to register.
    pub fn register(self, graph : &mut Graph) -> TextureID {
        let registered_self = graph.textures.register(self, |instance, id| instance.id = TextureID(id));

        assert_ne!(registered_self.id(), TextureID(usize::MAX));

        registered_self.id()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct TextureID(usize);

impl TextureID {
    pub fn get<'a>(&self, graph : &'a Graph) -> &'a Texture {
        graph.textures.find(*self).unwrap()
    }

    pub fn get_options<'a>(&self, pass : &'a Pass) -> &'a TextureOptions {
        pass.textures.get(self).unwrap()
    }
}

impl Into<ResourceID> for TextureID {
    fn into(self) -> ResourceID { ResourceID::Texture(self) }
}

impl Into<Identifier> for TextureID {
    fn into(self) -> Identifier { Identifier::Numeric(self.0) }
}

impl Identifiable for Texture {
    type IdentifierType = TextureID;

    fn id(&self) -> Self::IdentifierType { self.id }
    fn name(&self) -> &'static str { self.name }
}

#[derive(Default)]
pub struct TextureOptions {
    pub usage_flags : ash::vk::ImageUsageFlags,
}

impl ResourceOptions for TextureOptions {
    fn access_flags(&self) -> ResourceAccessFlags {
        todo!()
    }
}
