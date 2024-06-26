use std::{fmt::Display, fs::File, io::{BufReader, Error, ErrorKind, Read, Seek, SeekFrom, Take}, path::PathBuf};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use mapchunk::MapChunk;

#[derive(Eq, PartialEq)]
pub struct FourCC(u32);
impl PartialEq<u32> for FourCC {
    fn eq(&self, other: &u32) -> bool { self.0 == *other }
}

impl Display for FourCC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(f, "{}{}{}{}", char::from_u32((self.0 >> 0) & 0xFF).unwrap_or('?'),
                char::from_u32((self.0 >> 8) & 0xFF).unwrap_or('?'),
                char::from_u32((self.0 >> 16) & 0xFF).unwrap_or('?'),
                char::from_u32((self.0 >> 24) & 0xFF).unwrap_or('?'))
        }
    }
}

impl PartialEq<FourCC> for str {
    fn eq(&self, other: &FourCC) -> bool {
        let mut slf : u32 = 0;
        for (i, b) in self.bytes().enumerate() {
            slf |= (b as u32) << (32 - 8 * i);
        }
        slf == other.0
    }
}

impl PartialEq<&str> for FourCC {
    fn eq(&self, other: &&str) -> bool {
        let mut slf : u32 = 0;
        for (i, b) in other.bytes().enumerate() {
            slf |= (b as u32) << (32 - 8 * i);
        }
        slf == self.0
    }
}

struct Chunk<'a> {
    pub chunk_id : FourCC,
    pub stream : &'a [u8],
}

fn read_chunks<R, H, S>(source : &mut R, mut state : S, mut handler : H) -> S where R : Read, H : FnMut(Chunk, &mut S) {
    loop {
        let chunk_id = match source.read_u32::<LittleEndian>() {
            Ok(chunk_id) => FourCC(chunk_id),
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return state,
            _ => panic!("Unknown error")
        };

        let chunk_size = match source.read_u32::<LittleEndian>() {
            Ok(chunk_size) => chunk_size,
            _ => panic!("Parsing error, out of band chunk size")
        };

        if chunk_size > 0 {
            let mut stream = Vec::with_capacity(chunk_size as _);
            _ = source.read_exact(&mut stream[..]);

            handler(Chunk { chunk_id, stream : &stream }, &mut state);
        }
    }
}

pub struct ADT {
    chunks : Vec<MapChunk>,
}
impl ADT {
    pub fn open(file : PathBuf) -> Result<ADT, Error> {
        let mut data = BufReader::new(File::open(file).unwrap());

        let slf = read_chunks(&mut data, ADT {
            chunks : vec![]
        }, |mut data, state| {
            match data.chunk_id {
                value if value == "MVER" => { },
                value if value == "MCNK" => {
                    let length = data.stream.len();
                    match MapChunk::new(&mut data.stream, length as _ ) {
                        Ok(map_chunk) => state.chunks.push(map_chunk),
                        Err(_) => panic!()
                    };
                },
                _ => { }
            }
        });

        Ok(slf)
    }
}

pub mod mapchunk;