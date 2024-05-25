use std::{fs, path::PathBuf, sync::Arc};

use ash::vk;

use crate::{traits::handle::Handle, vk::logical_device::LogicalDevice};

pub struct PipelinePool {
    device : Arc<LogicalDevice>,
    cache : vk::PipelineCache,

    path : PathBuf,
}

impl PipelinePool {
    pub fn new(device : Arc<LogicalDevice>, path : PathBuf) -> Self {
        let data = fs::read(path.as_path()).unwrap_or(vec![]);
        
        let create_info = vk::PipelineCacheCreateInfo::default()
            .initial_data(&data);

        let cache = unsafe {
            device.handle().create_pipeline_cache(&create_info, None)
                .expect("An error occured while creating a pipeline cache")
        };

        Self { cache, path, device }
    }

    pub fn save(&self) {
        unsafe {
            let data = self.device.handle().get_pipeline_cache_data(self.cache).unwrap_or(vec![]);

            _ = fs::write(self.path.as_path(), data);
        }
    }
}

impl Handle<vk::PipelineCache> for PipelinePool {
    fn handle(&self) -> vk::PipelineCache { self.cache }
}

impl Drop for PipelinePool {
    fn drop(&mut self) {
        self.save();

        unsafe {
            self.device.handle().destroy_pipeline_cache(self.cache, None);
        }
    }
}