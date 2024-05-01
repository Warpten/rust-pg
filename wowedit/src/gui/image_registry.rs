use std::sync::{atomic::{AtomicU64, Ordering}, mpsc::{self, Receiver, Sender}, Arc};

pub type ImageRegistryReceiver = Receiver<RegistryCommand>;

#[derive(Clone)]
pub struct ImageRegistry {
    sender: Sender<RegistryCommand>,
    counter: Arc<AtomicU64>,
}

impl ImageRegistry {
    pub fn new() -> (Self, ImageRegistryReceiver) {
        let (sender, receiver) = mpsc::channel();
        (
            Self {
                sender,
                counter: Arc::new(AtomicU64::new(0)),
            },
            receiver,
        )
    }

    pub fn register_user_texture(
        &self,
        image_view: ash::vk::ImageView,
        sampler: ash::vk::Sampler,
    ) -> egui::TextureId {
        let id = egui::TextureId::User(self.counter.fetch_add(1, Ordering::SeqCst));
        self.sender
            .send(RegistryCommand::RegisterUserTexture {
                image_view,
                sampler,
                id,
            })
            .expect("Failed to send register user texture command.");
        id
    }

    pub fn unregister_user_texture(&self, id: egui::TextureId) {
        let _ = self
            .sender
            .send(RegistryCommand::UnregisterUserTexture { id });
    }
}
pub enum RegistryCommand {
    RegisterUserTexture {
        image_view: ash::vk::ImageView,
        sampler: ash::vk::Sampler,
        id: egui::TextureId,
    },
    UnregisterUserTexture {
        id: egui::TextureId,
    },
}