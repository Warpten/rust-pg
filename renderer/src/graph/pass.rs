use std::collections::HashMap;

use super::{manager::{Identifiable, Identifier}, resource::{Resource, ResourceAccessFlags}};

pub struct Pass {
    name : &'static str,
    id : usize,

    resources : HashMap<usize, ResourceUsage>,
    executes_before : Vec<usize>,
    executes_after : Vec<usize>,
}

impl Pass {
    pub fn new(name : &'static str, id : usize) -> Self {
        Self {
            name,
            id,

            resources : HashMap::new(),
            executes_after : vec![],
            executes_before : vec![]
        }
    }

    pub fn validate(&self) { }

    /// Returns usage flags of a given resource for this pass.
    /// 
    /// # Arguments
    /// 
    /// * `resource` - The resource this pass may potentially use.
    pub fn uses(&self, resource : &Resource) -> Option<ResourceAccessFlags> {
        self.resources.get(&resource.id()).copied()
    }

    pub fn resources(&self) -> &HashMap<usize, ResourceAccessFlags> {
        &self.resources
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
    }
}

impl Identifiable for Pass {
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> usize { self.id }
}