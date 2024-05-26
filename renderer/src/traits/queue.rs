use ash::vk;

use crate::{traits::handle::Handle, vk::queue::QueueFamily};

pub trait Queue : Handle<vk::Queue> {
    fn family(&self) -> &QueueFamily;
}