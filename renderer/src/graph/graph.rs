
use super::{manager::Manager, pass::Pass, resource::{Buffer, Resource, Texture}};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    passes : Manager<Pass>,
    ressources : Manager<Resource>,
    // synchronizations : Manager<Synchronization>,
    // sequences : Manager<Sequencing>,
}

impl Graph {
    /// Creates a new render graph.
    pub fn new() -> Self {
        let mut this = Self {
            passes : Manager::new(),
            ressources : Manager::new(),
        };

        // this.register_texture("builtin://backbuffer");
        this
    }

    pub fn build(&mut self) {
        // Panic if the graph is insane
        // self.passes.for_each(|pass| pass.validate());

        // 1. Find the backbuffer.
        //    Make sure at least one pass writes to it.

        // 2. Traverse the tree bottom-up
        //    It's too late for my brain to function so here goes.
        //    https://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        //    https://blog.traverseresearch.nl/render-graph-101-f42646255636
        let backbuffer = self.get_texture("builtin://backbuffer").unwrap();

        // Begin by looking at all passes that write to the backbuffer
        let backbuffer_writers = backbuffer.writers(self, false);
        assert_eq!(backbuffer_writers.is_empty(), false, "No pass writes to the backbuffer");

        // backbuffer_writers.iter()
        //     .for_each(|&pass| self.traverse_dependencies(&pass, 0));

        // For now all our passes are in an array; we now want to group them into strands
        // (because of constraint::Sequencing) (where a strand is a sequence of passes in
        // a fixed order). We will then add external synchronization to individual passes
        // (because of constraint::Synchronization), possibly delaying to the next passes
        // in the strand to reduce the time spent explicitely waiting.
    }
    
    /// Registers a new rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this pass.
    pub fn register_pass(&mut self, instance : Pass) {
        self.passes.register(instance);
    }

    /// Registers a new texture.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this texture.
    pub fn register_texture(&mut self, texture : Texture) {
        self.ressources.register(Resource::Texture(texture));
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying the pass to find.
    pub fn find_pass(&self, name : &'static str) -> Option<&Pass> {
        self.passes.find(name)
    }

    /// Finds a rendering pass given its identifier.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The pass's identifier.
    pub fn find_pass_by_id(&self, id : usize) -> Option<&Pass> {
        self.passes.find_by_id(id)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that resource.
    pub fn get_resource(&self, name : &'static str) -> Option<&Resource> {
        self.ressources.find(name)
    }

    /// Returns a registered resource, given its ID in this graph.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID of that texture.
    pub fn get_resource_by_id(&self, id : usize) -> Option<&Resource> {
        self.ressources.find_by_id(id)
    }

    /// Returns a registered texture, given an uniquely identifying name.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn get_texture(&self, name : &'static str) -> Option<&Texture> {
        self.get_texture_impl(Graph::get_resource, name)
    }

    /// Returns a registered texture, given its ID.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The id of that texture in this graph.
    pub fn get_texture_by_id(&self, id : usize) -> Option<&Texture> {
        self.get_texture_impl(Graph::get_resource_by_id, id)
    }

    fn get_texture_impl<F, Arg>(&self, resource_supplier : F, arg : Arg) -> Option<&Texture>
        where F : Fn(&Self, Arg) -> Option<&Resource>
    {
        match resource_supplier(self, arg) {
            Some(resource) => {
                match resource {
                    Resource::Texture(value) => Some(value),
                    _ => None
                }
            },
            None => None
        }
    }

    /// Returns a registered resource, given its ID.
    /// If no buffer with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that buffer in this graph.
    pub fn get_buffer(&self, name : &'static str) -> Option<&Buffer> {
        self.get_buffer_impl(Graph::get_resource, name)
    }

    /// Returns a registered buffer, given its ID.
    /// If no buffer with that ID exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The id of that buffer in this graph.
    pub fn get_buffer_by_id(&self, id : usize) -> Option<&Buffer> {
        self.get_buffer_impl(Graph::get_resource_by_id, id)
    }

    fn get_buffer_impl<F, Arg>(&self, resource_supplier : F, arg : Arg) -> Option<&Buffer>
        where F : Fn(&Self, Arg) -> Option<&Resource>
    {
        match resource_supplier(self, arg) {
            Some(resource) => {
                match resource {
                    Resource::Buffer(value) => Some(value),
                    _ => None
                }
            },
            None => None
        }
    }

    /// Returns a registered buffer, given an uniquely identifying name.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that buffer.
    pub fn get_buffer_resource(&self, name : &'static str) -> Option<&Buffer> {
        match self.ressources.find(name) {
            Some(resource) => {
                match resource {
                    Resource::Buffer(value) => Some(value),
                    _ => None,
                }
            }
            None => None
        }
    }
}
