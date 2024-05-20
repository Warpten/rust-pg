use bitmask_enum::bitmask;
use crate::graph::attachment::{Attachment, AttachmentID};
use crate::graph::buffer::{Buffer, BufferID};
use crate::graph::manager::Identifier;
use crate::graph::pass::PassID;
use crate::graph::texture::{Texture, TextureID};


pub trait Identifiable {
    /// The type of the identifier associated with this resource.
    type IdentifierType : Into<Identifier>;

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
pub enum ResourceID {
    Texture(TextureID),
    Buffer(BufferID),
    Attachment(AttachmentID),
    Virtual(PassID, Box<ResourceID>)
}

impl ResourceID {
    pub fn devirtualize(&self) -> &ResourceID {
        let mut drill_res : &ResourceID = self;
        while let ResourceID::Virtual(_, res) = drill_res {
            drill_res = &**res;
        }

        drill_res
    }
}

pub trait ResourceOptions {
    fn access_flags(&self) -> ResourceAccessFlags;
}