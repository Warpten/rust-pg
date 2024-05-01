use super::pass::Pass;

pub struct Texture {
    usage : ash::vk::ImageUsageFlags,
    /// Indice of all the passes using this texture
    passes : Vec<usize>,
}

impl Texture {
    pub fn usage(&self) -> ash::vk::ImageUsageFlags { self.usage }
    pub fn add_usage(&mut self, usage : ash::vk::ImageUsageFlags) {
        self.usage |= usage;
    }

    pub fn add_pass(&mut self, pass : &Pass) {
        self.passes.push(pass.index());
    }
}

pub struct Buffer {

}

pub enum Resource {
    Texture(Texture),
    Buffer(Buffer),
    None
}

impl Default for Resource {
    fn default() -> Self {
        Self::None
    }
}