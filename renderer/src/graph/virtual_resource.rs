use super::{buffer::BufferID, texture::TextureID};

pub enum VirtualResource {
    Texture(TextureID),
    Buffer(BufferID),
}
