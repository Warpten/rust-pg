

use bitmask_enum::bitmask;

use super::resource::{Buffer, Texture};

pub struct Pass {
    name : &'static str,
    // Index of this pass in the graph. Unrelated to execution order.
    index : usize,
    color : InputOutput<Texture>,
    storage_texture : InputOutput<Texture>,
    blits : InputOutput<Texture>,
    storage : InputOutput<Buffer>,
}

impl Pass {
    pub fn new(index : usize, name : &'static str) -> Self {
        Self {
            name,
            index,
            color : InputOutput::new(),
            storage_texture : InputOutput::new(),
            blits : InputOutput::new(),
            storage : InputOutput::new(),
        }
    }

    pub fn name(&self) -> &'static str { &self.name }
    pub fn index(&self) -> usize { self.index }

    pub fn add_color_output(&mut self, resource : &Texture) {
        self.color.add(ResourceAccessFlags::Write, resource);
    }
}

// Ressources =====

pub struct AccessedResource<T> {
    stages : ash::vk::PipelineStageFlags2,
    access : ash::vk::AccessFlags2,
    resource : Box<T>
}

impl<T> AccessedResource<T> {
    pub fn stages(&self) -> ash::vk::PipelineStageFlags2 { self.stages }
    pub fn access(&self) -> ash::vk::AccessFlags2 { self.access }
    pub fn resource(&self) -> &T { &self.resource }
}

pub struct InputOutput<T> {
    inputs : Vec<AccessedResource<T>>,
    outputs : Vec<AccessedResource<T>>,
}

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read,
    Write
}

impl<T> InputOutput<T> {
    pub fn new() -> Self {
        Self { inputs : vec![], outputs : vec![] }
    }

    pub fn add(&mut self, access : ResourceAccessFlags, instance : AccessedResource<T>) {
        if (access & ResourceAccessFlags::Read) != 0 {
            self.inputs.push(instance);
        }

        if (access & ResourceAccessFlags::Write) != 0 {
            self.outputs.push(instance);
        }
    }

    pub fn inputs(&self) -> &[AccessedResource<T>] {
        &self.inputs[..]
    }

    pub fn outputs(&self) -> &[AccessedResource<T>] {
        &self.outputs[..]
    }
}
