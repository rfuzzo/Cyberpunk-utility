//#![warn(clippy::all, rust_2018_idioms)]

mod app;
extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Read, Result};
use std::mem;
use std::path::PathBuf;

pub use app::TemplateApp;

#[derive(Debug, Clone)]
pub struct Archive {
    pub header: Header,
    pub index: Index,
}

impl Archive {
    // Function to read a Header from a file
    pub fn from_file(file_path: &PathBuf) -> Result<Archive> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::with_capacity(mem::size_of::<Header>());

        file.read_to_end(&mut buffer)?;

        // Ensure that the buffer has enough bytes to represent a Header
        if buffer.len() < mem::size_of::<Header>() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File does not contain enough data to parse Header",
            ));
        }

        let mut cursor = io::Cursor::new(&buffer);
        let header = Header::from_reader(&mut cursor)?;

        // move to offset Header.IndexPosition
        cursor.set_position(header.index_position);
        let index = Index::from_reader(&mut cursor)?;

        Ok(Archive { header, index })
    }

    // get filehashes
    pub fn get_file_hashes(&self) -> Vec<u64> {
        self.index
            .file_entries
            .iter()
            .map(|f| f.1.name_hash_64)
            .collect::<Vec<_>>()
    }
}

//static HEADER_MAGIC: u32 = 1380009042;
//static HEADER_SIZE: i32 = 40;
//static HEADER_EXTENDED_SIZE: i32 = 0xAC;

#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub magic: u32,
    pub version: u32,
    pub index_position: u64,
    pub index_size: u32,
    pub debug_position: u64,
    pub debug_size: u32,
    pub filesize: u64,
}

impl Header {
    fn from_reader(cursor: &mut Cursor<&Vec<u8>>) -> io::Result<Self> {
        let header = Header {
            magic: cursor.read_u32::<LittleEndian>()?,
            version: cursor.read_u32::<LittleEndian>()?,
            index_position: cursor.read_u64::<LittleEndian>()?,
            index_size: cursor.read_u32::<LittleEndian>()?,
            debug_position: cursor.read_u64::<LittleEndian>()?,
            debug_size: cursor.read_u32::<LittleEndian>()?,
            filesize: cursor.read_u64::<LittleEndian>()?,
        };

        Ok(header)
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    pub file_table_offset: u32,
    pub file_table_size: u32,
    pub crc: u64,
    pub file_entry_count: u32,
    pub file_segment_count: u32,
    pub resource_dependency_count: u32,
    // pub dependencies: Vec<Dependency>,
    pub file_entries: HashMap<u64, FileEntry>,
    // pub file_segments: Vec<FileSegment>,
}

impl Index {
    fn from_reader(cursor: &mut Cursor<&Vec<u8>>) -> io::Result<Self> {
        let mut index = Index {
            file_table_offset: cursor.read_u32::<LittleEndian>()?,
            file_table_size: cursor.read_u32::<LittleEndian>()?,
            crc: cursor.read_u64::<LittleEndian>()?,
            file_entry_count: cursor.read_u32::<LittleEndian>()?,
            file_segment_count: cursor.read_u32::<LittleEndian>()?,
            resource_dependency_count: cursor.read_u32::<LittleEndian>()?,
            file_entries: HashMap::default(),
        };

        // read files
        for _i in 0..index.file_entry_count {
            let entry = FileEntry::from_reader(cursor)?;
            index.file_entries.insert(entry.name_hash_64, entry);
        }

        // ignore the rest of the archive

        Ok(index)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileSegment {
    pub offset: u64,
    pub size: u32,
    pub z_size: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FileEntry {
    pub name_hash_64: u64,
    pub timestamp: u64, //SystemTime,
    pub num_inline_buffer_segments: u32,
    pub segments_start: u32,
    pub segments_end: u32,
    pub resource_dependencies_start: u32,
    pub resource_dependencies_end: u32,
    pub sha1_hash: [u8; 20],
}

impl FileEntry {
    fn from_reader(cursor: &mut Cursor<&Vec<u8>>) -> io::Result<Self> {
        let mut entry = FileEntry {
            name_hash_64: cursor.read_u64::<LittleEndian>()?,
            timestamp: cursor.read_u64::<LittleEndian>()?,
            num_inline_buffer_segments: cursor.read_u32::<LittleEndian>()?,
            segments_start: cursor.read_u32::<LittleEndian>()?,
            segments_end: cursor.read_u32::<LittleEndian>()?,
            resource_dependencies_start: cursor.read_u32::<LittleEndian>()?,
            resource_dependencies_end: cursor.read_u32::<LittleEndian>()?,
            sha1_hash: [0; 20],
        };

        cursor.read_exact(&mut entry.sha1_hash[..])?;

        Ok(entry)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    pub hash: u64,
}
