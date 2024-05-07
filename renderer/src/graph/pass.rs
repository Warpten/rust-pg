use std::{collections::HashMap, hint};

use super::{manager::{Identifiable, Identifier}, resource::{Resource, ResourceUsage, TextureUsage}};

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
    }
}

impl Identifiable for Pass {
    fn name(&self) -> &'static str { self.name }
    fn id(&self) -> usize { self.id }
}