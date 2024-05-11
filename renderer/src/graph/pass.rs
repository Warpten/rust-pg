use std::collections::HashMap;

use super::{manager::{Identifiable, Identifier}, resource::ResourceID, Graph};

pub struct Pass {
    pub(in self) id : PassID,
    name : &'static str,

    pub(in self) inputs : HashMap<&'static str, ResourceID /* remote */>,
    pub(in self) outputs : HashMap<&'static str, ResourceID /* remote */>,
}

impl Pass {
    pub fn new(name : &'static str) -> Pass {
        Self {
            name,
            id : PassID(usize::MAX),

            inputs : HashMap::new(),
            outputs : HashMap::new(),
        }
    }

    pub fn name(&self) -> &'static str { self.name }

    /// Adds an input to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of this input. Locally to the pass, this name must be unique.
    /// * `resource` - A [`ResourceID`] identifying the input [`Resource`].
    #[inline]
    pub fn add_input(mut self, name : &'static str, resource : ResourceID) -> Self {
        self.inputs.insert(name, resource);
        self
    }
    
    /// Adds an output to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of this input. Locally to the pass, this name must be unique.
    /// * `resource` - A [`ResourceID`] identifying the input [`Resource`].
    #[inline]
    pub fn add_output(mut self, name : &'static str, resource : ResourceID) -> Self {
        self.inputs.insert(name, resource);
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

        for (_, input_id) in &registered_self.inputs {
            match manager.resources.find_mut(input_id.clone()) {
                Some(resource) => resource.register_reader(registered_self.id),
                None => panic!("Inconsistent state"),
            }
        }

        for (_, input_id) in &registered_self.outputs {
            match manager.resources.find_mut(input_id.clone()) {
                Some(resource) => resource.register_writer(registered_self.id),
                None => panic!("Inconsistent state"),
            }
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
        match self.inputs.get(name) {
            None => ResourceID::None,
            Some(value) => ResourceID::Virtual(self.id(), Box::new(value.clone())),
        }
    }

    /// Returns the [`ResourceID`] of an output identified by its name .The returned resource id is
    /// local to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the input resource. Locally to the pass, this name must be unique.
    pub fn output(&self, name : &'static str,) -> ResourceID {
        match self.outputs.get(name) {
            None => ResourceID::None,
            Some(value) => ResourceID::Virtual(self.id(), Box::new(value.clone())),
        }
    }

    /// Returns all the inputs of this pass.
    pub fn inputs(&self) -> Vec<&ResourceID> { self.inputs.values().collect::<Vec<_>>() }

    /// Returns all the outputs of this pass.
    pub fn outputs(&self) -> Vec<ResourceID> {
        self.outputs.values().map(|resource| {
            ResourceID::Virtual(self.id(), Box::new(resource.clone()))
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
