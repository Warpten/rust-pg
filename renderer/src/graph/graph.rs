use std::{collections::HashMap, sync::Arc};


use crate::{graph::manager::Identifiable, utils::topological_sort::TopologicalSorter, LogicalDevice, Queue};

use super::{buffer::{Buffer, BufferID}, manager::{Identifier, Manager}, pass::{Pass, PassID}, resource::{Resource, ResourceID}, texture::{Texture, TextureID}};

/// A rendering graph.
/// 
/// A rendering graph declares a set of passes and resources. Each pass can refer to the 
pub struct Graph {
    pub(in super) passes : Manager<Pass>,
    pub(in super) resources : Manager<Resource>,
    backbuffer : TextureID,
}

struct TextureHistoryPoint;
struct BufferHistoryPoint;

enum SynthesizedResource {
    Texture(Texture, Vec<TextureHistoryPoint>),
    Buffer(Buffer, Vec<BufferHistoryPoint>),
}

impl Graph {
    /// Creates a new render graph.
    pub fn new() -> Self {
        Self {
            passes : Manager::default(),
            resources : Manager::default(),
            backbuffer : TextureID(Identifier::None),
        }
    }

    pub fn backbuffer(&self) -> Option<&Texture> { self.find_texture(self.backbuffer) }

    pub fn build(&self, queue : &Queue, device : &Arc<LogicalDevice>) {
        assert_ne!(self.backbuffer, TextureID(Identifier::None), "No backbuffer declared for this graph");
        
        let backbuffer = unsafe { self.backbuffer().unwrap_unchecked() };

        let mut topological_sequence : Vec<&Pass> = {
            let mut sorter = TopologicalSorter::<PassID>::default();
            for pass in self.passes.iter() {
                // TODO: Skip passes with no edges or passes that do not end up writing to any of the graph's real outputs

                // For each pass, find the edges (as in, passes that read from its outputs).
                let edges = self.passes.iter().filter(|other_pass| {
                    other_pass.inputs().iter().any(|input| {
                        match input {
                            ResourceID::Virtual(edge_start, _) => *edge_start == pass.id(),
                            _ => false
                        }
                    })
                }).map(Pass::id).collect::<Vec<_>>();

                sorter = sorter.add_node(pass.id(), edges);
            }

            match sorter.sort_kahn() {
                Ok(sorted) => sorted.iter().filter_map(|&pass_id| self.find_pass(pass_id)).collect::<Vec<_>>(),
                Err(_) => panic!("Cyclic graph detected"),
            }
        };

        // TODO: Understand how all of this relates to swapchain framebuffer output.
        // Don't I need to invert the topology now? The last pass should be writing a frame to the swapchain's
        // framebuffer...
        
        // Now that we have a sequence of topologically ordered passes, we should reorder them a tiny bit
        // to reduce stalls in the pipeline when hard dependencies are specified for no reason.
        self.reorder(&mut topological_sequence);

        // At this point, pass ordering should not change, so let's freeze the variable
        let topological_sequence = topological_sequence;

        // Start by collecting the life cycle of resources used in the graph.
        let (buffer_history, texture_history) = {
            let mut texture_history = HashMap::<(TextureID, PassID), TextureHistoryPoint>::default();
            let mut buffer_history = HashMap::<(BufferID, PassID), BufferHistoryPoint>::default();

            for pass in topological_sequence {
                for input in pass.inputs() {
                    // Recursively look for the physical resource
                    let mut drilled_input = input;
                    while let ResourceID::Virtual(_, resource_id) = drilled_input {
                        drilled_input = *resource_id;
                    }

                    match drilled_input {
                        ResourceID::Texture(texture) => {
                            texture_history.insert((texture, pass.id()), TextureHistoryPoint { });
                        },
                        ResourceID::Buffer(buffer) => {
                            buffer_history.insert((buffer, pass.id()), BufferHistoryPoint { });
                        },
                        ResourceID::Virtual(_, _) => unreachable!("What the hell is happening"),
                        _ => (),
                    }
                }
            }
            (texture_history, buffer_history)
        };

        // https://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        // https://blog.traverseresearch.nl/render-graph-101-f42646255636
        // https://blog.traverseresearch.nl/an-update-to-our-render-graph-17ca4154fd23
        // https://levelup.gitconnected.com/organizing-gpu-work-with-directed-acyclic-graphs-f3fd5f2c2af3
    }

    fn reorder(&self, seq : &mut Vec<&Pass>) { /* todo implement */}

    /// Find a rendering pass.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for the pass.
    pub fn find_pass(&self, identifier : PassID) -> Option<&Pass> {
        self.passes.find(identifier)
    }

    /// Returns a registered resource, given an uniquely identifying name.
    /// If no resource with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A unique identifier for that resource.
    pub fn find_resource(&self, identifier : ResourceID) -> Option<&Resource> {
        self.resources.find(identifier)
    }

    /// Returns a registered texture, given an uniquely identifying name.
    /// If no texture with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that texture.
    pub fn find_texture(&self, identifier : TextureID) -> Option<&Texture> {
        self.find_resource(identifier.to_resource()).and_then(|resource| {
            match resource {
                Resource::Texture(value) => Some(value),
                _ => None
            }
        })
    }

    /// Returns a registered resource, given its ID.
    /// If no buffer with that name exists, returns an empty Option.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of that buffer in this graph.
    pub fn find_buffer(&self, identifier : &BufferID) -> Option<&Buffer> {
        self.find_resource(identifier.to_resource()).and_then(|resource| {
            match resource {
                Resource::Buffer(buffer) => Some(buffer),
                _ => None
            }
        })
    }

    pub fn reset(&mut self) {
        self.passes.clear();
        self.resources.clear();
    }
}
