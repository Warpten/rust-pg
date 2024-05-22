use crate::graph::Graph;
use crate::graph::manager::Identifier;
use crate::graph::pass::Pass;
use crate::graph::resource::{Identifiable, PhysicalResourceID, ResourceAccessFlags, ResourceID, ResourceOptions};

pub struct Texture {
    id   : TextureID,
    name : &'static str,

    layout : ash::vk::ImageLayout,
    levels : u32,
    layers : u32,
    format : ash::vk::Format,
}

impl Texture {
    pub fn create_info(&self) -> ash::vk::ImageCreateInfo {
        ash::vk::ImageCreateInfo::default()
            .mip_levels(self.levels)
            .array_layers(self.layers)
            .format(self.format)
            .initial_layout(self.layout)
    }

    pub fn new(name : &'static str) -> Texture {
        Self {
            name,
            id : TextureID(usize::MAX),

            layout : ash::vk::ImageLayout::UNDEFINED,
            levels : 1,
            layers : 1,
            format : ash::vk::Format::UNDEFINED
        }
    }

    #[inline] pub fn layout(&self) -> ash::vk::ImageLayout { self.layout }
    #[inline] pub fn with_layout(mut self, layout : ash::vk::ImageLayout) -> Self {
        self.layout = layout;
        self
    }

    #[inline] pub fn levels(&self) -> u32 { self.levels }
    #[inline] pub fn with_levels(mut self, levels : u32) -> Self {
        self.levels = levels;
        self
    }

    #[inline] pub fn layers(&self) -> u32 { self.layers }
    #[inline] pub fn with_layers(mut self, layers : u32) -> Self {
        self.layers = layers;
        self
    }

    /// Registers this attachment on the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph on which to register.
    pub fn register(self, graph : &mut Graph) -> TextureID {
        let registered_self = graph.textures.register(self, |instance, id| instance.id = TextureID(id));

        assert_ne!(registered_self.id(), TextureID(usize::MAX));

        registered_self.id()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct TextureID(usize);

impl TextureID {
    /// Returns the actual texture associated with this ID in the given graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph in which to search for the texture identified by this ID.
    pub fn get<'a>(&self, graph : &'a Graph) -> Option<&'a Texture> {
        graph.textures.find(*self)
    }

    /// Returns the options of this texture in the given pass.
    ///
    /// # Arguments
    ///
    /// * `pass` - The pass in which to look for options.
    pub fn get_options<'a>(&self, pass : &'a Pass) -> Option<&'a TextureOptions> {
        pass.textures.get(self)
    }

    /// Returns a virtual resource ID associated with this texture and the given pass if
    /// said pass has this resource as input.
    ///
    /// # Arguments
    ///
    /// * `pass` - The pass in which to search.
    pub fn of_pass(&self, pass : &Pass) -> Option<ResourceID> {
        pass.resources().find(move |res| {
            if let ResourceID::Virtual(_, res) = res {
                if let PhysicalResourceID::Texture(tex) = res {
                    tex == self
                } else {
                    false
                }
            } else {
                false
            }
        }).cloned()
    }
}

impl Into<ResourceID> for TextureID {
    fn into(self) -> ResourceID { ResourceID::Physical(PhysicalResourceID::Texture(self)) }
}

impl Into<Identifier> for TextureID {
    fn into(self) -> Identifier { Identifier::Numeric(self.0) }
}

impl Default for TextureID {
    fn default() -> Self { Self(usize::MAX) }
}

impl Identifiable for Texture {
    type IdentifierType = TextureID;

    fn id(&self) -> Self::IdentifierType { self.id }
    fn name(&self) -> &'static str { self.name }
}

#[derive(Default)]
pub struct TextureOptions {
    pub usage_flags : ash::vk::ImageUsageFlags,
    pub layout : Option<ash::vk::ImageLayout>,
}

impl ResourceOptions for TextureOptions {
    fn access_flags(&self) -> ResourceAccessFlags {
        todo!()
    }
}
