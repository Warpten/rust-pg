use std::ops::{Deref, Range};

use bytes::Buf;
use custom_attrs::CustomAttrs;

use crate::dbcd::raw::Chunked;

#[derive(Debug)]
pub struct FieldInfo {
    field_offset: u16,
    field_offset_bits: u16,

    field_size: u16,
    field_size_bits: u16,

    pub additional_data_size: usize,
    pub storage_type: FieldCompressionType,
}
impl FieldInfo {
    pub fn arity(&self) -> u32 {
        match self.storage_type {
            FieldCompressionType::BitpackedIndexedArray { arity, .. } => arity,
            _ => {
                if self.field_size == 0 {
                    1
                } else {
                    (self.field_size_bits / (self.field_size * 8)) as u32
                }
            }
        }
    }

    #[inline] pub fn offset_bytes(&self) -> usize { self.field_offset_bits as usize / 8 }
    #[inline] pub fn offset_bits(&self) -> usize { self.field_offset_bits as usize }

    #[inline] pub fn size_bits(&self) -> usize { self.field_size_bits as usize }
    #[inline] pub fn size_bytes(&self) -> usize { (self.size_bits() + (self.offset_bits() % 8) + 7) / 8 }

    pub fn new(mut field_info: &[u8], mut field_storage_info: &[u8]) -> FieldInfo {
        let size = (32 - field_info.get_i16_le() as u16) / 8;
        let position = field_info.get_u16_le();

        let field_offset_bits = field_storage_info.get_u16_le();
        let field_size_bits = field_storage_info.get_u16_le();
        let additional_data_size = field_storage_info.get_u32_le() as usize;

        let storage_type = match field_storage_info.get_u32_le() {
            0 => {
                FieldCompressionType::None
            },
            1 => {
                let bitpacking = (
                    field_storage_info.get_u32_le(),
                    field_storage_info.get_u32_le()
                );

                let flags = field_storage_info.get_u32_le();

                FieldCompressionType::Bitpacked {
                    bitpacking,
                    flags,
                }
            },
            2 => {
                let default = field_storage_info.get_u32_le();

                FieldCompressionType::Common {
                    default
                }
            },
            3 => {
                let bitpacking = (
                    field_storage_info.get_u32_le(),
                    field_storage_info.get_u32_le()
                );
                FieldCompressionType::BitpackedIndexed {
                    bitpacking
                }
            },
            4 => {
                let bitpacking = (
                    field_storage_info.get_u32_le(),
                    field_storage_info.get_u32_le()
                );

                let arity = field_storage_info.get_u32_le();

                FieldCompressionType::BitpackedIndexedArray {
                    bitpacking,
                    arity
                }
            },
            _ => panic!("Unsupported compression type")
        };

        FieldInfo {
            field_offset: position,
            field_offset_bits,
            field_size: size,
            field_size_bits,
            additional_data_size,
            storage_type,
        }
    }
}

#[derive(Debug, CustomAttrs)]
#[attr(
    #[function = "category"]
    pub category: FieldCompressionCategory
)]
pub enum FieldCompressionType {
    #[attr(category = FieldCompressionCategory::Inline)]
    None,
    #[attr(category = FieldCompressionCategory::Inline)]
    Bitpacked { bitpacking: (u32, u32), flags: u32 },
    #[attr(category = FieldCompressionCategory::Common)]
    Common { default: u32 },
    #[attr(category = FieldCompressionCategory::Pallet)]
    BitpackedIndexed { bitpacking: (u32, u32) },
    #[attr(category = FieldCompressionCategory::Pallet)]
    BitpackedIndexedArray { bitpacking: (u32, u32), arity: u32 }
}

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub enum FieldCompressionCategory {
    Inline = 0,
    Pallet = 1,
    Common = 2,
    MAX
}