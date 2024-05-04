use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::utils::Manager;

use super::{pass::Pass, resource::{Buffer, Resource, Texture}, Sequencing, Synchronization};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    passes : Rc<Manager<Pass, Rc<Graph>>>,
    ressources : Rc<Manager<Resource, Rc<Graph>>>,
    synchronizations : Rc<Manager<Synchronization, Rc<Graph>>>,
    sequences : Rc<Manager<Sequencing, Rc<Graph>>>,
}

impl Graph {
    /// Creates a new render graph.
    pub fn new() -> Rc<Self> {
        let this = Rc::new(Self {
            passes : Manager::new(|_, name, id, graph| Pass::new(Rc::downgrade(graph), id, name)),
            ressources : Manager::new(|this, name, id, graph| todo!()),
            synchronizations : Manager::new(|this, name, id, graph| todo!()),
            sequences : Manager::new(|this, name, id, graph| todo!())
        });

        this.register_texture("builtin://backbuffer");
        this
    }

    pub fn build(&mut self) {
        // Panic if the graph is insane
        self.passes.for_each(|pass| pass.validate());

        // 1. Find the backbuffer.
        //    Make sure at least one pass writes to it.

        // 2. Traverse the tree bottom-up
        //    It's too late for my brain to function so here goes.
        //    https://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        //    https://blog.traverseresearch.nl/render-graph-101-f42646255636
        let backbuffer = self.get_texture("builtin://backbuffer").unwrap();

        // Begin by looking at all passes that write to the backbuffer
        let backbuffer_writers = backbuffer.writers(false).collect::<Vec<_>>();
        assert_eq!(backbuffer_writers.is_empty(), false, "No pass writes to the backbuffer");

        // backbuffer_writers.iter()
        //     .for_each(|&pass| self.traverse_dependencies(&pass, 0));

        // For now all our passes are in an array; we now want to group them into strands
        // (because of constraint::Sequencing) (where a strand is a sequence of passes in
        // a fixed order). We will then add external synchronization to individual passes
        // (because of constraint::Synchronization), possibly delaying to the next passes
        // in the strand to reduce the time spent explicitely waiting.
    }

    fn traverse_dependencies(&self, pass : &Pass, depth : usize) {
        assert!(depth < self.passes.len(), "Cyclic graph detected late");
    }

    /// Registers a synchronization directive between two render passes.
    /// Either pass will wait for the other before continuing.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the synchronization barrier.
    /// * `stags` - A bitmask of all the pipeline stages on which the passes should synchronize.
    /// * `passes` - The passes that should be externally synchronized.
    pub fn synchronize(
        &mut self,
        name : &'static str,
        stages : ash::vk::PipelineStageFlags2,
        passes : &[Pass])
    {
        // self.synchronizations.register(name, |_, _| Synchronization::new(stages, passes));
    }

    /// Registers a sequencing directive between two render passes.
    /// The second pass will not execute before the first one is done.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the synchronization barrier.
    /// * `stags` - A bitmask of all the pipeline stages on which the passes should synchronize.
    /// * `first` - The pass that must execute first.
    /// * `second` - The pass that must execute last.
    pub fn sequence(&mut self, name : &'static str, stages : ash::vk::PipelineStageFlags2, first : &Pass, second: &Pass) {
        // self.sequences.register(name, |_, _| Sequencing::new(stages, first, second));
    }

    // ^^^ Synchronization / Render passes vvv

    /// Registers a new rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this pass.
    pub fn register_pass(self : &mut Rc<Graph>, name : &'static str) -> Rc<Pass> {
        /*self.passes.register(
            name,
            |id, name| Pass::new(Rc::clone(self), id, name)
        )*/
        todo!()
    }

    /// Registers a new texture.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this texture.
    pub fn register_texture(self : &Rc<Graph>, name : &'static str) -> &Texture {
        /*let resource = self.ressources.register(name,
            |id, name| Resource::Texture { id, value : Texture::new(Rc::clone(self), id) });
        match Rc::as_ref(&resource) {
            Resource::Texture { id, value } => value,
            _ => panic!()
        }*/
        todo!()
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying the pass to find.
    pub fn find_pass(&self, name : &'static str) -> Option<Rc<Pass>> {
        self.passes.find(name)
    }

    /// Finds a rendering pass given its identifier.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The pass's identifier.
    pub fn find_pass_by_id(&self, id : usize) -> Option<Rc<Pass>> { self.passes.find_by_id(id) }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that resource.
    pub fn get_resource(&self, name : &'static str) -> Option<Rc<Resource>> {
        self.ressources.find(name)
    }

    /// Returns a registered resource, given its ID in this graph.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID of that texture.
    pub fn get_resource_by_id(&self, id : usize) -> Option<Rc<Resource>> {
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
        where F : Fn(&Self, Arg) -> Option<Rc<Resource>>
    {
        /*match resource_supplier(self, arg) {
            Some(resource) => {
                match Rc::as_ref(&resource) {
                    Resource::Texture { id, value } => Some(value),
                    _ => None
                }
            },
            None => None
        }*/
        todo!()
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
        where F : Fn(&Self, Arg) -> Option<Rc<Resource>>
    {
        /*match resource_supplier(self, arg) {
            Some(resource) => {
                match Rc::as_ref(&resource) {
                    Resource::Buffer { id, value } => Some(value),
                    _ => None
                }
            },
            None => None
        }*/
        todo!()
    }

    /// Returns a registered buffer, given an uniquely identifying name.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that buffer.
    pub fn get_buffer_resource(&self, name : &'static str) -> Option<&Buffer> {
        /*match self.ressources.find(name) {
            Some(resource) => {
                match resource.as_ref() {
                    Resource::Buffer { id, value } => Some(value),
                    _ => None,
                }
            }
            None => None
        }*/
        todo!()
    }
}
