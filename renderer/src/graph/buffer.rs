use super::{manager::{Identifiable, Identifier}, pass::PassID, resource::ResourceID};

pub struct Buffer {
    readers : Vec<PassID>,
    writers : Vec<PassID>,
}

impl Buffer {
    pub(in super) fn register_reader(&mut self, pass_id : PassID) {
        self.readers.push(pass_id);
    }

    pub(in super) fn register_writer(&mut self, pass_id : PassID) {
        self.writers.push(pass_id);
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct BufferID(pub(in super) Identifier);

impl BufferID {
    pub fn to_resource(&self) -> ResourceID {
        ResourceID::Buffer(*self)
    }
}

impl nohash_hasher::IsEnabled for BufferID { }

impl From<BufferID> for Identifier {
    fn from(value: BufferID) -> Self { value.0 }
}

impl Identifiable for Buffer {
    type Key = BufferID;

    fn name(&self) -> &'static str { todo!() }
    fn id(&self) -> BufferID { todo!() }
}

pub struct BufferUsage;