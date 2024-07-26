use std::collections::HashMap;

#[derive(Default)]
pub struct KeyRing {
    keys: HashMap<u64, [u8; 16]>,
}
impl KeyRing {
    pub fn add(&mut self, name: u64, key: &[u8]) {
        if let Ok(slice) = TryInto::<[u8; 16]>::try_into(key) {
            self.keys.insert(name, slice);
        }
    }

    /// Returns the key with its name, or a [Empty](None) [`Option`] if none is found.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the key.
    pub fn try_get(&self, name: u64) -> Option<&[u8; 16]> {
        self.keys.get(&name)
    }
}
