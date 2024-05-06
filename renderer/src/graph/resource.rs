use std::collections::HashMap;

use bitmask_enum::bitmask;

use super::{manager::Identifiable, pass::Pass, Graph};

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read = 0x01,
    Write = 0x02
}

/// Models a texture resource.
pub struct Texture {
    name : &'static str,
    usage : ash::vk::ImageUsageFlags,
    passes : HashMap<usize, ResourceAccessFlags>
}

impl Texture {
    pub fn new(name : &'static str) -> Self {
        Self {
            name,
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
    pub fn writers(&self, owner : &Graph, only : bool) -> Vec<&Pass> {
        self.accessors(owner, ResourceAccessFlags::Write, only)
    }

    /// Returns all passes that read from this resource in any order.
    /// 
    /// # Arguments
    /// 
    /// * `only` - If set to `true`, will only return [`Pass`]es that
    ///   don't write to this texture.
    pub fn readers(&self, owner : &Graph, only : bool) -> Vec<&Pass> {
        self.accessors(owner, ResourceAccessFlags::Read, only)
    }

    pub fn accessors(&self, owner : &Graph, flags : ResourceAccessFlags, only : bool) -> Vec<&Pass> {
        self.passes.iter()
            .filter(move |&(_, v)| {
                if only {
                    (*v & flags) == flags
                } else {
                    (*v & flags) != 0
                }
            })
            .filter_map(move |(&k, _)| owner.find_pass_by_id(k))
            .collect::<Vec<_>>()
    }

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
        // access_flags : ResourceAccessFlags,
        usage : ash::vk::ImageUsageFlags
    ) {
        self.usage |= usage;
        /*self.passes.entry(pass.index())
            .and_modify(|value| *value |= access_flags)
            .or_insert(access_flags);*/
    }
}

impl Identifiable for Texture {
    fn name(&self) -> &'static str {
        self.name()
    }
}

/// Models a buffer resource.
pub struct Buffer {

}

impl Identifiable for Buffer {
    fn name(&self) -> &'static str {
        todo!()
    }
}


pub enum Resource {
    Texture(Texture),
    Buffer(Buffer),
}

impl Identifiable for Resource {
    fn name(&self) -> &'static str {
        match self {
            Self::Texture(value) => value.name(),
            Self::Buffer(value) => value.name(),
        }
    }
}

impl Resource {
    pub fn writers(&self, owner : &Graph, only : bool) -> Vec<&Pass> {
        match &self {
            Self::Texture(value) => value.writers(owner, only),
            _ => unimplemented!()
        }
    }
}
