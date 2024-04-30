use std::{cmp::Ordering, ffi::CString, ops::Deref, sync::Arc};
use ash::vk;

use super::PhysicalDevice;

pub struct Instance {
    pub handle : ash::Instance,
    pub debug_utils : ash::ext::debug_utils::Instance,
    pub debug_messenger : ash::vk::DebugUtilsMessengerEXT,
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target { &self.handle }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils.destroy_debug_utils_messenger(self.debug_messenger, None);
            self.handle.destroy_instance(None);
        }
    }
}

impl Instance {
    unsafe extern "system" fn vulkan_debug_utils_callback(
        message_severity : vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types : vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data : *const vk::DebugUtilsMessengerCallbackDataEXT,
        _p_user_data : *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        let severity = match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
            _ => panic!("[UNKNOWN]"),
        };
        let types = match message_types {
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
            _ => panic!("[UNKNOWN]"),
        };
        let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
        println!("[DEBUG]{}{}{:?}", severity, types, message);

        vk::FALSE
    }

    /// Returns all physical devices of this Vulkan instance. The returned [`Vec`] is sorted by device type,
    /// in the following order:
    /// 
    /// 1. [`ash::vk::PhysicalDeviceType::DISCRETE_GPU`]
    /// 2. [`ash::vk::PhysicalDeviceType::INTEGRATED_GPU`]
    /// 3. [`ash::vk::PhysicalDeviceType::VIRTUAL_GPU`]
    /// 4. [`ash::vk::PhysicalDeviceType::CPU`]
    /// 5. [`ash::vk::PhysicalDeviceType::OTHER`]
    /// 
    /// # Arguments
    /// 
    /// * `cmp` A comparator function that returns an ordering.
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumeratePhysicalDevices`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumeratePhysicalDevices.html) fails.
    pub fn get_physical_devices<F>(self : Arc<Instance>, cmp : F) -> Vec<Arc<PhysicalDevice>>
        where F : FnMut(&Arc<PhysicalDevice>, &Arc<PhysicalDevice>) -> Ordering
    {
        let physical_devices = unsafe {
            self.handle.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        let mut devices = physical_devices.iter().map(|physical_device| {
            PhysicalDevice::new(physical_device.clone(), self)
        }).collect::<Vec<_>>();
        
        devices.sort_by(cmp);

        devices
    }

    /// Creates a new [`Instance`].
    /// 
    /// # Arguments
    /// 
    /// * `handle` - The global Vulkan entry table.
    /// * `app_name` - The name of the application.
    /// * `instance_extensions` - An array of extensions to apply to this instance.
    ///
    /// # Panics
    ///
    /// * Panics if [`vkCreateInstance`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateInstance.html) failed.
    /// * Panics if [`vkCreateDebugUtilsMessengerEXT`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDebugUtilsMessengerEXT.html) failed.
    pub fn new(handle : Arc<ash::Entry>, app_name : CString, instance_extensions: Vec<CString>) -> Arc<Self> {
        let mut debug_utils_messenger_create_info = {
            vk::DebugUtilsMessengerCreateInfoEXT::default()
                .flags(vk::DebugUtilsMessengerCreateFlagsEXT::empty())
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                )
                .pfn_user_callback(Some(Self::vulkan_debug_utils_callback))
        };

        let app_info =
            vk::ApplicationInfo::default()
                .application_name(&app_name)
                .application_version(vk::make_api_version(1, 0, 0, 0))
                .api_version(vk::API_VERSION_1_3);

        const VALIDATION: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

        let extension_names = instance_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<_>>();
        let raw_layer_names = VALIDATION.iter()
            .map(|&l| CString::new(l).unwrap())
            .collect::<Vec<_>>();
        let layer_names = raw_layer_names.iter()
            .map(|l| l.as_ptr())
            .collect::<Vec<_>>();
        
        let instance_create_info = vk::InstanceCreateInfo::default()
            .push_next(&mut debug_utils_messenger_create_info)
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        let instance = unsafe {
            handle.create_instance(&instance_create_info, None)
                .expect("Failed to create instance")
        };

        // setup debug utils
        let debug_utils_loader = ash::ext::debug_utils::Instance::new(&handle, &instance);
        let debug_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)
                .expect("Failed to create debug utils messenger")
        };

        Arc::new(Self {
            handle : instance,
            debug_utils : debug_utils_loader,
            debug_messenger,
        })
    }
}