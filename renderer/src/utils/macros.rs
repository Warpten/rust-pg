#[macro_export]
macro_rules! inject_equality {
    ($x:ty) => {
        pub struct $ty(ash::vk::$ty);
        impl core::cmp::PartialEq for $ty {
            fn eq(&self, other : &self) -> bool {
                
            }
        }
        impl core::cmp::Eq for $ty { }
        impl Deref for $ty {
            type Target = ash::vk::$ty;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
    }
}