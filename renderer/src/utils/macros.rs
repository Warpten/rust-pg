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

#[macro_export]
macro_rules! make_handle {
    ($x:ty, $h:ty, $f:ident) => {
        impl crate::traits::handle::Handle<$h> for $x {
            fn handle(&self) -> $h { self.$f }
        }

        impl Into<$h> for $x {
            fn into(self) -> $h { self.$f }
        }
    };
    ($x:ty, $h:ty) => {
        impl crate::traits::handle::Handle<$h> for $x {
            fn handle(&self) -> $h { self.handle }
        }

        impl Into<$h> for $x {
            fn into(self) -> $h { self.handle }
        }
    }
}