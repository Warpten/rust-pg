use std::collections::{hash_map::Entry, HashMap};

use bitmask_enum::bitmask;

use super::{buffer::{Buffer, BufferID, BufferUsage}, manager::{Identifiable, Identifier}, pass::Pass, texture::{Texture, TextureID, TextureUsage}};

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read = 0x01,
    Write = 0x02
}

pub enum Resource {
    Texture(Texture),
    Buffer(Buffer),
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ResourceID(pub(in super) usize);

impl From<TextureID> for ResourceID {
    fn from(val: TextureID) -> Self { ResourceID(val.0) }
}

impl From<BufferID> for ResourceID {
    fn from(val: BufferID) -> Self { ResourceID(val.0) }
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
            Resource::Texture(value) => value.id().into(),
            Resource::Buffer(value) => value.id().into(),
        }
    }
}

pub enum ResourceUsage {
    Texture(TextureUsage),
    Buffer(BufferUsage),
}
