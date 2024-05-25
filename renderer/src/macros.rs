
macro_rules! valueless_builder {
    ($fn:ident, $val:path) => {
        #[inline] pub fn $fn(mut self) -> Self {
            self.memory_location = $val;
            self
        }
    };
}

macro_rules! value_builder {
    ($fn:ident, $type:ty) => {
        #[inline] pub fn $fn(mut self, $fn : $type) -> Self {
            self.$fn = $fn;
            self
        }
    };
    ($fn:ident, $an:ident, &$lf:lifetime $type:ty) => {
        #[inline] pub fn $fn(mut self, $an : &$lf $type) -> Self {
            self.$an = $an;
            self
        }
    };
    ($fn:ident, $an:ident, $type:ty) => {
        #[inline] pub fn $fn(mut self, $an : $type) -> Self {
            self.$an = $an;
            self
        }
    };
    ($fn:ident, $an:ident, $mn:ident, $type:path) => {
        #[inline] pub fn $fn(mut self, $an : $type) -> Self {
            self.$mn = $an;
            self
        }
    }
}
