use std::collections::{hash_map::Entry, HashMap};

use bitmask_enum::bitmask;

use super::{manager::{Identifiable, Identifier}, pass::Pass};

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read = 0x01,
    Write = 0x02
}

/// Models a texture resource.
pub struct Texture {
    name : &'static str,
    id : usize,

    accessors : HashMap<usize, ResourceAccessFlags>,
    format : ash::vk::Format,
    levels : u32,
    layers : u32,
}

impl Identifiable for Texture {
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> usize { self.id }
}

impl Texture {
    pub fn new(name : &'static str, id : usize, levels : u32, layers : u32, format : ash::vk::Format) -> Self {
        Self {
            name,
            id,
            
            accessors : HashMap::new(),
            levels,
            layers,
            format,
        }
    }

    pub fn format(&self) -> ash::vk::Format { self.format }
    pub fn levels(&self) -> u32 { self.levels }
    pub fn layers(&self) -> u32 { self.layers }

    pub(in super) fn add_user(&mut self, pass : usize, usage_flags : ResourceAccessFlags) {
        match self.accessors.entry(pass) {
            Entry::Occupied(entry) => { *entry.into_mut() |= usage_flags; },
            Entry::Vacant(entry) => { entry.insert(usage_flags); }
        };
    }

    /// Returns all passes that write to this resource in any order.
    /// 
    /// # Arguments
    /// 
    /// * `only` - If set to `true`, will only return [`Pass`]es that
    ///   don't read from this texture.
    pub fn writers(&self, only : bool) -> Vec<Identifier<Pass>> {
        self.accessors(ResourceAccessFlags::Write, only)
    }

    /// Returns all passes that read from this resource in any order.
    /// 
    /// # Arguments
    /// 
    /// * `only` - If set to `true`, will only return [`Pass`]es that
    ///   don't write to this texture.
    pub fn readers(&self, only : bool) -> Vec<Identifier<Pass>> {
        self.accessors(ResourceAccessFlags::Read, only)
    }

    /// Returns all passes that access this resource with the associated flags.
    /// 
    /// # Arguments
    /// 
    /// * `flags` - One or many access flags.
    /// * `only` - Determines if the provided access flags should be the only ones used for the pass to be returned.
    pub fn accessors(&self, flags : ResourceAccessFlags, only : bool) -> Vec<Identifier<Pass>> {
        self.accessors.iter()
            .filter(move |&(_, v)| {
                if only {
                    (*v & flags) == flags
                } else {
                    (*v & flags) != 0
                }
            })
            .map(move |(&k, _)| k.into())
            .collect::<Vec<Identifier<Pass>>>() // TODO: drop this collect
    }
}

#[derive(Clone)]
pub struct TextureUsage {
    pub access_flags : ResourceAccessFlags,
    pub usage_flags : ash::vk::ImageUsageFlags
}

// ===== Buffer =====

pub struct Buffer {

}

impl Identifiable for Buffer {
    fn name(&self) -> &'static str { todo!() }
    fn id(&self) -> usize { todo!() }
}

pub struct BufferUsage;

// ===== Resource =====

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

    fn id(&self) -> usize {
        match self {
            Resource::Texture(value) => value.id(),
            Resource::Buffer(value) => value.id(),
        }
    }
}

impl Resource {
    pub fn writers(&self, only : bool) -> Vec<Identifier<Pass>> {
        match &self {
            Self::Texture(value) => value.writers(only),
            _ => unimplemented!()
        }
    }
}

pub enum ResourceUsage {
    Texture(TextureUsage),
    Buffer(BufferUsage),
}
