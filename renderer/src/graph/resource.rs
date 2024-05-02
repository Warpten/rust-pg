use std::{collections::HashMap, rc::Rc, slice::Iter};

use super::{pass::{Pass, ResourceAccessFlags}, Graph};

/// A texture buffer.
pub struct Texture {
    owner : Rc<Graph>,
    /// ID of this texture in the owner [`Graph`].
    index : u32,
    usage : ash::vk::ImageUsageFlags,
    /// Associative map of all the passes using this texture.
    ///   Key is the unique identifier of that pass in the graph
    ///       (and just so happens to be its index)
    ///   Value is the combined acces flags on that texture by the pass.
    passes : HashMap<usize, ResourceAccessFlags>,
}

impl Texture {
    pub fn new(owner : Rc<Graph>, index : u32) -> Self {
        Self {
            owner,
            index,
            usage : Default::default(),
            passes : HashMap::new()
        }
    }

    /// Returns all passes that write to this resource in any order.
    pub fn writers(&self) -> impl Iterator<Item = &Pass>{
        self.accessors(ResourceAccessFlags::Write)
    }

    /// Returns all passes that read from this resource in any order.
    pub fn readers(&self) -> impl Iterator<Item = &Pass> {
        self.accessors(ResourceAccessFlags::Read)
    }

    /// Returns all passes that access this resource with any of the flag combination provided.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - A combination of access flags.
    pub fn accessors(&self, flags : ResourceAccessFlags) -> impl Iterator<Item = &Pass> {
        self.passes.iter()
            .filter(move |&(_, v)| (*v & flags) != 0)
            .filter_map(|(k, _)| self.owner.find_pass_by_id(*k))
    }

    /// Returns the graph owning this texture.
    pub fn graph(&self) -> &Graph {
        &self.owner.as_ref()
    }

    /// Returns this texture's ID in the owning graph.
    pub fn id(&self) -> u32 { self.index }

    /// Returns the combined usages of this texture.
    pub fn usage(&self) -> ash::vk::ImageUsageFlags { self.usage }

    /// Indicates to this texture that it is being used by a given pass.
    /// 
    /// # Arguments
    /// 
    /// * `pass` - The pass using this texture.
    /// * `access_flags` - The access flags used by a pass on this texture.
    /// * `usage` - How is this image used?
    /// 
    /// # Notes
    /// 
    /// This method should **always** be called from either of:
    /// * [`Pass::texture_attachment`]
    /// * [`Pass::color_attachment`]
    pub(super) fn add_usage(
        &mut self,
        pass : &Pass,
        access_flags : ResourceAccessFlags,
        usage : ash::vk::ImageUsageFlags
    ) {
        self.usage |= usage;
        self.passes.entry(pass.index())
            .and_modify(|value| *value |= access_flags)
            .or_insert(access_flags);
    }
}

pub struct Buffer {

}

pub enum Resource {
    Texture { value: Texture, id : usize },
    Buffer { value : Buffer, id : usize },
    None
}

impl Default for Resource {
    fn default() -> Self {
        Self::None
    }
}