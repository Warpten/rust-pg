use std::collections::HashMap;
use crate::graph::attachment::{AttachmentID, AttachmentOptions};
use crate::graph::buffer::{BufferID, BufferOptions};
use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::resource::{Identifiable, ResourceID, ResourceAccessFlags};
use crate::graph::texture::{TextureID, TextureOptions};

pub struct Pass {
    id : PassID,
    name : &'static str,

    resource_names : HashMap<&'static str, ResourceID>,

    pub(in crate) textures    : HashMap<TextureID, TextureOptions>,
    pub(in crate) buffers     : HashMap<BufferID, BufferOptions>,
    pub(in crate) attachments : HashMap<AttachmentID, AttachmentOptions>,

    resources   : HashMap<ResourceID, ResourceAccessFlags>,
}

impl Pass {
    pub fn new(name : &'static str,) -> Self {
        Self {
            name,
            id : PassID(usize::MAX),

            resource_names : Default::default(),

            resources   : Default::default(),
            textures    : Default::default(),
            buffers     : Default::default(),
            attachments : Default::default(),
        }
    }

    /// Adds a texture to this pass.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this resource.
    /// * `resource` - The identifier of the [`Texture`] to add.
    /// * `access_flags` - A set of flags indicating how this texture is used.
    /// * `options` - Options associated with the texture.
    ///
    /// # Panics
    ///
    /// * Panics if `name` is not unique for this pass.
    /// * Panics if `resource` does not end up referencing a [`Texture`].
    pub fn add_texture(mut self, name : &'static str, resource : &ResourceID, access_flags : ResourceAccessFlags, options : TextureOptions) -> Self
    {
        if let ResourceID::Texture(texture) = resource.devirtualize() {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());
            self.resources.insert(resource.clone(), access_flags);

            self.textures.insert(*texture, options);
            self
        } else {
            panic!("The provided resource identifier is not a resource")
        }
    }

    /// Adds a buffer to this pass.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this resource.
    /// * `resource` - The identifier of the [`Buffer`] to add.
    /// * `access_flags` - A set of flags indicating how this buffer is used.
    /// * `options` - Options associated with the buffer.
    ///
    /// # Panics
    ///
    /// * Panics if `name` is not unique for this pass.
    /// * Panics if `resource` does not end up referencing a [`Buffer`].
    pub fn add_buffer(mut self, name : &'static str, resource : ResourceID, access_flags : ResourceAccessFlags, options : BufferOptions) -> Self {
        if let ResourceID::Buffer(buffer) = resource.devirtualize() {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());
            self.resources.insert(resource.clone(), access_flags);
            
            self.buffers.insert(*buffer, options);
            self
        } else {
            panic!("The provided resource identifier is not a resource")
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
    pub fn add_attachment(mut self, name : &'static str, resource : &ResourceID, access_flags : ResourceAccessFlags, options : AttachmentOptions) -> Self {
        if let ResourceID::Attachment(attachment) = resource.devirtualize() {
            assert!(!self.resource_names.contains_key(name), "A resource with this name already exists");

            self.resource_names.insert(name, resource.clone());
            self.resources.insert(resource.clone(), access_flags);

            self.attachments.insert(*attachment, options);
            self
        } else {
            panic!("The provided resource identifier is not a resource")
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
            if let ResourceID::Texture(_) = resource.devirtualize() {
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
            if let ResourceID::Buffer(_) = resource.devirtualize() {
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
            if let ResourceID::Attachment(_) = resource.devirtualize() {
                true
            } else {
                false
            }
        })
    }

    pub(in crate) fn inputs(&self) -> Vec<&ResourceID> {
        self.resources.iter().filter(|(res, flags)| flags.contains(ResourceAccessFlags::Read))
            .map(|(k, v)| k)
            .collect()
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