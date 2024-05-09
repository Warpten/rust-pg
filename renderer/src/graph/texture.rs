use std::{collections::HashMap, hint};

use super::{manager::Identifiable, resource::{Resource, ResourceAccessFlags, ResourceID}, Graph};


/// Models a texture resource.
pub struct Texture {
    pub(in self) id : TextureID,
    name : &'static str,

    format : ash::vk::Format,
    levels : u32,
    layers : u32,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TextureID(pub (in super) usize);

impl From<ResourceID> for TextureID {
    fn from(value: ResourceID) -> Self { TextureID(value.0) }
}

impl Identifiable for Texture {
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> TextureID { self.id }
    
    type Key = TextureID;
}

impl Texture {
    pub fn new(name : &'static str, levels : u32, layers : u32, format : ash::vk::Format) -> Self {
        Self {
            name,
            id : TextureID(usize::MAX), 
            
            levels,
            layers,
            format,
        }
    }

    pub fn format(&self) -> ash::vk::Format { self.format }
    pub fn levels(&self) -> u32 { self.levels }
    pub fn layers(&self) -> u32 { self.layers }

    pub fn register(self, manager : &mut Graph) -> TextureID {
        let texture = Resource::Texture(self);

        manager.resources.register(texture, |instance, id| {
            match instance {
                Resource::Texture(texture) => texture.id = TextureID(id),
                _ => unsafe { hint::unreachable_unchecked() }
            };
        }).into()
    }
}


#[derive(Clone)]
pub struct TextureUsage {
    pub access_flags : ResourceAccessFlags,
    pub usage_flags : ash::vk::ImageUsageFlags
}