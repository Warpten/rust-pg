use ash::vk;
use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::pass::Pass;
use crate::graph::resource::{Identifiable, ResourceID, ResourceOptions, ResourceAccessFlags, PhysicalResourceID};

pub struct Attachment {
    id   : AttachmentID,
    name : &'static str,

    samples : u32,
}

impl Attachment {
    pub fn new(name : &'static str) -> Self {
        Self {
            id : AttachmentID(usize::MAX),
            name,
            samples : 1,
        }
    }

    pub fn samples(mut self, samples : u32) -> Self {
        self.samples = samples;
        self
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
    pub fn get<'a>(&self, graph : &'a Graph) -> Option<&'a Attachment> {
        graph.attachments.find(*self)
    }

    pub fn get_options<'a>(&self, pass : &'a Pass) -> Option<&'a AttachmentOptions> {
        pass.attachments.get(self)
    }
}

impl Into<ResourceID> for AttachmentID {
    fn into(self) -> ResourceID { ResourceID::Physical(PhysicalResourceID::Attachment(self)) }
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
    pub load_operation : AttachmentLoadOperation,
    pub store_operation : AttachmentStoreOperation,
    pub initial_layout : vk::ImageLayout,
    pub final_layout : vk::ImageLayout,
}

impl ResourceOptions for AttachmentOptions {
    fn access_flags(&self) -> ResourceAccessFlags {
        let mut flags = ResourceAccessFlags::none();

        match self.load_operation {
            AttachmentLoadOperation::Load => flags = flags.and(ResourceAccessFlags::Read),
            AttachmentLoadOperation::Clear(_) => flags = flags.and(ResourceAccessFlags::Write),
            AttachmentLoadOperation::DontCare => (),
        };

        match self.store_operation {
            AttachmentStoreOperation::Store => flags = flags.and(ResourceAccessFlags::Write),
            AttachmentStoreOperation::DontCare => (),
        };

        flags
    }
}

impl Default for AttachmentOptions {
    fn default() -> Self {
        Self {
            load_operation : Default::default(),
            store_operation : Default::default(),
            initial_layout : Default::default(),
            final_layout : Default::default()
        }
    }
}

#[derive(Default)]
pub enum AttachmentLoadOperation {
    Load,
    // TODO: The clear value is passed when VkCmdBeginRenderPass happens;
    //       but if the framebuffer has multiple attachments, we need a clear
    //       for all of them; do we abstract the framebuffer as multiple
    //       attachments or treat it as a single one?
    Clear(vk::ClearValue),
    #[default]
    DontCare,
}

#[derive(Default)]
pub enum AttachmentStoreOperation {
    Store,
    #[default]
    DontCare,
}
