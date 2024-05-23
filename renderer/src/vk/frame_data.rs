use super::SemaphorePool;

pub struct FrameData {
    pub index : usize,
    pub semaphore_pool : SemaphorePool,
    pub render_fence : ash::vk::Fence,
}