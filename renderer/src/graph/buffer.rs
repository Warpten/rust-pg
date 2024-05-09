use super::manager::Identifiable;


pub struct Buffer {

}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BufferID(pub(in super) usize);

impl Identifiable for Buffer {
    type Key = BufferID;

    fn name(&self) -> &'static str { todo!() }
    fn id(&self) -> BufferID { todo!() }
}

pub struct BufferUsage;