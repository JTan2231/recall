use std::time::SystemTime;

struct FileHeaders {
    last_modified: u128,
    created: u128,
    content_length: String,
    filename: String,
    content_location: usize,
}

impl FileHeaders {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.last_modified.to_be_bytes());
        bytes.extend_from_slice(&self.created.to_be_bytes());
        bytes.extend_from_slice(self.content_length.as_bytes());
        bytes.extend_from_slice(self.filename.as_bytes());
        bytes.extend_from_slice(&self.content_location.to_be_bytes());

        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> FileHeaders {
        let last_modified = u128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let created = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
        let content_length = std::str::from_utf8(&bytes[32..]).unwrap().to_string();
        let filename = std::str::from_utf8(&bytes[32..]).unwrap().to_string();
        let content_location = usize::from_be_bytes(bytes[32..40].try_into().unwrap());

        FileHeaders {
            last_modified,
            created,
            content_length,
            filename,
            content_location,
        }
    }
}

struct Blob {
    headers: Vec<FileHeaders>,
    data: Vec<u8>,
}

impl Blob {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for header in &self.headers {
            bytes.extend_from_slice(&header.to_bytes());
        }

        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn headers_from_bytes(bytes: &Vec<u8>) -> Vec<FileHeaders> {
        let header_length = std::mem::size_of::<FileHeaders>();
        let mut headers = Vec::new();
        let mut offset = 0;
        while offset < bytes.len() {
            let header = FileHeaders::from_bytes(bytes[offset..offset + header_length].to_vec());
            headers.push(header);
            offset += header_length;
        }

        headers
    }

    fn from_bytes(bytes: Vec<u8>) -> Blob {
        let header_length = std::mem::size_of::<FileHeaders>();
        let headers = Blob::headers_from_bytes(&bytes);
        let total_header_length = headers.len() * header_length;
        let data = bytes[total_header_length..].to_vec();

        Blob { headers, data }
    }
}

pub fn blobify(files: Vec<String>) -> Blob {
    let mut headers = Vec::new();
    let mut data = Vec::new();
    let mut headers_size = 0;
    for f in files {
        let metadata = std::fs::metadata(&f).unwrap();
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
            content_length: metadata.len().to_string(),
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
        headers[i].content_location = content_offset;
        content_offset += data[i].len();
        blob_data.extend_from_slice(&data[i]);
    }

    Blob {
        headers,
        data: blob_data,
    }
}
