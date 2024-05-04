use std::{borrow::BorrowMut, rc::Rc};

use super::pass::Pass;

/// Describes a synchronization condition that ensures all identified [`Pass`]es are done before
/// continuing. The scheduler will try to execute these passes on different queues.
pub struct Synchronization {
    pass_indices : Vec<usize>,
    stages : ash::vk::PipelineStageFlags2,
}

impl Synchronization {
    pub fn new(stages : ash::vk::PipelineStageFlags2, passes : &[Pass]) -> Self {
        Self {
            pass_indices : passes.iter().map(Pass::index).collect::<Vec<_>>(),
            stages
        }
    }

    pub fn stages(&self) -> ash::vk::PipelineStageFlags2 { self.stages }

    pub fn affects(&self, pass : &Pass) -> bool {
        self.pass_indices.contains(&pass.index())
    }
}

/// A sequencing condition that ensures a pass is finished before another executes. This only works
/// if both passes execute on the same queue.
pub struct Sequencing {
    first : usize,
    second : usize,
    stages : ash::vk::PipelineStageFlags2
}

impl Sequencing {
    pub fn new(stages : ash::vk::PipelineStageFlags2, first_pass : &mut Rc<Pass>, second_pass : &mut Rc<Pass>) -> Self {
        assert!(first_pass.index() < second_pass.index(), "Cyclic graph detected");

        let this = Self {
            first : first_pass.index(),
            second : second_pass.index(),
            stages
        };

        Pass::link(first_pass, second_pass);
        this
    }
    
    pub fn stages(&self) -> ash::vk::PipelineStageFlags2 { self.stages }
}
