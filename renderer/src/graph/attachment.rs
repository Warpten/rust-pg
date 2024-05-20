use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::pass::Pass;
use crate::graph::resource::{Identifiable, ResourceID};
use crate::graph::texture::TextureOptions;

use super::resource::ResourceAccessFlags;

pub struct Attachment {
    id   : AttachmentID,
    name : &'static str,
}

impl Attachment {
    pub fn new(name : &'static str) -> Self {
        Self {
            id : AttachmentID(usize::MAX),
            name
        }
    }

    /// Registers this attachment on the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph on which to register.
    pub fn register(self, graph : &mut Graph) -> AttachmentID {
        let registered_self = graph.attachments.register(self, |instance, id| instance.id = AttachmentID(id));

        assert_ne!(registered_self.id(), AttachmentID(usize::MAX));

        registered_self.id()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct AttachmentID(usize);

impl AttachmentID {
    pub fn get<'a>(&self, graph : &'a Graph) -> &'a Attachment {
        graph.attachments.find(*self).unwrap()
    }

    pub fn get_options(&self, pass : &Pass) -> &AttachmentOptions {
        pass.attachments.get(self).unwrap()
    }
}

impl Into<ResourceID> for AttachmentID {
    fn into(self) -> ResourceID { ResourceID::Attachment(self) }
}

impl Into<Identifier> for AttachmentID {
    fn into(self) -> Identifier { Identifier::Numeric(self.0) }
}

impl Identifiable for Attachment {
    type IdentifierType = AttachmentID;

    fn id(&self) -> Self::IdentifierType { self.id }
    fn name(&self) -> &'static str { self.name }
}

pub struct AttachmentOptions {
    pub load_op : ash::vk::AttachmentLoadOp,
    pub store_op : ash::vk::AttachmentStoreOp,
}

impl AttachmentOptions {
    pub fn access_flags(&self) -> ResourceAccessFlags {
        let flags = ResourceAccessFlags::none();
    }
}
