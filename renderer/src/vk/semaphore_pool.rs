use ash::vk;

use crate::orchestration::rendering::RenderingContext;

/// See <https://github.com/KhronosGroup/Vulkan-Samples/blob/master/framework/semaphore_pool.h>.
pub struct SemaphorePool {
    context : RenderingContext,
    handles: Vec<vk::Semaphore>,
    active_count: usize,
}

impl SemaphorePool {
    pub fn new(context : &RenderingContext) -> Self {
        SemaphorePool {
            context : context.clone(),
            handles: Vec::new(),
            active_count: 0,
        }
    }

    /// Requests a semaphore from the pool. If no semaphore is available, a new semaphore will be created and managed.
    pub fn request(&mut self) -> vk::Semaphore {
        if self.active_count < self.handles.len() {
            let index = self.active_count;
            self.active_count = self.active_count + 1;
            self.handles[index]
        } else {
            unsafe {
                let semaphore_create_info = vk::SemaphoreCreateInfo::default();
                let semaphore = self.context.device
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

    /// Signals to this pool that all semaphores are free to use.
    pub fn reset(&mut self) {
        self.active_count = 0;
    }
}

impl Drop for SemaphorePool {
    fn drop(&mut self) {
        self.reset();
        
        unsafe {
            self.handles.iter().for_each(|s| {
                self.context.device.handle().destroy_semaphore(*s, None);
            });
        }
    }
}