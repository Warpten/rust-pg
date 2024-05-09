use std::{hint, marker::PhantomData};

use super::{manager::{Identifiable, Identifier}, resource::{Resource, ResourceAccessFlags, ResourceID}, Graph};


/// Models a texture resource.
pub struct Texture {
    pub(in self) id : TextureID,
    name : &'static str,

    format : ash::vk::Format,
    levels : u32,
    layers : u32,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextureID(pub (in super) usize);

impl Into<Identifier<ResourceID>> for TextureID {
    fn into(self) -> Identifier<ResourceID> { Identifier::Numeric(self.0, PhantomData::default()) }
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

        let resource_id = manager.resources.register(texture, |instance, id| {
            match instance {
                Resource::Texture(texture) => texture.id = TextureID(id),
                _ => unsafe { hint::unreachable_unchecked() }
            };
        });

        match resource_id {
            ResourceID::Texture(texture) => texture,
            _ => panic!("This should not happen"),
        }
    }
}


#[derive(Clone)]
pub struct TextureUsage {
    pub access_flags : ResourceAccessFlags,
    pub usage_flags : ash::vk::ImageUsageFlags
}