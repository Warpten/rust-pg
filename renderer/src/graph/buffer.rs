use crate::graph::attachment::AttachmentOptions;
use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::pass::Pass;
use crate::graph::resource::{Identifiable, ResourceAccessFlags, ResourceID, ResourceOptions};

pub struct Buffer {
    id   : BufferID,
    name : &'static str,
}

impl Buffer {
    pub fn new(name : &'static str) -> Self {
        Self {
            id : BufferID(usize::MAX),
            name
        }
    }

    /// Registers this pass on the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph on which to register.
    pub fn register(self, graph : &mut Graph) -> BufferID {
        let registered_self = graph.buffers.register(self, |instance, id| instance.id = BufferID(id));

        assert_ne!(registered_self.id(), BufferID(usize::MAX));

        registered_self.id()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct BufferID(usize);

impl BufferID {
    pub fn get<'a>(&self, graph : &'a Graph) -> &'a Buffer {
        graph.buffers.find(*self).unwrap()
    }

    pub fn get_options<'a>(&self, pass : &'a Pass) -> Option<&'a BufferOptions> {
        pass.buffers.get(self)
    }
}

impl Into<ResourceID> for BufferID {
    fn into(self) -> ResourceID { ResourceID::Buffer(self) }
}

impl Into<Identifier> for BufferID {
    fn into(self) -> Identifier { Identifier::Numeric(self.0) }
}

impl Identifiable for Buffer {
    type IdentifierType = BufferID;

    fn id(&self) -> Self::IdentifierType { self.id }
    fn name(&self) -> &'static str { self.name }
}

pub struct BufferOptions { }

impl ResourceOptions for BufferOptions {
    fn access_flags(&self) -> ResourceAccessFlags {
        todo!()
    }
}
