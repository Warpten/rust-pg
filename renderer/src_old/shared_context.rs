pub struct SharedContext {
    entry: Entry,
    instance: Instance,

    debug_utils_loader: ash::ext::debug_utils::Instance,
    debug_call_back: ash::vk::DebugUtilsMessengerEXT,
    
    device: Arc<LogicalDevice>,
    allocator: ManuallyDrop<Arc<Mutex<Allocator>>>,
}