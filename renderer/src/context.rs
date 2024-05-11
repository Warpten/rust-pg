use std::{cmp::Ordering, ffi::CString, sync::Arc};

use crate::traits::BorrowHandle;

use super::PhysicalDevice;

pub struct Context {
    entry : Arc<ash::Entry>,
    handle : ash::Instance,
    pub debug_utils : ash::ext::debug_utils::Instance,
    pub debug_messenger : ash::vk::DebugUtilsMessengerEXT,
    
}

impl BorrowHandle for Context {
    type Target = ash::Instance;

    fn handle(&self) -> &ash::Instance { &self.handle }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils.destroy_debug_utils_messenger(self.debug_messenger, None);
            self.handle.destroy_instance(None);
        }
    }
}

impl Context {
    unsafe extern "system" fn vulkan_debug_utils_callback(
        message_severity : ash::vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types : ash::vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data : *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
        _p_user_data : *mut std::ffi::c_void,
    ) -> ash::vk::Bool32 {
        let severity = match message_severity {
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
            _ => panic!("[UNKNOWN]"),
        };
        let types = match message_types {
            ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
            ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
            ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
            _ => panic!("[UNKNOWN]"),
        };
        let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
        println!("[DEBUG]{}{}{:?}", severity, types, message);

        ash::vk::FALSE
    }

    pub fn entry(&self) -> &Arc<ash::Entry> { &self.entry }

    /// Returns all physical devices of this Vulkan instance. The returned [`Vec`] is sorted according to the provided comparator.
    /// # Arguments
    /// 
    /// * `cmp` A comparator function that returns an ordering.
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumeratePhysicalDevices`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumeratePhysicalDevices.html) fails.
    pub fn get_physical_devices<F>(self : &Arc<Context>, cmp : F) -> Vec<PhysicalDevice>
        where F : FnMut(&PhysicalDevice, &PhysicalDevice) -> Ordering
    {
        let physical_devices = unsafe {
            self.handle.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        let mut devices = physical_devices.iter().map(|physical_device| {
            PhysicalDevice::new(physical_device.clone(), &self)
        }).collect::<Vec<_>>();
        
        devices.sort_by(cmp);

        devices
    }

    /// Creates a new [`Context`].
    /// 
    /// # Arguments
    /// 
    /// * `app_name` - The name of the application.
    /// * `instance_extensions` - An array of extensions to apply to this instance.
    ///
    /// # Panics
    ///
    /// * Panics if [`vkCreateInstance`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateInstance.html) failed.
    /// * Panics if [`vkCreateDebugUtilsMessengerEXT`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDebugUtilsMessengerEXT.html) failed.
    pub fn new(app_name : CString, instance_extensions: Vec<CString>) -> Arc<Self> {
        let entry = Arc::new(unsafe { ash::Entry::load().unwrap() });
        let mut debug_utils_messenger_create_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
            .flags(ash::vk::DebugUtilsMessengerCreateFlagsEXT::empty())
            .message_severity(
                ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            )
            .message_type(
                ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            )
            .pfn_user_callback(Some(Self::vulkan_debug_utils_callback));

        let app_info = ash::vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(ash::vk::make_api_version(1, 0, 0, 0))
            .api_version(ash::vk::API_VERSION_1_3);

        const VALIDATION: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

        let extension_names = instance_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<_>>();
        let raw_layer_names = VALIDATION.iter()
            .map(|&l| CString::new(l).unwrap())
            .collect::<Vec<_>>();
        let layer_names = raw_layer_names.iter()
            .map(|l| l.as_ptr())
            .collect::<Vec<_>>();
        
        let instance_create_info = ash::vk::InstanceCreateInfo::default()
            .push_next(&mut debug_utils_messenger_create_info)
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        let instance = unsafe {
            entry.create_instance(&instance_create_info, None)
                .expect("Failed to create instance")
        };

        // setup debug utils
        let debug_utils_loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let debug_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)
                .expect("Failed to create debug utils messenger")
        };

        Arc::new(Self {
            entry,
            handle : instance,
            debug_utils : debug_utils_loader,
            debug_messenger
        })
    }
}