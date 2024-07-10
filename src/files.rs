use sha2::digest::Update;
use sha2::{Digest, Sha256};

extern crate glob;
use std::io::Write;

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct StagedFile {
    pub filename: String,
    pub hash: String,
}

// TODO: I think a lot of the usage of this function is hasty and suboptimal
//       I'd bet there's a better, more centralized/efficient way
//       of handling this
pub fn normalize_filename(filename: String) -> String {
    let mut normalized = filename.clone();
    if !filename.starts_with("./") {
        normalized = "./".to_string() + &normalized;
    }

    normalized
}

pub fn get_hash(content: &Vec<u8>) -> String {
    let mut hasher = Sha256::new();
    Update::update(&mut hasher, &content);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub fn get_directory_files(dir: String) -> Vec<String> {
    let mut files = Vec::new();
    let mut stack = Vec::new();
    stack.push(std::path::Path::new(&dir).to_path_buf());
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).expect("Failed to read directory") {
                let entry = entry.expect("Failed to read entry");
                let entry_path = entry.path();
                stack.push(entry_path);
            }
        } else {
            files.push(normalize_filename(path.to_str().unwrap().to_string()));
        }
    }

    files
}

pub fn get_unignored_files() -> Vec<String> {
    let gitignore = match std::fs::read_to_string(".gitignore") {
        Ok(gi) => gi,
        Err(_) => "".to_string(),
    };

    let mut ignore_globs = gitignore
        .lines()
        .filter(|line| !line.starts_with("#"))
        .map(|line| {
            if line.ends_with("/") {
                "./".to_string() + line + "*"
            } else if line.starts_with("/") {
                "./".to_string() + &line[1..] + "/*"
            } else if line.starts_with("./") {
                "./".to_string() + line
            } else {
                "./".to_string() + line
            }
        })
        .collect::<Vec<String>>();

    ignore_globs.extend(vec!["./.git/*".to_string()]);

    let mut unignored_files = Vec::new();
    fn walk(dir: &std::path::Path, file_list: &mut Vec<String>, ignore_globs: &Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("Failed to read directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.is_dir() {
                walk(&path, file_list, ignore_globs);
            } else {
                let path_str = path.to_str().unwrap().to_string();
                if ignore_globs.iter().any(|glob| {
                    let pattern = glob::Pattern::new(glob).unwrap();
                    pattern.matches(&path_str)
                }) {
                    continue;
                }

                file_list.push(normalize_filename(path_str));
            }
        }
    }

    walk(
        std::path::Path::new("."),
        &mut unignored_files,
        &ignore_globs,
    );

    unignored_files
}

// list of filename -> content hash mappings
// space separated, one key-value pair per line
// example entry: "file1.txt 1234567890abcdef"
pub fn read_staging_file() -> Vec<StagedFile> {
    let contents = std::fs::read_to_string(".recall/staged_files").expect("Failed to read file");
    let files = contents.split("\n").collect::<Vec<&str>>();

    files
        .iter()
        .filter(|&f| !f.is_empty())
        .map(|f| {
            let parts = f.split(" ").collect::<Vec<&str>>();
            StagedFile {
                filename: normalize_filename(parts[0].to_string()),
                hash: parts[1].to_string(),
            }
        })
        .collect()
}

pub fn write_staging_file(staged_files: Vec<StagedFile>) {
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(".recall/staged_files")
        .expect("Failed to open file");

    for file in staged_files {
        output
            .write_all(format!("{} {}\n", normalize_filename(file.filename), file.hash).as_bytes())
            .expect("Failed to write to file");
    }
}

// tracked_files is just a newline separated list of filenames
pub fn is_tracked(filename: String) -> bool {
    let contents = std::fs::read_to_string(".recall/tracked_files").expect("Failed to read file");
    let files = contents.split("\n").collect::<Vec<&str>>();

    files.iter().any(|&f| f == filename)
}

pub fn add_to_tracked_files(filename: String) {
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(".recall/tracked_files")
        .expect("Failed to open file");

    output
        .write_all(format!("{}\n", normalize_filename(filename)).as_bytes())
        .expect("Failed to write to file");
}

pub fn read_tracked_files() -> Vec<String> {
    let contents = std::fs::read_to_string(".recall/tracked_files").expect("Failed to read file");
    contents
        .split("\n")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
