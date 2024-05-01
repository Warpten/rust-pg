use std::collections::HashMap;

use super::{pass::Pass, resource::{Buffer, Resource, Texture}, Sequencing, Synchronization};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    passes : ObjectManager<Pass>,
    ressources : ObjectManager<Resource>,
    synchronizations : ObjectManager<Synchronization>,
    sequences : ObjectManager<Sequencing>,
}

impl Graph {
    pub fn build() {
        // Builds the entire command buffer.
        todo!();
    }

    pub fn invalidate() {
        // Invalidates any command buffer previously built and reuploads it to the GPU
        todo!();
    }

    pub fn dispatch() {
        // Submits the built command buffer to the device.
        todo!();
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
        self.synchronizations.register(name, |_, _| Synchronization::new(stages, passes));
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
        self.sequences.register(name, |_, _| Sequencing::new(stages, first, second));
    }

    /// Registers a new rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying this pass.
    pub fn register_pass(&mut self, name : &'static str) -> &Pass {
        self.passes.register(name, |id, name| Pass::new(id, name))
    }

    /// Finds a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique name identifying the pass to find.
    pub fn find_pass(&self, name : &'static str) -> Option<&Pass> {
        self.passes.find(name)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn get_texture_resource(&self, name : &'static str) -> Option<&Texture> {
        match self.ressources.find(name) {
            Some(resource) => {
                match resource {
                    Resource::Texture(texture) => Some(texture),
                    _ => panic!("This resource is not a texture")
                }
            }
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
                    Resource::Buffer(buffer) => Some(buffer),
                    _ => panic!("This resource is not a buffer")
                }
            }
            None => None
        }
    }
}

struct ObjectManager<T> {
    instances : Vec<T>,
    index_map : HashMap<&'static str, usize>,
}

impl<T> ObjectManager<T> {
    pub fn register_instance(&mut self, name : &'static str, instance : T) -> &T {
        let index = self.index_map.get(name);
        assert!(index.is_none(), "An object with this name already exists");

        self.instances.push(instance);
        &self.instances[*index.unwrap()]
    }

    pub fn register<Factory>(&mut self, name : &'static str, instancer : Factory) -> &T
        where Factory : Fn(usize, &'static str) -> T
    {
        match self.index_map.get(name) {
            Some(&index) => {
                &self.instances[index]
            },
            None => {
                let index = self.instances.len();
                self.instances.push(instancer(index, name));
                self.index_map.insert(name, index);
                
                &self.instances[index]
            }
        }
    }

    pub fn find(&self, name : &'static str) -> Option<&T> {
        self.index_map.get(name).and_then(|&index| self.instances.get(index))
    }

    pub fn reset(&mut self) {
        self.instances.clear();
        self.index_map.clear();
    }
}
