use bitmask_enum::bitmask;
use crate::graph::attachment::{Attachment, AttachmentID};
use crate::graph::buffer::{Buffer, BufferID};
use crate::graph::manager::Identifier;
use crate::graph::pass::PassID;
use crate::graph::texture::{Texture, TextureID};

pub trait Identifiable {
    /// The type of the identifier associated with this resource.
    type IdentifierType : Into<Identifier> + Copy;

    fn id(&self) -> Self::IdentifierType;
    fn name(&self) -> &'static str;
}

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    /// This resource is read from.
    Read,
    /// This resource is written to.
    Write
}

pub enum Resource<'a> {
    Buffer(&'a Buffer),
    Texture(&'a Texture),
    Attachment(&'a Attachment),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PhysicalResourceID {
    Texture(TextureID),
    Buffer(BufferID),
    Attachment(AttachmentID)
}

impl PhysicalResourceID {
    pub fn is_texture(&self) -> bool {
        if let Self::Texture(_) = self { true } else { false }
    }
    pub fn is_buffer(&self) -> bool {
        if let Self::Buffer(_) = self { true } else { false }
    }
    pub fn is_attachment(&self) -> bool {
        if let Self::Attachment(_) = self { true } else { false }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ResourceID {
    /// A physical resource.
    Physical(PhysicalResourceID),
    /// A virtual resources. This effectively links a resource with a pass
    /// and serves as a way to identify pass inputs that are another pass's
    /// output.
    Virtual(PassID, PhysicalResourceID)
}

impl ResourceID {
    pub fn texture(tex : TextureID) -> Self {
        Self::Physical(PhysicalResourceID::Texture(tex))
    }

    pub fn buffer(buf : BufferID) -> Self {
        Self::Physical(PhysicalResourceID::Buffer(buf))
    }

    pub fn attachment(att : AttachmentID) -> Self {
        Self::Physical(PhysicalResourceID::Attachment(att))
    }

    pub fn devirtualize(&self) -> &PhysicalResourceID {
        match self {
            ResourceID::Physical(res) => res,
            ResourceID::Virtual(_, res) => res,
        }
    }
}

pub trait ResourceOptions {
    fn access_flags(&self) -> ResourceAccessFlags;
}