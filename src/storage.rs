use std::time::SystemTime;
use zstd::stream::encode_all;

// TODO: there's some obvious error handling
//       that needs to be done here

// TODO: memory inefficient and frankly pretty gross
//       can we read files without having
//       the entire Save in memory? yes, but not today

pub const HASH_LENGTH: usize = 64;
pub const CREATOR_LENGTH: usize = 32;

const USIZE_LEN: usize = std::mem::size_of::<usize>();
const U128_LEN: usize = std::mem::size_of::<u128>();

type Hash = [u8; HASH_LENGTH];
type Creator = [u8; CREATOR_LENGTH];

#[macro_export]
macro_rules! to_byte_slice {
    ($slice:expr, $length:expr) => {{
        let mut arr: [u8; $length] = [0; $length];
        let len = std::cmp::min($slice.len(), $length);
        arr[..len].copy_from_slice(&$slice[..len]);
        arr
    }};
}

macro_rules! read_to_slice {
    ($bytes:expr, $cursor:expr, $length:expr) => {{
        let data = read($bytes, $cursor, $length);

        to_byte_slice!(data, $length)
    }};
}

macro_rules! read_to_value {
    ($bytes:expr, $cursor:expr, $length:expr, $type:ty) => {{
        let data = match read($bytes, $cursor, $length).try_into() {
            Ok(data) => data,
            Err(e) => panic!("Failed to convert bytes to type: {:?}", e),
        };

        <$type>::from_be_bytes(data)
    }};
}

fn read(bytes: &Vec<u8>, cursor: &mut usize, length: usize) -> Vec<u8> {
    let data = bytes[*cursor..*cursor + length].to_vec();
    *cursor += length;

    data
}

#[derive(Debug)]
pub struct FileHeaders {
    pub last_modified: u128,
    pub created: u128,
    pub content_length: usize,
    pub filename_length: usize,
    pub filename: String,
    pub content_location: usize,
}

impl FileHeaders {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.last_modified.to_be_bytes());
        bytes.extend_from_slice(&self.created.to_be_bytes());
        bytes.extend_from_slice(&self.content_length.to_be_bytes());
        bytes.extend_from_slice(&self.filename_length.to_be_bytes());
        bytes.extend_from_slice(self.filename.as_bytes());
        bytes.extend_from_slice(&self.content_location.to_be_bytes());

        bytes
    }

    fn from_bytes(bytes: &Vec<u8>) -> FileHeaders {
        let mut cursor = 0;
        let last_modified = read_to_value!(&bytes, &mut cursor, U128_LEN, u128);
        let created = read_to_value!(&bytes, &mut cursor, U128_LEN, u128);
        let content_length = read_to_value!(&bytes, &mut cursor, USIZE_LEN, usize);
        let filename_length = read_to_value!(&bytes, &mut cursor, USIZE_LEN, usize);
        let filename = String::from_utf8(read(&bytes, &mut cursor, filename_length)).unwrap();
        let content_location = read_to_value!(&bytes, &mut cursor, USIZE_LEN, usize);

        FileHeaders {
            last_modified,
            created,
            content_length,
            filename_length,
            filename,
            content_location,
        }
    }

    fn len(&self) -> usize {
        U128_LEN + U128_LEN + USIZE_LEN + USIZE_LEN + self.filename_length + USIZE_LEN
    }
}

#[derive(Debug)]
pub struct Blob {
    pub headers: Vec<FileHeaders>,
    pub data: Vec<u8>,
}

// NOTE: this is _always_ compressed with the headers included
impl Blob {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let headers_size: usize = self.headers.iter().map(|h| h.len()).sum();

        bytes.extend_from_slice(&headers_size.to_be_bytes());
        for header in &self.headers {
            bytes.extend_from_slice(&header.to_bytes());
        }

        bytes.extend_from_slice(&self.data);
        let compressed_bytes = encode_all(&bytes as &[u8], 3).unwrap();

        compressed_bytes
    }

    fn headers_from_bytes(bytes: &Vec<u8>) -> Vec<FileHeaders> {
        let mut cursor = 0;
        let headers_size = read_to_value!(&bytes, &mut cursor, USIZE_LEN, usize);
        let mut headers = Vec::new();
        while cursor < headers_size {
            let header = FileHeaders::from_bytes(&bytes[cursor..].to_vec());
            cursor += header.len();
            headers.push(header);
        }

        headers
    }

    pub fn from_bytes(bytes: &Vec<u8>) -> Blob {
        let decompressed_bytes = zstd::stream::decode_all(&bytes[..]).unwrap();
        let headers = Blob::headers_from_bytes(&decompressed_bytes);
        let headers_size = headers.iter().map(|h| h.len()).sum::<usize>() + USIZE_LEN;
        let data = decompressed_bytes[headers_size..].to_vec();

        Blob { headers, data }
    }

    pub fn get_file(&self, filename: &str) -> Option<Vec<u8>> {
        for header in &self.headers {
            if header.filename == filename {
                return Some(
                    self.data
                        [header.content_location..header.content_location + header.content_length]
                        .to_vec(),
                );
            }
        }

        None
    }
}

pub fn blobify(files: Vec<String>) -> Blob {
    let mut headers = Vec::new();
    let mut data = Vec::new();
    let mut headers_size = 0;
    for f in files {
        let metadata = std::fs::metadata(&f).unwrap();
        let filename_bytes = f.as_bytes();
        let blob_headers = FileHeaders {
            last_modified: metadata
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros(),
            created: metadata
                .created()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros(),
            content_length: metadata.len() as usize,
            filename_length: filename_bytes.len(),
            filename: f.clone(),
            content_location: 0,
        };

        let datum: Vec<u8> = std::fs::read(&f).unwrap();

        headers_size += blob_headers.to_bytes().len();

        headers.push(blob_headers);
        data.push(datum);
    }

    let mut content_offset = headers_size;
    let mut blob_data = Vec::new();
    for i in 0..data.len() {
        headers[i].content_location = content_offset - headers_size;
        content_offset += data[i].len();
        blob_data.extend_from_slice(&data[i]);
    }

    Blob {
        headers,
        data: blob_data,
    }
}

pub struct SaveHeaders {
    pub hash: Hash,
    pub memo: String,
    pub memo_size: usize,
    pub created_date: u128,
    pub creator: Creator,
}

impl SaveHeaders {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.hash);
        bytes.extend_from_slice(&self.memo_size.to_be_bytes());
        bytes.extend_from_slice(self.memo.as_bytes());
        bytes.extend_from_slice(&self.created_date.to_be_bytes());
        bytes.extend_from_slice(&self.creator);

        bytes
    }

    fn from_bytes(bytes: &Vec<u8>) -> SaveHeaders {
        let mut cursor = 0;
        let hash = read_to_slice!(&bytes, &mut cursor, HASH_LENGTH);
        let memo_size = read_to_value!(&bytes, &mut cursor, USIZE_LEN, usize);
        let memo = String::from_utf8(read(&bytes, &mut cursor, memo_size)).unwrap();
        let created_date = read_to_value!(&bytes, &mut cursor, U128_LEN, u128);
        let creator = read_to_slice!(&bytes, &mut cursor, CREATOR_LENGTH);

        SaveHeaders {
            hash,
            memo_size,
            memo,
            created_date,
            creator,
        }
    }

    fn len(&self) -> usize {
        HASH_LENGTH + USIZE_LEN + self.memo_size + U128_LEN + CREATOR_LENGTH
    }
}

pub struct Save {
    pub headers: SaveHeaders,
    pub blob: Blob,
}

impl Save {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.headers.to_bytes();
        bytes.extend_from_slice(&self.blob.to_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &Vec<u8>) -> Save {
        let headers = SaveHeaders::from_bytes(bytes);
        let blob = Blob::from_bytes(&bytes[headers.len()..].to_vec());

        Save { headers, blob }
    }
}
