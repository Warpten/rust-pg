use std::collections::HashMap;

use ash::vk::Queue;

use crate::QueueAffinity;

use super::{manager::{Identifiable, Identifier}, resource::{Resource, ResourceID, ResourceUsage}, Graph};

pub struct Pass {
    pub(in self) id : PassID,
    name : &'static str,
    affinity : QueueAffinity,

    resources : HashMap<&'static str, (ResourceID, ResourceUsage)>,
}

impl Pass {
    pub fn new(name : &'static str) -> Pass {
        Self {
            name,
            id : PassID(usize::MAX),
            affinity : QueueAffinity::none(),

            resources : HashMap::new(),
        }
    }

    pub fn name(&self) -> &'static str { self.name }

    pub(in crate) fn affinity(&self) -> QueueAffinity { self.affinity }

    /// Adds a resource to this pass.
    /// 
    /// # Example
    /// 
    /// The following code works fine and pass B will correctly refer to **backbuffer_resource** after pass A is done executing.
    /// ```
    /// pub fn register_pass(&self, graph : &mut Graph, backbuffer_resource : ResourceID) {
    ///     let a = Pass::new("A")
    ///         .add_resource("Backbuffer", backbuffer_resource, ResourceUsage::ReadWrite)
    ///         .register(graph);
    /// 
    ///     let b = Pass::new("B")
    ///         .add_input("Backbuffer", a.output("Backbuffer"))
    ///         .register(graph);
    /// }
    /// ```
    /// 
    /// This one has pass A read from a resource, write to another one, and has B read from the latter resource.
    /// ```
    /// pub fn register_pass(&self, graph : &mut Graph, backbuffer_resource : ResourceID, some_unrelated_resource : ResourceID) {
    ///     let a = Pass::new("A")
    ///         .add_resource("Backbuffer", backbuffer_resource, ResourceUsage::ReadOnly)
    ///         .add_resource("Backbuffer", some_unrelated_resource, ResourceUsage::WriteOnly)
    ///         .register(graph);
    /// 
    ///     let b = Pass::new("B")
    ///         .add_resource("Backbuffer", a.output("Backbuffer")) 
    ///         .register(graph);
    /// }
    /// ```
    /// 
    /// Users are heavily encouraged to name their resources with naming conventions that allow them to differ between read-only
    /// resources (such as "BackbufferRead"), write-only resources (such as "BackbufferWrite"), and read-write resources (such as
    /// "BackBuffer"), to minimize confusion.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of this input. Locally to the pass, this name must be unique.
    /// * `resource` - A [`ResourceID`] identifying the input resource.
    /// * `usage` - An access mask for the resource.
    pub fn add_resource(mut self, name : &'static str, resource : ResourceID, usage : ResourceUsage) -> Self {
        self.resources.insert(name, (resource, usage));
        self
    }

    /// Finalizes this pass and registers it on a graph. The object is moved from this call and no longer accessible.
    /// 
    /// # Arguments
    /// 
    /// * `graph` - The graph that will take ownership of this pass.
    #[inline]
    pub fn register(self, manager : &mut Graph) -> PassID {
        let registered_self = manager.passes.register(self, |instance, id| instance.id = PassID(id));

        for (resource_id, usage) in registered_self.resources.values() {
            match manager.resources.find_mut(resource_id.clone()) {
                Some(resource) => {
                    match usage {
                        ResourceUsage::ReadOnly => resource.register_reader(registered_self.id),
                        ResourceUsage::WriteOnly => resource.register_writer(registered_self.id),
                        ResourceUsage::ReadWrite => {
                            resource.register_reader(registered_self.id);
                            resource.register_writer(registered_self.id);
                        },
                    }
                },
                None => panic!("Inconsistent state"),
            };
        }

        registered_self.id()
    }

    /// Returns the [`ResourceID`] of an input identified by its name. The returned resource id is
    /// local to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the input resource. Locally to the pass, this name must be unique.
    pub fn input(&self, name : &'static str) -> ResourceID {
        match self.resources.get(name) {
            Some((resource, usage)) if *usage == ResourceUsage::ReadOnly || *usage == ResourceUsage::ReadWrite
                => ResourceID::Virtual(self.id(), Box::new(resource.clone())),
            _ => ResourceID::None,
        }
    }

    /// Returns the [`ResourceID`] of an output identified by its name .The returned resource id is
    /// local to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the input resource. Locally to the pass, this name must be unique.
    pub fn output(&self, name : &'static str) -> ResourceID {
        match self.resources.get(name) {
            Some((resource, usage)) if *usage == ResourceUsage::WriteOnly || *usage == ResourceUsage::ReadWrite
                => ResourceID::Virtual(self.id(), Box::new(resource.clone())),
            _ => ResourceID::None,
        }
    }

    /// Returns all the inputs of this pass.
    pub fn inputs(&self) -> Vec<ResourceID> {
        self.resources.values().filter_map(|(resource, usage)| {
            match usage {
                ResourceUsage::WriteOnly => None,
                _ => Some(resource.clone()),
            }
        }).collect::<Vec<_>>()
    }

    /// Returns all the outputs of this pass.
    pub fn outputs(&self) -> Vec<ResourceID> {
        self.resources.values().filter_map(|(resource, usage)| {
            match usage {
                ResourceUsage::WriteOnly => None,
                _ => Some(ResourceID::Virtual(self.id, Box::new(resource.clone()))),
            }
        }).collect::<Vec<_>>()
    }
    
    pub fn validate(&self) { }
}

impl Identifiable for Pass {
    type Key = PassID;
    
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> Self::Key { self.id }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct PassID(usize);

impl PassID {
    pub const NONE : PassID = PassID(usize::MAX);

    pub fn raw(&self) -> usize { self.0 }

    /// Retrieves the actual pass from the graph.
    pub fn get(self, graph : &Graph) -> Option<&Pass> { graph.find_pass(self) }
    
    pub fn input(&self, graph : &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.input(name)).unwrap_or(ResourceID::None)
    }

    pub fn output(&self, graph: &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.output(name)).unwrap_or(ResourceID::None)
    }

    #[deprecated = "May be removed, usage is absurd"]
    pub fn sequencing_point(&self) -> ResourceID {
        ResourceID::Virtual(self.clone(), Box::new(ResourceID::None))
    }
}

impl nohash_hasher::IsEnabled for PassID { }

impl From<PassID> for Identifier {
    fn from(value: PassID) -> Self {
        Identifier::Numeric(value.0)
    }
}
