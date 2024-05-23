pub trait Handle {
    type Target : ash::vk::Handle;

    fn handle(&self) -> Self::Target;
}

pub trait BorrowHandle {
    type Target;
    
    fn handle(&self) -> &Self::Target;
}
