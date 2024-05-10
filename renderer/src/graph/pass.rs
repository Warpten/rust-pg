use std::collections::HashMap;

use super::{manager::{Identifiable, Identifier}, resource::ResourceID, Graph};

pub struct Pass {
    pub(in self) id : PassID,
    name : &'static str,

    pub(in self) inputs : HashMap<&'static str, ResourceID /* local alias */>,
    pub(in self) outputs : HashMap<&'static str, ResourceID /* local alias*/>,
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
        self.inputs.insert(name, ResourceID::Virtual(self.id, Box::new(resource)));
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
        self.inputs.insert(name, ResourceID::Virtual(self.id, Box::new(resource)));
        self
    }

    /// Finalizes this pass and registers it on a graph. The object is moved from this call and no longer accessible.
    /// 
    /// # Arguments
    /// 
    /// * `graph` - The graph that will take ownership of this pass.
    #[inline]
    pub fn register(self, manager : &mut Graph) -> PassID {
        for (_, input_id) in &self.inputs {
            match manager.resources.find_mut(input_id.clone()) {
                Some(resource) => resource.register_reader(self.id),
                None => panic!("Inconsistent state")
            };
        }

        for (_, output_id) in &self.outputs {
            match manager.resources.find_mut(output_id.clone()) {
                Some(resource) => resource.register_writer(self.id),
                None => panic!("Inconsistent state")
            };
        }

        manager.passes.register(self, |instance, id| instance.id = PassID(id))
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
            Some(value) => value.clone(),
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
            Some(value) => value.clone(), // See the documentation on ResourceID to understand why this clone call is necessary
        }
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
    /// Retrieves the actual pass from the graph.
    pub fn get(self, graph : &Graph) -> Option<&Pass> { graph.find_pass(self) }
    
    pub fn input(&self, graph : &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.input(name)).unwrap_or(ResourceID::None)
    }

    pub fn output(&self, graph: &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.output(name)).unwrap_or(ResourceID::None)
    }
}

impl From<PassID> for Identifier {
    fn from(value: PassID) -> Self {
        Identifier::Numeric(value.0)
    }
}
