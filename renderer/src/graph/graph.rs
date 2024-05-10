use std::hint;

use super::{buffer::{Buffer, BufferID}, manager::{Identifier, Manager}, pass::{Pass, PassID}, resource::{Resource, ResourceID}, texture::{Texture, TextureID}};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    pub(in super) passes : Manager<Pass>,
    pub(in super) resources : Manager<Resource>,
    backbuffer : TextureID,
}

impl Graph {
    /// Creates a new render graph.
    pub fn new() -> Self {
        Self {
            passes : Manager::default(),
            resources : Manager::default(),
            backbuffer : TextureID(Identifier::None),
        }
    }

    pub fn backbuffer(&self) -> Option<&Texture> { self.find_texture(self.backbuffer) }

    pub fn build(&self) {
        assert_ne!(self.backbuffer, TextureID(Identifier::None), "No backbuffer declared for this graph");
        
        let texture = self.backbuffer().unwrap();

        texture.writers().iter().for_each(|&pass| {
            let pass = self.find_pass(pass);
        })

        // 2. Traverse the tree bottom-up
        //    It's too late for my brain to function so here goes.
        //    https://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        //    https://blog.traverseresearch.nl/render-graph-101-f42646255636
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for the pass.
    pub fn find_pass(&self, identifier : PassID) -> Option<&Pass> {
        self.passes.find(identifier)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for that resource.
    pub fn find_resource(&self, identifier : ResourceID) -> Option<&Resource> {
        self.resources.find(identifier)
    }

    /// Returns a registered texture, given an uniquely identifying name.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn find_texture(&self, identifier : TextureID) -> Option<&Texture> {
        self.find_resource(identifier.to_resource()).and_then(|resource| {
            match resource {
                Resource::Texture(value) => Some(value),
                _ => None
            }
        })
    }

    /// Returns a registered resource, given its ID.
    /// If no buffer with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that buffer in this graph.
    pub fn find_buffer(&self, identifier : &BufferID) -> Option<&Buffer> {
        self.find_resource(identifier.to_resource()).and_then(|resource| {
            match resource {
                Resource::Buffer(buffer) => Some(buffer),
                _ => None
            }
        })
    }

    pub fn reset(&mut self) {
        self.passes.clear();
        self.resources.clear();
    }
}
