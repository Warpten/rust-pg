use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use ash::vk;
use nohash_hasher::IntMap;

#[derive(Default)]
pub struct DescriptorSetInfo {
    pub buffers : IntMap<u32, Vec<vk::DescriptorBufferInfo>>,
    pub images : IntMap<u32, Vec<vk::DescriptorImageInfo>>,
}

impl DescriptorSetInfo {
    pub fn buffers(mut self, slot : u32, infos : Vec<vk::DescriptorBufferInfo>) -> Self {
        self.buffers.insert(slot, infos);
        self
    }

    pub fn images(mut self, slot : u32, infos : Vec<vk::DescriptorImageInfo>) -> Self {
        self.images.insert(slot, infos);
        self
    }

    pub fn is_empty(&self) -> bool { self.images.is_empty() && self.buffers.is_empty() }
}

impl Hash for DescriptorSetInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (k, v) in &self.buffers {
            k.hash(state);
            for d in v {
                d.buffer.hash(state);
                d.offset.hash(state);
                d.range.hash(state);
            }
        }
        for (k, v) in &self.images {
            k.hash(state);
            for d in v {
                d.image_layout.hash(state);
                d.image_view.hash(state);
                d.sampler.hash(state);
            }
        }
    }
}

fn vec_eq<T>(left : &Vec<T>, right: &Vec<T>, f : fn(&T, &T) -> bool) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for i in 0..left.len() {
        if !f(&left[i], &right[i]) {
            return false;
        }
    }

    true
}

fn pairwise_eq<K, V, H, F : Fn(&V, &V) -> bool>(left : &HashMap<K, V, H>, right : &HashMap<K, V, H>, f : F) -> bool
    where K : Eq + Hash,
          H : BuildHasher
{
    if left.len() != right.len() {
        return false;
    }

    for (k, v) in left.iter() {
        if !right.contains_key(k) { return false; }
        if !f(v, &right[k]) { return false; }
    }

    true
}

impl PartialEq for DescriptorSetInfo {
    fn eq(&self, other: &Self) -> bool {
        if self.buffers.len() != other.buffers.len() || self.images.len() != other.images.len() {
            return false;
        }

        pairwise_eq(&self.buffers, &other.buffers, |left, right| {
            vec_eq(left, right, |left, right| {
                left.buffer == right.buffer && left.offset == right.offset && left.range == right.range
            })
        }) && pairwise_eq(&self.images, &other.images, |left, right| {
            vec_eq(left, right, |left, right| {
                left.image_layout == right.image_layout && left.image_view == right.image_view && left.sampler == right.sampler
            })
        })
    }
}

impl Eq for DescriptorSetInfo { }