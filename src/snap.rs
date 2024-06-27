struct Snapshot {
    hash: String,
    previous: Option<String>,
    date: String,
    diff: String,
    filename: String,
}

impl Snapshot {
    fn new(hash: String, previous: Option<String>, date: String, diff: String) -> Snapshot {
        Snapshot {
            hash,
            previous,
            date,
            diff,
            filename: "".to_string(),
        }
    }
}

pub fn make_snapshot(filename: String) {}
