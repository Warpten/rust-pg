
pub trait Mesh {
    fn get_binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription>;
    fn get_attributes() -> Vec<ash::vk::VertexInputAttributeDescription>;
}