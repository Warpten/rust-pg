use std::{collections::HashMap, marker::PhantomData};

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
    pub fn add_output(mut self, name : &'static str, resource : ResourceID) -> Self {
        self.inputs.insert(name, ResourceID::Virtual(self.id, Box::new(resource)));
        self
    }

    /// Finalizes this pass and registers it on a graph. The object is moved from this call and no longer accessible.
    /// 
    /// # Arguments
    /// 
    /// * `graph` - The graph that will take ownership of this pass.
    pub fn register(self, manager : &mut Graph) -> PassID {
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
/*
    /// Returns usage flags of a given resource for this pass.
    /// 
    /// # Arguments
    /// 
    /// * `resource` - The resource this pass may potentially use.
    pub fn resource_usage(&self, resource : &impl Identifiable) -> Option<&ResourceUsage> {
        self.resources.get(&resource.id())
    }

    pub unsafe fn texture_usage(&self, resource : &impl Identifiable) -> Option<&TextureUsage> {
        self.resource_usage(resource).map(|resource_usage| {
            match resource_usage {
                ResourceUsage::Texture(val) => val,
                _ => unsafe { hint::unreachable_unchecked() },
            }
        })
    }

    pub fn used_resources(&self) -> Vec<Identifier<Resource>> {
        self.resources.keys()
            .map(|&id| id.into())
            .collect::<Vec<_>>()
    }

    /// Adds a resource used as either an input or an output to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `resource` - The resource being used.
    pub fn use_resource(&mut self, resource : &mut Resource, usage : ResourceUsage) -> &mut Self {
        let id = resource.id();

        match (resource, &usage) {
            (Resource::Texture(texture), ResourceUsage::Texture(usage)) => {
                texture.add_user(self.id(), usage.access_flags);
            },
            (Resource::Buffer(buffer), ResourceUsage::Buffer(usage)) => {
                ();
            },
            (_, _) => panic!("Usage and resource types do not match")
        };
        self.resources.insert(id, usage);
        self
    }

    /// Links two passes together.
    /// 
    /// # Arguments
    /// 
    /// * `before` - The pass that must execute first.
    /// * `after` - The pass that must execute second.
    pub fn link(before : &mut Pass, after : &mut Pass) {
        before.executes_before.push(after.id());
        after.executes_after.push(before.id());
    }

    /// Returns identifiers of all the passes that execute before this pass.
    pub fn executes_after(&self) -> Vec<Identifier<Pass>> {
        self.executes_after.iter().map(|&id| id.into()).collect::<Vec<_>>()
    }
    
    /// Returns identifiers of all the passes that execute after this one.
    pub fn executes_before(&self) -> Vec<Identifier<Pass>> {
        self.executes_before.iter().map(|&id| id.into()).collect::<Vec<_>>()
    }*/
}

impl Identifiable for Pass {
    type Key = PassID;
    
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> Self::Key { self.id }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PassID(usize);

impl PassID {
    pub fn get(self, graph : &Graph) -> Option<&Pass> { graph.find_pass(self.into()) }
    
    pub fn input(&self, graph : &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.input(name)).unwrap_or(ResourceID::None)
    }

    pub fn output(&self, graph: &Graph, name : &'static str) -> ResourceID {
        self.get(graph).map(|pass| pass.output(name)).unwrap_or(ResourceID::None)
    }
}

impl Into<Identifier<PassID>> for PassID {
    fn into(self) -> Identifier<PassID> { Identifier::Numeric(self.0, PhantomData::default()) }
}
