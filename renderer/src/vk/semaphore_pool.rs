use std::sync::Arc;

use crate::{traits::handle::BorrowHandle, vk::LogicalDevice};

/// See <https://github.com/KhronosGroup/Vulkan-Samples/blob/master/framework/semaphore_pool.h>.
pub struct SemaphorePool {
    device: Arc<LogicalDevice>,
    handles: Vec<ash::vk::Semaphore>,
    active_count: usize,
}

impl SemaphorePool {
    pub fn new(device: &Arc<LogicalDevice>) -> Self {
        SemaphorePool {
            device : device.clone(),
            handles: Vec::new(),
            active_count: 0,
        }
    }

    pub fn rent_semaphore(&mut self) -> ash::vk::Semaphore {
        if self.active_count < self.handles.len() {
            let index = self.active_count;
            self.active_count = self.active_count + 1;
            self.handles[index]
        } else {
            unsafe {
                let semaphore_create_info = ash::vk::SemaphoreCreateInfo::default();
                let semaphore = self
                    .device
                    .handle()
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("Failed to allocate a new semaphore");

                self.handles.push(semaphore.clone());
                semaphore
            }
        }
    }

    pub fn get_active_count(&self) -> usize {
        self.active_count
    }

    pub fn reset(&mut self) {
        self.active_count = 0;
    }
}

impl Drop for SemaphorePool {
    fn drop(&mut self) {
        self.reset();
        unsafe {
            self.handles.iter().for_each(|s| {
                self.device.handle().destroy_semaphore(*s, None);
            });
        }
    }
}