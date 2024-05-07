
use std::{collections::HashMap, marker::PhantomData};

use super::{manager::{Identifiable, Identifier, Manager}, pass::Pass, resource::{Buffer, Resource, Texture}};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    passes : Manager<Pass>,
    ressources : Manager<Resource>,
    // synchronizations : Manager<Synchronization>,
    // sequences : Manager<Sequencing>,
}

type SequencedValue<T> = Vec<(usize, T)>;

impl Graph {
    /// Creates a new render graph.
    pub fn new() -> Self {
        Self {
            passes : Manager::new(Pass::new),
            ressources : Manager::new(|_, _| unimplemented!("You should use deferred texture creation instead")),
        }
    }

    pub fn build(&self) {
        // Panic if the graph is insane
        self.passes.iter().for_each(|pass| pass.validate());

        // 1. Find the backbuffer.
        //    Make sure at least one pass writes to it.
        let backbuffer = self.find_resource("builtin://backbuffer".into()).unwrap();

        // 2. Find writers to the backbuffer 
        let writers = backbuffer.writers(false);
        assert_eq!(writers.is_empty(), false, "No pass writes to the backbuffer");
        
        // 3. Find the last writer to the backbuffer.
        // TODO: This will superbly fail if we have non-graphics passes **after** the last write to the framebuffer
        //       ... why can this ever happen?
        let tree_root = writers.iter()
            .cloned()
            .filter_map(|identifier| self.find_pass(identifier))
            .find(|writer| writer.executes_before().is_empty());

        match tree_root {
            Some(tree_root) => self.build_graph(tree_root),
            None => panic!()
        }

        // 2. Traverse the tree bottom-up
        //    It's too late for my brain to function so here goes.
        //    https://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        //    https://blog.traverseresearch.nl/render-graph-101-f42646255636
    }

    fn build_graph(&self, root : &Pass) {
        // Welcome to madness, exhibit B (exhibit A was this whole repository)

        // Because working with trees in Rust is a bit unwiedly (or so I've heard...), I've decided to instead
        // encode the tree as a sequence of integer offsets into a vector. While this works for one-on-one links,
        // this runs into a bit of a situation for many-to-one.
        // I thought about using a 0 offset dummy element, but that doesn't immediately jump out to me as "it works",
        // so for now, all you get is my rambling.

        // Collect parent passes, in any order
        let parents : Vec<&Pass> = root.executes_after().iter()
            .cloned()
            .filter_map(|identifier| self.find_pass(identifier))
            .collect::<Vec<_>>();

        let mut texture_history = History::<Texture, TextureLayout>::new();
        self.build_texture_history(root, &mut texture_history);
    }

    fn build_texture_history(&self, root : &Pass, texture_history : &mut History<Texture, TextureLayout>) {
        for (&identifier, &usage) in root.resources() {
            let texture = unsafe {
                self.find_texture(identifier.into()).unwrap_unchecked()
            };
        }
    }
    
    /// Registers a new rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this pass.
    pub fn register_pass(&mut self, name : &'static str) -> &mut Pass {
        self.passes.register(name)
    }

    /// Registers a new texture.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this texture.
    pub fn register_texture(&mut self, name : &'static str) -> &mut Resource {
        self.ressources.register_deferred(name, |name, id| Resource::Texture(Texture::new(name, id, 1, 1)))
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for the pass.
    pub fn find_pass(&self, identifier : Identifier<Pass>) -> Option<&Pass> {
        self.passes.find(identifier)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for that resource.
    pub fn find_resource(&self, identifier : Identifier<Resource>) -> Option<&Resource> {
        self.ressources.find(identifier)
    }

    /// Returns a registered texture, given an uniquely identifying name.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn find_texture(&self, identifier : Identifier<Resource>) -> Option<&Texture> {
        self.find_resource(identifier).and_then(|resource| {
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
    pub fn find_buffer(&self, identifier : Identifier<Resource>) -> Option<&Buffer> {
        self.find_resource(identifier).and_then(|resource| {
            match resource {
                Resource::Buffer(buffer) => Some(buffer),
                _ => None
            }
        })
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for the pass.
    pub fn find_pass_mut(&mut self, identifier : Identifier<Pass>) -> Option<&mut Pass> {
        self.passes.find_mut(identifier)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for that resource.
    pub fn find_resource_mut(&mut self, identifier : Identifier<Resource>) -> Option<&mut Resource> {
        self.ressources.find_mut(identifier)
    }

    /// Returns a registered texture, given an uniquely identifying name.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn find_texture_mut(&mut self, identifier : Identifier<Resource>) -> Option<&mut Texture> {
        self.find_resource_mut(identifier).and_then(|resource| {
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
    pub fn find_buffer_mut(&mut self, identifier : Identifier<Resource>) -> Option<&mut Buffer> {
        self.find_resource_mut(identifier).and_then(|resource| {
            match resource {
                Resource::Buffer(buffer) => Some(buffer),
                _ => None
            }
        })
    }
}

/// Stores history for a set of objects
struct History<T : Identifiable, V> {
    values : HashMap<usize /* resource identifier */, Vec<(usize /* pass identifier */, V /* value for pass */)>>,
    _marker : PhantomData<(T, V)>,
}

impl<T : Identifiable, V> History<T, V> {
    pub fn new() -> Self {
        Self { values : HashMap::new(), _marker : PhantomData::default() }
    }

    pub fn register(&mut self, resource : &T, pass : &Pass) {
        match self.values.get_mut(&resource.id()) {
            Some(value) => {
                value.push((pass.id(), pass.usage_of(resource)))
            },
            None => {
                self.values.insert(resource.id(), vec![(pass.id(), pass.usage_of(resource))]);
            }
        }
    }
}