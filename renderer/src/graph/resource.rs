use bitmask_enum::bitmask;

use super::{buffer::{Buffer, BufferID}, manager::Identifiable, pass::PassID, texture::{Texture, TextureID}};

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read = 0x01,
    Write = 0x02
}

pub enum Resource {
    Texture(Texture),
    Buffer(Buffer),
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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ResourceID {
    Texture(TextureID),
    Buffer(BufferID),
    
    Virtual(PassID, Box<ResourceID>),
    None
}

impl From<TextureID> for ResourceID {
    fn from(val: TextureID) -> Self { ResourceID::Texture(val) }
}

impl From<BufferID> for ResourceID {
    fn from(val: BufferID) -> Self { ResourceID::Buffer(val) }
}

