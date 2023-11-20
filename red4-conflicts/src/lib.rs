//#![warn(clippy::all, rust_2018_idioms)]

mod app;
extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::{self, File};
use std::hash::Hasher;
use std::io::{self, Cursor, Read, Result};
use std::mem;
use std::path::{Path, PathBuf};

pub use app::TemplateApp;

#[link(name = "kraken_static")]
extern "C" {
    // EXPORT int Kraken_Decompress(const byte *src, size_t src_len, byte *dst, size_t dst_len)
    fn Kraken_Decompress(
        buffer: *const u8,
        bufferSize: i64,
        outputBuffer: *mut u8,
        outputBufferSize: i64,
    ) -> i32;
}

#[derive(Debug, Clone)]
pub struct Archive {
    pub header: Header,
    pub index: Index,

    // custom
    pub file_names: Vec<String>,
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

        // read custom data
        let mut file_names: Vec<String> = vec![];
        if let Ok(custom_data_length) = cursor.read_u32::<LittleEndian>() {
            if custom_data_length > 0 {
                cursor.set_position(HEADER_EXTENDED_SIZE);
                if let Ok(footer) = LxrsFooter::from_reader(&mut cursor) {
                    // add files to hashmap
                    for f in footer.files {
                        file_names.push(f);
                    }
                }
            }
        }

        // move to offset Header.IndexPosition
        cursor.set_position(header.index_position);
        let index = Index::from_reader(&mut cursor)?;

        Ok(Archive {
            header,
            index,
            file_names,
        })
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
static HEADER_EXTENDED_SIZE: u64 = 0xAC;

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

#[derive(Debug, Clone)]
pub struct LxrsFooter {
    pub files: Vec<String>,
}

impl LxrsFooter {
    //const MINLEN: u32 = 20;
    const MAGIC: u32 = 0x4C585253;

    fn from_reader(cursor: &mut Cursor<&Vec<u8>>) -> io::Result<Self> {
        let magic = cursor.read_u32::<LittleEndian>()?;
        if magic != LxrsFooter::MAGIC {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid magic"));
        }
        let _version = cursor.read_u32::<LittleEndian>()?;
        let size = cursor.read_i32::<LittleEndian>()?;
        let zsize = cursor.read_i32::<LittleEndian>()?;
        let count = cursor.read_i32::<LittleEndian>()?;

        let mut files: Vec<String> = vec![];
        if size > zsize {
            let mut compressed_buffer = Vec::with_capacity(zsize as usize);
            cursor.read_exact(&mut compressed_buffer[..])?;
            let output_buffer_size = compressed_buffer.len();
            let mut output_buffer: Vec<u8> = Vec::with_capacity(output_buffer_size);
            // buffer is compressed
            let _result = unsafe {
                Kraken_Decompress(
                    compressed_buffer.as_ptr(),
                    compressed_buffer.len().try_into().unwrap(),
                    output_buffer.as_mut_ptr(),
                    output_buffer_size.try_into().unwrap(),
                )
            };
        } else if size < zsize {
            // error
            return Err(io::Error::new(io::ErrorKind::Other, "invalid buffer"));
        } else {
            // no compression
            for _i in 0..count {
                // read NullTerminatedString
                if let Ok(string) = read_null_terminated_string(cursor) {
                    files.push(string);
                }
            }
        }

        let footer = LxrsFooter { files };

        Ok(footer)
    }
}

/////////////////////////////////////////////////////////////////////////////////////////

fn read_null_terminated_string<R>(reader: &mut R) -> io::Result<String>
where
    R: Read,
{
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        reader.read_exact(&mut byte)?;

        if byte[0] == 0 {
            break;
        }

        buffer.push(byte[0]);
    }

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Get top-level files of a folder with given extension
fn get_files(folder_path: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !folder_path.exists() {
        return files;
    }

    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == extension {
                            files.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    files
}

/// Calculate FNV1a64 hash of a PathBuf
fn fnv1a64_hash_path(path: &Path) -> u64 {
    let path_string = path.to_string_lossy();
    let mut hasher = fnv::FnvHasher::default();
    hasher.write(path_string.as_bytes());
    hasher.finish()
}

/// Reads the metadata-resources.csv (csv of hashes and strings) from https://www.cyberpunk.net/en/modding-support
fn parse_csv_data(csv_data: &[u8]) -> HashMap<u64, String> {
    let mut reader = csv::ReaderBuilder::new().from_reader(csv_data);
    let mut csv_map: HashMap<u64, String> = HashMap::new();

    for result in reader.records() {
        match result {
            Ok(record) => {
                // Assuming the CSV has two columns: String and u64
                if let (Some(path), Some(hash_str)) = (record.get(0), record.get(1)) {
                    if let Ok(hash) = hash_str.parse::<u64>() {
                        csv_map.insert(hash, path.to_string());
                    } else {
                        eprintln!("Error parsing u64 value: {}", hash_str);
                    }
                } else {
                    eprintln!("Malformed CSV record: {:?}", record);
                }
            }
            Err(err) => eprintln!("Error reading CSV record: {}", err),
        }
    }

    csv_map
}
