use bitmask_enum::bitmask;

use super::{buffer::{Buffer, BufferID}, manager::{Identifiable, Identifier}, pass::PassID, texture::{Texture, TextureID}};

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read = 0x01,
    Write = 0x02
}

pub enum Resource {
    Texture(Texture),
    Buffer(Buffer),
}

impl Resource {
    pub fn register_reader(&mut self, pass_id : PassID) {
        match self {
            Resource::Texture(texture) => texture.register_reader(pass_id),
            Resource::Buffer(buffer) => buffer.register_reader(pass_id),
        }
    }
    pub fn register_writer(&mut self, pass_id : PassID) {
        match self {
            Resource::Texture(texture) => texture.register_writer(pass_id),
            Resource::Buffer(buffer) => buffer.register_writer(pass_id),
        }
    }
}

impl Identifiable for Resource {
    type Key = ResourceID;

    fn name(&self) -> &'static str {
        match self {
            Self::Texture(value) => value.name(),
            Self::Buffer(value) => value.name(),
        }
    }

    fn id(&self) -> ResourceID {
        match self {
            Self::Texture(value) => value.id().into(),
            Self::Buffer(value) => value.id().into(),
        }
    }
}

/// Encapsulates varying types of resource.
/// 
/// Note that this cannot implement [`Copy`] because the [`ResourceID::Virtual`] variant is
/// recursive and needs an indirection that is not [`Copy`]able.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ResourceID {
    Texture(TextureID),
    Buffer(BufferID),
    
    Virtual(PassID, Box<ResourceID>),
    None
}

impl From<ResourceID> for Identifier {
    fn from(value: ResourceID) -> Self {
        match value {
            ResourceID::Texture(texture) => Identifier::from(texture),
            ResourceID::Buffer(buffer) => Identifier::from(buffer),
            ResourceID::Virtual(_, resource) => Identifier::from(*resource),
            ResourceID::None => Identifier::Numeric(usize::MAX),
        }
    }
}

impl From<TextureID> for ResourceID {
    fn from(val: TextureID) -> Self { ResourceID::Texture(val) }
}

impl From<BufferID> for ResourceID {
    fn from(val: BufferID) -> Self { ResourceID::Buffer(val) }
}

