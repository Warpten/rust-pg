use std::backtrace::Backtrace;
use std::collections::HashSet;
use std::ffi::CStr;
use std::ptr::null;
use std::{hint, slice};
use std::{cmp::Ordering, ffi::CString, sync::Arc};

use ash::vk;

use crate::traits::handle::Handle;
use crate::vk::physical_device::PhysicalDevice;
use crate::window::Window;

use super::queue::QueueFamily;

pub struct Context {
    pub(in crate) entry : Arc<ash::Entry>,
    pub(in crate) instance : ash::Instance,
    debug_utils : ash::ext::debug_utils::Instance,
    debug_messenger : vk::DebugUtilsMessengerEXT,
    
}

impl Context {
    pub fn get_device_extensions(&self, device : &PhysicalDevice) -> Vec<vk::ExtensionProperties> {
        unsafe {
            self.instance.enumerate_device_extension_properties(device.handle())
                .expect("Failed to enumerate device extensions")
        }
    }

    /// Selects a [`PhysicalDevice`] and its associated graphics and presentation [`queue families`](QueueFamily).
    ///
    /// Device selection is done according to its classification, with the following order:
    ///
    /// 1. [`vk::PhysicalDeviceType::DISCRETE_GPU`]
    /// 2. [`vk::PhysicalDeviceType::INTEGRATED_GPU`]
    /// 3. [`vk::PhysicalDeviceType::VIRTUAL_GPU`]
    /// 4. [`vk::PhysicalDeviceType::CPU`]
    /// 5. [`vk::PhysicalDeviceType::OTHER`]
    ///
    /// If possible, the graphics and presentation queue families will be the same to reduce internal synchronization.
    pub fn select_physical_device(&self, window : &Window, device_extensions : &[CString]) -> (PhysicalDevice, QueueFamily, QueueFamily, QueueFamily) {
        self.get_physical_devices(|left, right| {
            // DISCRETE_GPU > INTEGRATED_GPU > VIRTUAL_GPU > CPU > OTHER
            match (right.properties().device_type, left.properties().device_type) {
                // Base equality case
                (a, b) if a == b => Ordering::Equal,

                // DISCRETE_GPU > ALL
                (vk::PhysicalDeviceType::DISCRETE_GPU, _) => Ordering::Greater,

                // DISCRETE > INTEGRATED > ALL
                (vk::PhysicalDeviceType::INTEGRATED_GPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::INTEGRATED_GPU, _) => Ordering::Greater,

                // DISCRETE, INTEGRATED > VIRTUAL > ALL
                (vk::PhysicalDeviceType::VIRTUAL_GPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::VIRTUAL_GPU, vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::VIRTUAL_GPU, _) => Ordering::Greater,

                // DISCRETE, INTEGRATED, VIRTUAL > CPU > ALL
                (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::DISCRETE_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::INTEGRATED_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::CPU, vk::PhysicalDeviceType::VIRTUAL_GPU) => Ordering::Less,
                (vk::PhysicalDeviceType::CPU, _) => Ordering::Greater,

                // ALL > OTHER
                (vk::PhysicalDeviceType::OTHER, _) => Ordering::Less,

                // Default case for branch solver
                (_, _) => unsafe { hint::unreachable_unchecked() },
            }
        })
        .into_iter()
        .filter(|device| -> bool {
            // 1. First, check for device extensions.
            // We start by collecting a device's extensions and then remove them from the extensions
            // we asked for. If no extension subside, we're good.
            let extensions_supported = {
                let device_extensions_names = self.get_device_extensions(device).into_iter()
                    .map(|device_extension| {
                        unsafe {
                            CStr::from_ptr(device_extension.extension_name.as_ptr()).to_owned()
                        }
                    }).collect::<Vec<_>>();

                let mut required_extensions = device_extensions.iter().collect::<HashSet<_>>();
                for extension_name in device_extensions_names {
                    required_extensions.remove(&extension_name);
                }

                required_extensions.is_empty()
            };

            // 2. Finally, check for swapchain support.
            let supports_present = {
                let surface_formats = window.get_surface_formats(device);
                let surface_present_modes = window.get_present_modes(device);

                !surface_formats.is_empty() && !surface_present_modes.is_empty()
            };

            return extensions_supported && supports_present
        }).find_map(|device| {
            // At this point, the current device is eligible and we just need to check for a present queue and a graphics queue.
            // To do that, we will grab the queue's families.

            let mut graphics_queue = None;
            let mut present_queue = None;
            let mut transfer_queue = None;

            for family in &device.queue_families[..] {
                if family.is_graphics() {
                    graphics_queue = Some(family.clone());

                    // If this family can present as well just use it as a graphics+present queue
                    if family.can_present(&window, &device) {
                        present_queue = Some(family.clone());
                    }
                }

                // Default to the first available present queue
                if family.can_present(&window, &device) && present_queue.is_none() {
                    present_queue = Some(family.clone());
                }

                // If this family can transfer and no transfer queue is found,
                // If this family can transfer and is only a transfer queue
                if family.is_transfer() && ((!family.is_graphics() && !family.is_compute()) || transfer_queue.is_none()) {
                    transfer_queue = Some(family.clone());
                }
            }

            match (graphics_queue, present_queue, transfer_queue) {
                (Some(g), Some(p), Some(t)) => Some((device, g, p, t)),
                _ => None
            }
        }).expect("Failed to select a physical device and an associated queue family")
    }

}

impl Context {
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
            _ => hint::unreachable_unchecked()
        };
        let types = match message_types {
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
            _ => hint::unreachable_unchecked()
        };
        let callback_data = &*p_callback_data;

        let message = CStr::from_ptr(callback_data.p_message);
        println!("======================================================");
        println!("A validation error occured in Vulkan");
        println!("  {} {}: {:?}", severity, types, message);
        #[cfg(debug_assertions)]
        println!("The Rust stack trace follows:");
        #[cfg(debug_assertions)]
        println!("  {}", Backtrace::capture());

        if callback_data.p_queue_labels != null() && callback_data.queue_label_count != 0 { // Print queue labels
            let queue_labels = slice::from_raw_parts(
                callback_data.p_queue_labels,
                callback_data.queue_label_count as _
            );

            println!("The active queue labels were:");
            for queue_label in queue_labels {
                if let Some(label) =  queue_label.label_name_as_c_str() {
                    println!("  - {:?}", label);
                }
            }
        }

        if callback_data.p_cmd_buf_labels != null() && callback_data.cmd_buf_label_count != 0 { // Print command buffer labels
            let labels = slice::from_raw_parts(
                callback_data.p_cmd_buf_labels,
                callback_data.cmd_buf_label_count as _
            );

            println!("The active command buffers were:");
            for label in labels {
                if let Some(label) = label.label_name_as_c_str() {
                    println!("  - {:?}", label);
                }
            }
        }

        if callback_data.p_objects != null() && callback_data.object_count != 0 { // Print object labels
            let labels = slice::from_raw_parts(
                callback_data.p_objects,
                callback_data.object_count as _
            );

            println!("The active objects were:");
            for label in labels {
                if let Some(label_str) = label.object_name_as_c_str() {
                    println!("  - 0x{:#016x} : {:?}", label.object_handle, label_str);
                } else {
                    println!("  - 0x{:#016x}", label.object_handle);
                }
            }
        }
        println!("======================================================");

        vk::FALSE
    }

    pub fn entry(&self) -> &Arc<ash::Entry> { &self.entry }

    pub fn handle(&self) -> &ash::Instance { &self.instance }

    /// Returns all physical devices of this Vulkan instance. The returned [`Vec`] is sorted according to the provided comparator.
    /// # Arguments
    /// 
    /// * `cmp` A comparator function that returns an ordering.
    ///
    /// # Panics
    ///
    /// Panics if [`vkEnumeratePhysicalDevices`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumeratePhysicalDevices.html) fails.
    pub fn get_physical_devices<F>(&self, cmp : F) -> Vec<PhysicalDevice>
        where F : FnMut(&PhysicalDevice, &PhysicalDevice) -> Ordering
    {
        let physical_devices = unsafe {
            self.instance.enumerate_physical_devices()
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
    pub fn new(app_name : CString, instance_extensions: Vec<CString>) -> Self {
        let entry = Arc::new(unsafe { ash::Entry::load().unwrap() });
        let mut debug_utils_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
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
            .pfn_user_callback(Some(Self::vulkan_debug_utils_callback));

        let app_info = vk::ApplicationInfo::default()
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

        Self {
            entry,
            instance,
            debug_utils : debug_utils_loader,
            debug_messenger
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils.destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}
