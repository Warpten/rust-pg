use std::hint;

use super::{manager::{Identifiable, Identifier}, pass::PassID, resource::{Resource, ResourceAccessFlags, ResourceID}, Graph};

/// Models a texture resource.
pub struct Texture {
    pub(in self) id : TextureID,
    name : &'static str,

    format : ash::vk::Format,
    levels : u32,
    layers : u32,

    readers : Vec<PassID>,
    writers : Vec<PassID>,
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
            id : TextureID(Identifier::None), 
            
            levels,
            layers,
            format,

            readers : vec![],
            writers : vec![],
        }
    }

    pub fn format(&self) -> ash::vk::Format { self.format }
    pub fn levels(&self) -> u32 { self.levels }
    pub fn layers(&self) -> u32 { self.layers }

    pub fn readers(&self) -> &Vec<PassID> { &self.readers }
    pub fn writers(&self) -> &Vec<PassID> { &self.writers }

    pub fn register(self, manager : &mut Graph) -> TextureID {
        let texture = Resource::Texture(self);

        let resource_id = manager.resources.register(texture, |instance, id| {
            match instance {
                Resource::Texture(texture) => texture.id = TextureID(Identifier::Numeric(id)),
                _ => unsafe { hint::unreachable_unchecked() }
            };
        });

        match resource_id {
            ResourceID::Texture(texture) => texture,
            _ => panic!("This should not happen"),
        }
    }

    pub(in super) fn register_reader(&mut self, pass_id : PassID) {
        self.readers.push(pass_id);
    }

    pub(in super) fn register_writer(&mut self, pass_id : PassID) {
        self.writers.push(pass_id);
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TextureID(pub(in super) Identifier);

impl TextureID {
    pub fn to_resource(&self) -> ResourceID {
        ResourceID::Texture(*self)
    }
}

impl From<TextureID> for Identifier {
    fn from(value: TextureID) -> Self { value.0 }
}

#[derive(Clone)]
pub struct TextureUsage {
    pub access_flags : ResourceAccessFlags,
    pub usage_flags : ash::vk::ImageUsageFlags
}