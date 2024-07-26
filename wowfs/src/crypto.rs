use std::{io::Read, marker::PhantomData};

pub struct RC4<'a> {
    key: &'a [u8],
    state: [u8; 256],

    i: u8,
    j: u8
}
impl RC4<'_> {
    /// Prepares a new RC4 stream.
    pub fn new(key: &[u8]) -> Self {
        let mut slf = Self {
            key,
            state: std::array::from_fn(|i| i as u8),
            i: 0,
            j: 0
        };

        rc4::ksa(&mut slf.state, key);
        slf
    }

    /// Updates the current RC4 state, modifying the provided stream.
    /// 
    /// # Arguments
    /// 
    /// * `data` - The data to use to update the stream. It will be modified in-place.
    pub fn update(&mut self, data: &mut [u8]) {
        let mut i = self.i;
        let mut j = self.j;

        for k in 0..data.len() {
            i = i.wrapping_add(1);
            j = j.wrapping_add(self.state[i as usize]);

            self.state.swap(i as usize, j as usize);

            let l = u8::wrapping_add(self.state[i as usize], self.state[j.0 as usize]);
            data[k] ^= self.state[l as usize];
        }

        self.i = i;
        self.j = j;
    }
}

mod rc4 {
    // Key-schedule algorithm
    pub fn ksa(s: &mut [u8], k: &[u8]) {
        let mut j : u8 = 0;
        for i in 0..256 {
            j = u8::wrapping_add(j, s[i]);
            j = u8::wrapping_add(j, k[i % k.len()]);
            s.swap(i, j as usize);
        }
    }
}