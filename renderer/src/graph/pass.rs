

use std::{marker::PhantomData, rc::Rc};

use bitmask_enum::bitmask;

use super::{resource::{Resource, Texture}, Graph};

pub struct Pass {
    owner : Rc<Graph>,
    name : &'static str,
    // Index of this pass in the graph. Unrelated to execution order.
    index : usize,
    sequenced_from : Vec<usize>,
    sequences_to: Vec<usize>,

    // Index of all inputs and outputs for this pass in the associated graph's resources
    inputs : Vec<usize>,
    outputs : Vec<usize>
}

impl Pass {
    /// Creates a new render pass.
    /// 
    /// # Arguments
    /// 
    /// * `owner` - The graph owning this render pass.
    /// * `index` - The unique identifier of this pass.
    /// * `name` - The name of this render pass.
    pub fn new(owner : Rc<Graph>, index : usize, name : &'static str) -> Self {
        Self {
            owner,
            name,
            index,

            sequenced_from : vec![],
            sequences_to : vec![],

            inputs : vec![],
            outputs : vec![],
        }
    }

    /// Returns all passes that are explicitely sequenced before this pass.
    pub fn dependencies(&self) -> impl Iterator<Item = &Pass> {
        self.sequenced_from.iter()
            .filter_map(|&index| self.owner.find_pass_by_id(index))
    }

    /// Returns all passes that are explicitely sequenced after this pass.
    pub fn dependants(&self) -> impl Iterator<Item = &Pass> {
        self.sequenced_from.iter()
            .filter_map(|&index| self.owner.find_pass_by_id(index))
    }

    pub fn inputs(&self) -> impl Iterator<Item = &Resource> {
        self.inputs.iter().filter_map(|&index| {
            self.owner.get_resource_by_id(index)
        })
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Resource> {
        self.outputs.iter().filter_map(|&index| {
            self.owner.get_resource_by_id(index)
        })
    }

    pub fn sequence_to(&mut self, next : &Pass) {
        self.sequences_to.push(next.index());
    }

    pub fn sequence_from(&mut self, previous : &Pass) {
        self.sequenced_from.push(previous.index());
    }

    pub(super) fn build(&self, buffer : ash::vk::CommandBuffer) {

    }

    pub(super) fn validate(&self) {
        assert!(self.sequenced_from.is_empty() != self.sequences_to.is_empty());
    }

    pub fn name(&self) -> &'static str { &self.name }
    pub fn index(&self) -> usize { self.index }

    // vvv Buffer inputs

    pub fn vertex_buffer(&mut self, name : &'static str) -> &mut Self {
        self.buffer_input(name,
            ash::vk::PipelineStageFlags2::VERTEX_INPUT,
            ash::vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER
        )
    }

    pub fn index_buffer(&mut self, name : &'static str) -> &mut Self {
        self.buffer_input(name,
            ash::vk::PipelineStageFlags2::VERTEX_INPUT,
            ash::vk::AccessFlags2::INDEX_READ,
            ash::vk::BufferUsageFlags::INDEX_BUFFER
        )
    }

    pub fn indirect_buffer(&mut self, name : &'static str) -> &mut Self {
        self.buffer_input(name,
            ash::vk::PipelineStageFlags2::DRAW_INDIRECT,
            ash::vk::AccessFlags2::INDEX_READ,
            ash::vk::BufferUsageFlags::INDIRECT_BUFFER
        )
    }
    
    fn buffer_input(
        &mut self,
        name : &'static str,
        stages : ash::vk::PipelineStageFlags2,
        access : ash::vk::AccessFlags2, 
        usage : ash::vk::BufferUsageFlags
    ) -> &mut Self {
        self
    }

    // ^^^ Buffer inputs / Color attachments vvvv

    /// Adds a color attachment to this pass.
    /// 
    /// # Arguments
    /// 
    /// * `access_flags` - A bitmask of [`ResourceAccessFlags`] indicating RO, WO, or RWO.
    /// * `load_op` - The load operation to use.
    /// * `store_op` - The store operation to use.
    /// * `texture` - The texture being used as a color attachment.
    pub fn color_attachment(
        &mut self,
        access_flags : ResourceAccessFlags,
        load_op : ash::vk::AttachmentLoadOp,
        store_op : ash::vk::AttachmentStoreOp,
        resource : &mut Texture
    ) -> &mut Self {
        // Inform the resource we are using it as a color attachment
        resource.add_usage(self, access_flags, ash::vk::ImageUsageFlags::COLOR_ATTACHMENT);

        // Register how this texture is used in this pass.
        self.color.add(access_flags, resource.id(), AttachmentDescription {
            load : load_op,
            store : store_op,
            layout : ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        });
        self
    }

    pub fn texture_attachment(
        &mut self,
        name : &'static str,
        access_flags : ResourceAccessFlags,
        load_op : ash::vk::AttachmentLoadOp,
        store_op : ash::vk::AttachmentStoreOp,
        resource : &mut Texture
    ) -> &mut Self {
        // Inform the resource it's used as a texture attachment
        resource.add_usage(self, access_flags, ash::vk::ImageUsageFlags::SAMPLED);

        self.textures.add(access_flags, resource.id(), AttachmentDescription {
            load : load_op,
            store : store_op,
            layout : ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL 
        });
        self
    }
}

struct AttachmentDescription {
    load : ash::vk::AttachmentLoadOp,
    store : ash::vk::AttachmentStoreOp,
    layout : ash::vk::ImageLayout,
}

pub struct InputOutput<T> {
    inputs : Vec<u32>,
    outputs : Vec<u32>,
    marker : PhantomData<T>,
}

#[bitmask(u8)]
pub enum ResourceAccessFlags {
    Read,
    Write
}

impl<T> InputOutput<T> {
    pub fn new() -> Self {
        Self { inputs : vec![], outputs : vec![], marker : PhantomData }
    }

    pub(in self) fn add(&mut self, access : ResourceAccessFlags, resource_id : u32, description : AttachmentDescription) {
        if (access & ResourceAccessFlags::Read) != 0 {
            self.inputs.push(resource_id);
        }

        if (access & ResourceAccessFlags::Write) != 0 {
            self.outputs.push(resource_id);
        }
    }
}
