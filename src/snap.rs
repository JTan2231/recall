use sha2::{Digest, Sha256};

struct Snapshot {
    commit_hash: String,
    contents_hash: String,
    data: Vec<u8>,
    created_date: String,
    filepath: String,
}

impl Snapshot {
    fn new(data: Vec<u8>, commit_hash: String, filepath: String) -> Self {
        let created_date = chrono::Local::now().to_string();

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        Self {
            commit_hash,
            contents_hash: hash,
            data,
            created_date,
            filepath,
        }
    }
}

pub fn get_file_hash(filename: String) -> String {
    let contents = std::fs::read_to_string(filename).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(contents);

    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}
