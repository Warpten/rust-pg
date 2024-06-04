use std::{fs, path::PathBuf};

use ash::vk;

use crate::{make_handle, vk::logical_device::LogicalDevice};

pub struct PipelinePool {
    cache : vk::PipelineCache,

    path : PathBuf,
}

impl PipelinePool {
    pub fn new(device : ash::Device, path : PathBuf) -> Self {
        let data = fs::read(path.as_path()).unwrap_or(vec![]);
        
        let create_info = vk::PipelineCacheCreateInfo::default()
            .initial_data(&data);

        let cache = unsafe {
            device.create_pipeline_cache(&create_info, None)
                .expect("An error occured while creating a pipeline cache")
        };

        Self { cache, path }
    }

    pub fn save(&self, device : &LogicalDevice) {
        unsafe {
            let data = device.handle().get_pipeline_cache_data(self.cache).unwrap_or(vec![]);

            _ = fs::write(self.path.as_path(), data);
        }
    }
}

make_handle! { PipelinePool, vk::PipelineCache, cache }
