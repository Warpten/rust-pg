use std::collections::HashMap;

use crate::graph::attachment::{AttachmentID, AttachmentOptions};
use crate::graph::buffer::{BufferID, BufferOptions};
use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::resource::{Identifiable, PhysicalResourceID, ResourceAccessFlags, ResourceID, ResourceOptions};
use crate::graph::texture::{TextureID, TextureOptions};
use crate::vk::command_buffer::CommandBuffer;

pub struct Pass {
    id : PassID,
    name : &'static str,

    resource_names : HashMap<&'static str, ResourceID>,
    pub(in crate) command_emitter : Option<fn(&CommandBuffer)>,

    pub(in crate) textures    : HashMap<TextureID, TextureOptions>,
    pub(in crate) buffers     : HashMap<BufferID, BufferOptions>,
    pub(in crate) attachments : HashMap<AttachmentID, AttachmentOptions>,
}

impl Pass {
    pub fn new(name : &'static str,) -> Self {
        Self {
            name,
            id : PassID(usize::MAX),

            resource_names : Default::default(),
            command_emitter : None,

            textures    : Default::default(),
            buffers     : Default::default(),
            attachments : Default::default(),
        }
    }

    pub fn emitter(mut self, emitter : fn(&CommandBuffer)) -> Self {
        self.command_emitter = Some(emitter);
        self
    }

    /// Adds a texture to this pass.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this resource.
    /// * `resource` - The identifier of the [`Texture`] to add.
    /// * `options` - Options associated with the texture.
    ///
    /// # Panics
    ///
    /// * Panics if `name` is not unique for this pass.
    /// * Panics if `resource` does not end up referencing a [`Texture`].
    pub fn add_texture(mut self, name : &'static str, resource : &ResourceID, options : TextureOptions) -> Self
    {
        let physical_texture = resource.devirtualize();
        if let PhysicalResourceID::Texture(texture) = physical_texture {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());

            self.textures.insert(*texture, options);
            self
        } else {
            panic!("The provided resource identifier is not a texture")
        }
    }

    /// Adds a buffer to this pass.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this resource.
    /// * `resource` - The identifier of the [`Buffer`] to add.
    /// * `options` - Options associated with the buffer.
    ///
    /// # Panics
    ///
    /// * Panics if `name` is not unique for this pass.
    /// * Panics if `resource` does not end up referencing a [`Buffer`].
    pub fn add_buffer(mut self, name : &'static str, resource : ResourceID, options : BufferOptions) -> Self {
        let physical_texture = resource.devirtualize();
        if let PhysicalResourceID::Buffer(buffer) = physical_texture {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());
            
            self.buffers.insert(*buffer, options);
            self
        } else {
            panic!("The provided resource identifier is not a buffer")
        }
    }

    /// Adds an attachment to this pass.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this resource.
    /// * `resource` - The identifier of the [`Attachment`] to add.
    /// * `access_flags` - A set of flags indicating how this attachment is used.
    /// * `options` - Options associated with the attachment.
    /// 
    /// # Panics
    ///
    /// * Panics if `name` is not unique for this pass.
    /// * Panics if `resource` does not end up referencing a [`Attachment`].
    pub fn add_attachment(mut self, name : &'static str, resource : &ResourceID, options : AttachmentOptions) -> Self {
        let physical_texture = resource.devirtualize();
        if let PhysicalResourceID::Attachment(attachment) = physical_texture {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());

            self.attachments.insert(*attachment, options);
            self
        } else {
            panic!("The provided resource identifier is not an attachment")
        }
    }

    /// Registers this pass on the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph on which to register.
    pub fn register<'a>(self, graph : &mut Graph) -> PassID {
        let registered_self = graph.passes.register(self, |instance, id| instance.id = PassID(id));

        assert_ne!(registered_self.id(), PassID(usize::MAX));

        registered_self.id()
    }

    /// Returns the resource ID of the texture with the given name. If the name exists but is not
    /// a texture, returns [`None`].
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the texture.
    pub fn texture(&self, name : &'static str) -> Option<&ResourceID> {
        return self.resource_names.get(name).filter(|resource| {
            if let PhysicalResourceID::Texture(_) = resource.devirtualize() {
                true
            } else {
                false
            }
        })
    }

    /// Returns the resource ID of the buffer with the given name. If the name exists but is not
    /// a buffer, returns [`None`].
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the buffer.
    pub fn buffer(&self, name : &'static str) -> Option<&ResourceID> {
        return self.resource_names.get(name).filter(|resource| {
            if let PhysicalResourceID::Buffer(_) = resource.devirtualize() {
                true
            } else {
                false
            }
        })
    }

    /// Returns the resource ID of the attachment with the given name. If the name exists but is not
    /// an attachment, returns [`None`].
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attachment.
    pub fn attachment(&self, name : &'static str) -> Option<&ResourceID> {
        return self.resource_names.get(name).filter(|resource| {
            if let PhysicalResourceID::Attachment(_) = resource.devirtualize() {
                true
            } else {
                false
            }
        })
    }

    pub(in crate) fn inputs(&self) -> impl Iterator<Item = &ResourceID> {
        self.get_resources(ResourceAccessFlags::Read)
    }
    
    pub(in crate) fn outputs(&self) -> impl Iterator<Item = &ResourceID> {
        self.get_resources(ResourceAccessFlags::Write)
    }

    pub(in crate) fn resources(&self) -> impl Iterator<Item = &ResourceID> {
        self.resource_names.values()
    }
    
    fn get_resources(&self, flags : ResourceAccessFlags) -> impl Iterator<Item = &ResourceID> {
        self.resource_names.values().filter(move |resource_name| {
            let physical_resource = resource_name.devirtualize();
            let options : &dyn ResourceOptions = match physical_resource {
                PhysicalResourceID::Texture(texture) => self.textures.get(texture).unwrap(),
                PhysicalResourceID::Buffer(buffer) => self.buffers.get(buffer).unwrap(),
                PhysicalResourceID::Attachment(attachment) => self.attachments.get(attachment).unwrap()
            };

            options.access_flags().contains(flags)
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct PassID(usize);

impl PassID {
    pub fn get<'a>(&self, graph : &'a Graph) -> &'a Pass {
        graph.passes.find(*self).unwrap()
    }
}

impl Into<Identifier> for PassID {
    fn into(self) -> Identifier { Identifier::Numeric(self.0) }
}

impl Identifiable for Pass {
    type IdentifierType = PassID;

    fn id(&self) -> Self::IdentifierType { self.id }
    fn name(&self) -> &'static str { self.name }
}

impl nohash_hasher::IsEnabled for PassID { }