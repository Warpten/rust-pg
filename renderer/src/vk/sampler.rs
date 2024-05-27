use std::sync::Arc;

use ash::vk;

use crate::make_handle;

use super::logical_device::LogicalDevice;

#[derive(Default)]
pub struct SamplerCreateInfo {
    address_mode : [vk::SamplerAddressMode; 3],
    anisotropy : bool,
    filter : [vk::Filter; 2],
    mipmap_mode : vk::SamplerMipmapMode,
    lod : [f32; 2],
}

impl SamplerCreateInfo {
    pub fn address_mode(mut self, u : vk::SamplerAddressMode, v : vk::SamplerAddressMode, w : vk::SamplerAddressMode) -> Self {
        self.address_mode = [u, v, w];
        self
    }

    value_builder! { anisotropy, bool }

    pub fn filter(mut self, min : vk::Filter, mag : vk::Filter) -> Self {
        self.filter = [min, mag];
        self
    }

    value_builder! { mipmap_mode , vk::SamplerMipmapMode }

    pub fn lod(mut self, min : f32, max : f32) -> Self {
        self.lod = [min, max];
        self
    }

    pub fn build(self, device : &Arc<LogicalDevice>) -> Sampler {
        unsafe {
            let create_info = vk::SamplerCreateInfo::default()
                .address_mode_u(self.address_mode[0])
                .address_mode_v(self.address_mode[1])
                .address_mode_w(self.address_mode[2])
                .anisotropy_enable(self.anisotropy)
                .mag_filter(self.filter[1])
                .min_filter(self.filter[0])
                .mipmap_mode(self.mipmap_mode);

            let handle = device.handle()
                .create_sampler(&create_info, None)
                .expect("Failed to create a sampler");

            Sampler { device : device.clone(), handle }
        }
    }
}

pub struct Sampler {
    device : Arc<LogicalDevice>,
    handle : vk::Sampler,
}

impl Sampler {
    pub fn builder() -> SamplerCreateInfo {
        SamplerCreateInfo::default()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_sampler(self.handle, None);
        }
    }
}

make_handle! { Sampler, vk::Sampler }
