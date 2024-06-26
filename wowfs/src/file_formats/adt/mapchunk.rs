use std::io::{Error, Read};

use bitmask_enum::bitmask;
use bytes::Buf;

use super::read_chunks;

pub struct Offset(pub u32);
impl Into<Offset> for u32 {
    fn into(self) -> Offset { Offset(self) }
}
pub struct Size(pub u32);
impl Into<Size> for u32 {
    fn into(self) -> Size { Size(self) }
}


#[bitmask(u32)]
pub enum HeaderFlags {
    HasMCSH          = (1 << 1),
    Impass           = (1 << 2),
    River            = (1 << 3),
    Ocean            = (1 << 4),
    Magma            = (1 << 5),
    Slime            = (1 << 6),
    HasMCCV          = (1 << 7),
    DoNotFixAlphaMap = (1 << 15),
    HighResHoles     = (1 << 16),
}

pub struct Header {
    flags : HeaderFlags,
    x : u32,
    y : u32,
    layers : u32,
    doodad_refs : u32,
    mcvt : Offset,
    mcnr : Offset,
    mcly : Offset,
    mcrf : Offset,
    mcal : (Offset, Size),
    mcsh : (Offset, Size),
    area_id : u32,
    map_obj_refs : u32,
    holes_low_res : u16, // Bitfield map corresponding to subchunks in a 4x4 area
    unknown : u16,
    low_quality_texturing_map : [u8; 16],
    pred_tex : u32,
    mcse : (Offset, Size),
    mclq : (Offset, Size),
    position : [f32; 3],
    mccv : Offset,
    mclv : Offset,
    unused : u32
}
impl Header {
    pub fn new(data : &[u8]) -> Result<Header, Error> {
        let mut cursor = data;

        let flags = cursor.get_u32_le();
        let x = cursor.get_u32_le();
        let y = cursor.get_u32_le();
        let layers = cursor.get_u32_le();
        let doodad_refs = cursor.get_u32_le();
        let mcvt = cursor.get_u32_le().into();
        let mcnr = cursor.get_u32_le().into();
        let mcly = cursor.get_u32_le().into();
        let mcrf = cursor.get_u32_le().into();
        let mcal = (cursor.get_u32_le().into(), cursor.get_u32_le().into());
        let mcsh = (cursor.get_u32_le().into(), cursor.get_u32_le().into());
        let area_id = cursor.get_u32_le();
        let map_obj_refs = cursor.get_u32_le().into();
        let holes_low_res = cursor.get_u16();
        let unknown = cursor.get_u16().into();
        let low_quality_texturing_map = {
            let mut low_quality_texturing_map : [u8; 16] = [0; 16];
            cursor.read_exact(&mut low_quality_texturing_map)?;
            low_quality_texturing_map
        };
        let pred_tex = cursor.get_u32_le();
        let mcse = (cursor.get_u32_le().into(), cursor.get_u32_le().into());
        let mclq = (cursor.get_u32_le().into(), cursor.get_u32_le().into());
        let position = [cursor.get_f32_le(), cursor.get_f32_le(), cursor.get_f32_le()];
        let mccv = cursor.get_u32_le().into();
        let mclv = cursor.get_u32_le().into();
        let unused = cursor.get_u32_le();

        Ok(Header {
            flags : HeaderFlags::from(flags),
            x, y,
            layers,
            doodad_refs,
            mcvt,
            mcnr,
            mcly,
            mcrf,
            mcal,
            mcsh,
            area_id,
            map_obj_refs,
            holes_low_res,
            unknown,
            low_quality_texturing_map,
            pred_tex,
            mcse,
            mclq,
            position,
            mccv,
            mclv,
            unused
        })
    }
}

pub struct MapChunk {
    header : Header,
    height_map : Option<[f32; 9 * 9 + 8 * 8]>,
    normal_map : Option<Normals>,
}
impl MapChunk {
    pub fn new<R>(data : &mut R, size : u64) -> Result<MapChunk, Error> where R : Read {
        let header = {
            let mut header_bytes = [0_u8; 0x80];
            data.read_exact(&mut header_bytes)?;

            Header::new(&header_bytes)
        }?;

        let mut chunk_data = data.take(size - 0x80);
        let slf = read_chunks(&mut chunk_data, MapChunk {
            header,
            height_map : None,
            normal_map : None,
        }, |mut chunk, slf| {
            match chunk.chunk_id {
                value if value == "MCVT" => {
                    let mut height_map = [0.0; 9 * 9 + 8 * 8];
                    for i in 0..height_map.len() {
                        height_map[i] = chunk.stream.get_f32_le();
                    }
                    
                    // Don't persist the height map if all vertices are set to zero.
                    if height_map.iter().any(|v| *v != 0.0) {
                        slf.height_map = Some(height_map);
                    }
                },
                value if value == "MCLV" => {
                    // Baked level designer-placed omnidirectional lights.

                    let mut light_values = [0_u32; 9 * 9 + 8 * 8];
                    for i in 0..light_values.len() {
                        light_values[i] = chunk.stream.get_u32_le();
                    }
                },
                value if value == "MCNR" => {
                    // Store normals in compressed on uncompressed form.

                    let mut normal_map = [0_u8; (9 * 9 + 8 * 8) * 3];
                    let mut compressed_normal_map = [0_u8; 9 * 9 + 8 * 8];
                    _ = chunk.stream.read_exact(&mut normal_map[..]);

                    let mut compressable = true;
                    for i in (0..(9 * 9 + 8 * 8)).step_by(1) {
                        let x = i * 3;
                        let y = i * 3 + 1;
                        let z = i * 3 + 1;

                        compressable &= normal_map[x] == 0 && normal_map[y] == 0;
                        compressed_normal_map[i] = normal_map[z];

                        if !compressable { break; }
                    }

                    slf.normal_map = if compressable {
                        Some(Normals::Compressed(compressed_normal_map))
                    } else {
                        Some(Normals::Flat(normal_map))
                    };
                }
                _ => println!("Unknown chunk type {}", chunk.chunk_id),
            }
        });

        Ok(slf)
    }
}

pub enum Normals {
    /// Stores (x, y, z) normals. (-127 = -1.0; 127 = 1.0)
    Flat([u8; (9 * 9 + 8 * 8) * 3]),
    /// Stores (0, 0, z) normals. (-127 = -1.0; 127 = 1.0)
    Compressed([u8; 9 * 9 + 8 * 8]),
}