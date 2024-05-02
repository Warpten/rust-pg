use std::{collections::HashMap, rc::Rc};

use super::{pass::{Pass, ResourceAccessFlags}, Graph};

/// Models a texture resource.
pub struct Texture {
    owner : Rc<Graph>,
    /// ID of this texture in the owner [`Graph`].
    index : usize,
    usage : ash::vk::ImageUsageFlags,
    /// Associative map of all the passes using this texture.
    ///   Key is the unique identifier of that pass in the graph
    ///       (and just so happens to be its index)
    ///   Value is the combined acces flags on that texture by the pass.
    passes : HashMap<usize, ResourceAccessFlags>,
}

impl Texture {
    pub fn new(owner : Rc<Graph>, index : usize) -> Self {
        Self {
            owner,
            index,
            usage : Default::default(),
            passes : HashMap::new()
        }
    }

    /// Returns all passes that write to this resource in any order.
    /// 
    /// # Arguments
    /// 
    /// * `only` - If set to `true`, will only return [`Pass`]es that
    ///   don't read from this texture.
    pub fn writers(&self, only : bool) -> impl Iterator<Item = &Pass>{
        self.accessors(move |i| {
            if only {
                (i & ResourceAccessFlags::Write) == ResourceAccessFlags::Write
            } else {
                (i & ResourceAccessFlags::Write) != 0
            }
        })
    }

    /// Returns all passes that read from this resource in any order.
    /// 
    /// # Arguments
    /// 
    /// * `only` - If set to `true`, will only return [`Pass`]es that
    ///   don't write to this texture.
    pub fn readers(&self, only : bool) -> impl Iterator<Item = &Pass> {
        self.accessors(move |i| {
            if only {
                (i & ResourceAccessFlags::Read) == ResourceAccessFlags::Read
            } else {
                (i & ResourceAccessFlags::Read) != 0
            }
        })
    }

    /// Returns all passes that access this resource with any of the flag combination provided.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - A combination of access flags.
    pub fn accessors<F>(&self, filter : F) -> impl Iterator<Item = &Pass>
        where F : Fn(ResourceAccessFlags) -> bool
    {
        self.passes.iter()
            .filter(move |&(_, v)| filter(*v))
            .filter_map(|(k, _)| self.owner.find_pass_by_id(*k))
    }

    /// Returns the graph owning this texture.
    pub fn graph(&self) -> &Graph {
        &self.owner.as_ref()
    }

    /// Returns this texture's ID in the owning graph.
    pub fn id(&self) -> usize { self.index }

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

/// Models a buffer resource.
pub struct Buffer {

}

pub enum Resource {
    Texture { value: Texture, id : usize },
    Buffer { value : Buffer, id : usize },
    None
}

impl Resource {
    pub fn writers(&self, only : bool) -> impl Iterator<Item = &Pass> {
        match &self {
            Self::Texture { id : _, value } => value.writers(only),
            _ => unimplemented!()
        }
    }
}

impl Default for Resource {
    fn default() -> Self {
        Self::None
    }
}