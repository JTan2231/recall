use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use crate::storage::{Save, SaveHeaders, CREATOR_LENGTH, HASH_LENGTH};

mod diff;
mod display;
mod files;
mod openai;
mod parser;
mod storage;

// TODO: use references lol

struct Flags {
    whitelist: Vec<String>,
    only_whitelist: bool,
    blacklist: Vec<String>,
    check_diff: bool,
    stdout: bool,
    update_goals: bool,
}

impl Flags {
    fn new() -> Self {
        Self {
            whitelist: Vec::new(),
            only_whitelist: false,
            blacklist: Vec::new(),
            check_diff: false,
            stdout: false,
            update_goals: false,
        }
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("usage: recall [command] [args]");
        return;
    }

    let command = args.get(1).expect("No command provided");
    match command.as_str() {
        "init" => init(),
        "stage" => {
            init_check();
            stage(args.iter().skip(2).map(|s| s.clone()).collect());
        }
        "unstage" => {
            init_check();
            unstage(args.iter().skip(2).map(|s| s.clone()).collect());
        }
        "save" => {
            init_check();
            if args.len() < 3 {
                eprintln!("usage: recall save [memo]");
                return;
            }

            save(args[2].clone());
        }
        "print-commit" => {
            print_commit();
        }
        "status" => {
            init_check();
            status();
        }
        "help" => {
            eprintln!("usage: recall [command] [args]");
            eprintln!("commands:");
            eprintln!("  init");
            eprintln!("  stage [files...]");
            eprintln!("  unstage [files...]");
            eprintln!("  save [memo]");
            eprintln!("  status");
        }
        _ => eprintln!("unknown command: {}", command),
    }
}

fn init_check() {
    if !Path::new(".recall").exists() {
        eprintln!("no .recall repository found--have you initialized a repository here?");
        std::process::exit(1);
    }
}

fn init() {
    // create .recall directory
    // throw an error if it already exists
    let recall_dir = Path::new(".recall");
    if recall_dir.exists() {
        eprintln!(
            ".recall directory already exists--has a repository already been initialized here?"
        );
        return;
    }

    std::fs::create_dir(recall_dir).expect("Failed to create directory");
    println!("Initialized recall repository");

    // create .recall/commits directory
    let commits_dir = recall_dir.join("commits");
    std::fs::create_dir(commits_dir).expect("Failed to create directory");
    println!("Created commits directory");

    fn touch(path: &Path) {
        let mut file = std::fs::File::create(path).expect("Failed to create file");
        file.write_all(b"").expect("Failed to write to file");
        println!("Created file: {}", path.display());
    }

    touch(&recall_dir.join("tracked_files"));
    touch(&recall_dir.join("staged_files"));
    touch(&recall_dir.join("history"));
}

fn parse_file_args(args: Vec<String>) -> Vec<String> {
    let mut files = Vec::new();
    for arg in args.iter() {
        if std::fs::metadata(arg).map(|m| m.is_dir()).unwrap_or(false) {
            files.extend(files::get_directory_files(arg.clone()));
        } else {
            files.push(files::normalize_filename(arg.clone()));
        }
    }

    files
}

fn stage(args: Vec<String>) {
    let files = parse_file_args(args);
    let mut staged_files = files::read_staging_file();
    for file in files {
        let path = Path::new(&file);
        if !path.exists() {
            eprintln!("file does not exist: {}", file);
            return;
        }

        if !files::is_tracked(file.clone()) {
            files::add_to_tracked_files(file.clone());
        }

        let contents = std::fs::read(&file).expect("Failed to read file");
        let file_hash = files::get_hash(&contents);

        let mut added = false;
        for staged_file in staged_files.iter_mut() {
            // ignore staged files without any pending changes
            if file_hash == staged_file.hash {
                added = true;
                continue;
            } else if file == staged_file.filename {
                added = true;
                staged_file.hash = file_hash.clone();
            }
        }

        if !added {
            staged_files.push(files::StagedFile {
                filename: file.clone(),
                hash: file_hash,
            });
        }
    }

    files::write_staging_file(staged_files);
}

// remove a file (or multiple) from the list of staged files
fn unstage(files: Vec<String>) {
    let files = parse_file_args(files);
    let mut staged_files = files::read_staging_file();
    for file in files {
        for (index, staged_file) in staged_files.iter().enumerate() {
            if staged_file.filename == file {
                staged_files.remove(index);
                break;
            }
        }
    }

    files::write_staging_file(staged_files);
}

fn get_head() -> String {
    let history = std::fs::read_to_string(".recall/history").expect("Failed to read file");

    history
        .lines()
        .last()
        .expect("No commits found")
        .to_string()
}

fn status() {
    let all_unignored_files = files::get_unignored_files();
    let tracked_files = files::read_tracked_files();
    let untracked_files: Vec<String> = all_unignored_files
        .iter()
        .filter(|f| !tracked_files.contains(f))
        .map(|f| f.clone())
        .collect();
    let staged_files = files::read_staging_file();

    // presumably, if there's no last commit,
    // then the tracked files will be empty
    //
    // TODO: pending commit implementation
    //
    let head = get_head();
    let head_path = Path::new(".recall/commits").join(head);
    let head_contents = std::fs::read(&head_path).expect("Failed to read file");
    let head_save = Save::from_bytes(&head_contents);
    let mut tracked_changed_files = Vec::new();
    for tracked_file in tracked_files {
        let contents = std::fs::read(&tracked_file).expect("Failed to read file");
        let file_hash = files::get_hash(&contents);

        match head_save.blob.get_file(&tracked_file) {
            Some(file_contents) => {
                let head_file_hash = files::get_hash(&file_contents);
                if file_hash != head_file_hash {
                    tracked_changed_files.push(tracked_file.clone());
                }
            }
            None => {
                println!("file not found in head commit: {}", tracked_file);
            }
        }
    }

    if !staged_files.is_empty() {
        println!("Staged files:");
        for staged_file in staged_files.iter() {
            println!("  {}", display::green_string(&staged_file.filename));
        }
    }

    if !tracked_changed_files.is_empty() {
        for tracked_file in tracked_changed_files.iter() {
            if staged_files.iter().any(|f| f.filename == *tracked_file) {
                continue;
            }

            println!("  {}", display::green_string(&tracked_file));
        }
    }

    println!();

    println!("Untracked files:");
    for untracked_file in untracked_files.iter() {
        if staged_files.iter().any(|f| f.filename == *untracked_file) {
            continue;
        }

        println!("  {}", display::red_string(&untracked_file));
    }
}

fn save(memo: String) {
    let staged_files = files::read_staging_file();
    let files: Vec<String> = staged_files.iter().map(|f| f.filename.clone()).collect();
    let blob = storage::blobify(files);

    // as a byte string
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Failed to get time")
        .as_micros();

    let hash = files::get_hash(&now.to_string().as_bytes().to_vec());

    let memo_size = memo.len();
    println!("memo size: {}", memo_size);
    let headers = SaveHeaders {
        hash: to_byte_slice!(hash.as_bytes(), HASH_LENGTH),
        memo,
        memo_size,
        created_date: now,
        creator: to_byte_slice!("recall".as_bytes(), CREATOR_LENGTH),
    };

    let save = Save { headers, blob };

    // write save bytes to file named with the hash
    let save_bytes = save.to_bytes();
    let save_path = Path::new(".recall/commits").join(hash.clone());
    let mut save_file = std::fs::File::create(save_path).expect("Failed to create file");
    save_file
        .write_all(&save_bytes)
        .expect("Failed to write to file");

    files::write_staging_file(Vec::new());

    let mut history_file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(".recall/history")
        .expect("Failed to open file");
    history_file
        .write_all(format!("{}\n", hash).as_bytes())
        .expect("Failed to write to file");
}

// this is just a testing function
fn print_commit() {
    let commit_path = Path::new(".recall/commits");
    let commit_files = std::fs::read_dir(commit_path).expect("Failed to read directory");
    for entry in commit_files {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        println!("commit: {}", path.display());
        let mut file = std::fs::File::open(path).expect("Failed to open file");
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .expect("Failed to read file");

        let save = Save::from_bytes(&contents);
        println!("hash: {:?}", save.headers.hash);
        println!("memo: {}", save.headers.memo);
        println!("created: {:?}", save.headers.created_date);
        println!("creator: {:?}", save.headers.creator);
        println!("files:");
        for header in save.blob.headers.iter() {
            println!("  {:?}", header.filename);
            println!(
                "  location, length: {}, {}",
                header.content_location, header.content_length
            );

            let content_bytes = &save.blob.data
                [header.content_location..header.content_location + header.content_length];

            let content = String::from_utf8(content_bytes.to_vec());
            match content {
                Ok(content) => println!("    {}", content),
                Err(_) => println!("    <binary>"),
            }
        }

        println!("data length: {}", save.blob.data.len());
    }
}

fn commit_generation() {
    let args = env::args().collect::<Vec<String>>();

    let mut flags = Flags::new();
    if args.len() > 1 {
        for (index, arg) in args.iter().skip(1).enumerate() {
            match arg.as_str() {
                "--help" => {
                    eprintln!("usage: {} [--stdout] [--whitelist [file1.ext] [file2.ext] [...]] [--only-whitelist] [--blacklist [file1.ext] [file2.ext] [...]] [--check-diff]", args[0]);
                    return;
                }
                "--stdout" => {
                    flags.stdout = true;
                }
                "--whitelist" => {
                    let new_index = index + 2;
                    if new_index < args.len() {
                        for s in args.iter().skip(new_index) {
                            if s.starts_with("--") {
                                break;
                            }

                            flags.whitelist.push(s.clone());
                        }
                    } else {
                        eprintln!("expected argument for --whitelist");
                        eprintln!(
                            "usage: {} --whitelist [file1.ext] [file2.ext] [...]",
                            args[0]
                        );
                        return;
                    }
                }
                "--only-whitelist" => {
                    flags.only_whitelist = true;
                }
                "--blacklist" => {
                    let new_index = index + 2;
                    if new_index < args.len() {
                        for s in args.iter().skip(new_index) {
                            if s.starts_with("--") {
                                break;
                            }

                            if std::fs::metadata(s).map(|m| m.is_dir()).unwrap_or(false) {
                                // if the arg is a directory, then grab all of the contained files
                                let dir_files: Vec<String> = std::fs::read_dir(s)
                                    .expect("Failed to read directory")
                                    .map(|entry| {
                                        entry
                                            .expect("Failed to read entry")
                                            .file_name()
                                            .into_string()
                                            .expect("Failed to convert OsString to String")
                                    })
                                    .collect();

                                flags.blacklist.extend(dir_files);
                            } else {
                                flags.blacklist.push(s.clone());
                            }
                        }
                    } else {
                        eprintln!("expected argument for --blacklist");
                        eprintln!(
                            "usage: {} --blacklist [file1.ext] [file2.ext] [...]",
                            args[0]
                        );
                        return;
                    }
                }
                "--check-diff" => {
                    flags.check_diff = true;
                }
                "--update-goals" => {
                    flags.update_goals = true;
                }
                _ => {}
            }
        }
    }

    let extension_whitelist = [
        ".rs", ".py", ".js", ".ts", ".html", ".css", ".json", ".yaml", ".yml", ".toml", ".md",
    ];

    println!("whitelist: {:?}", flags.whitelist);
    println!("blacklist: {:?}", flags.blacklist);

    let mut diff_files = flags.whitelist.clone();
    if flags.whitelist.is_empty() {
        diff_files.extend(extension_whitelist.iter().map(|s| format!("*{}", s)));
    } else {
        diff_files.extend(flags.whitelist.iter().map(|s| format!("*{}", s)));
    }

    if !flags.blacklist.is_empty() {
        fn traverse_dir(dir: &Path, target_filename: &str, results: &mut Vec<PathBuf>) {
            if dir.is_dir() {
                let entries = std::fs::read_dir(dir).unwrap();
                for entry in entries {
                    let entry = entry.unwrap();
                    let path = entry.path();

                    if path.is_dir() {
                        traverse_dir(&path, target_filename, results);
                    } else if let Some(filename) = path.file_name() {
                        if filename == target_filename {
                            results.push(path);
                        }
                    }
                }
            }
        }

        // git complains if the blacklisted file path isn't exact, so for each file we gotta find
        // its relative filepath
        let mut results = Vec::new();
        let root_dir = Path::new(".");
        for file in flags.blacklist.iter() {
            let mut file_results = Vec::new();
            traverse_dir(root_dir, file, &mut file_results);

            if file_results.len() > 1 {
                eprintln!("found multiple files with name: {}", file);
                return;
            }

            results.extend(file_results);
        }

        diff_files.extend(results.iter().map(|p| format!(":!{}", p.display())));
    }

    println!("using diff for files: {:?}", diff_files);

    let mut diff_output = Command::new("git");
    diff_output
        .arg("diff")
        .arg("--no-color")
        .arg("--no-ext-diff")
        .arg("--no-prefix")
        .arg("--no-renames")
        .arg("--cached")
        .arg("--minimal");

    for file in diff_files {
        diff_output.arg(file);
    }

    let diff_output = diff_output.output().expect("Failed to execute command");

    let diff_string: String;

    if diff_output.status.success() {
        diff_string =
            String::from_utf8(diff_output.stdout.clone()).expect("Invalid UTF-8 sequence");
    } else {
        let stderr = str::from_utf8(&diff_output.stderr).expect("Invalid UTF-8 sequence");
        diff_string = String::from("");
        eprintln!("Command failed with error:\n{}", stderr);
    }

    if flags.check_diff {
        println!("{}", diff_string);
        return;
    }

    if diff_string.is_empty() {
        eprintln!("No changes detected");
        return;
    }

    let system_prompt =
        std::fs::read_to_string("prompts/commit_prompt.txt").expect("Failed to read file");

    println!("prompting for a new commit message...");
    let commit_message = openai::prompt(system_prompt, diff_string.clone());

    if flags.update_goals {
        let goals_prompt =
            std::fs::read_to_string("prompts/goals_prompt.txt").expect("Failed to read file");

        let goalsets = parser::read_goals();
        let latest_goals = match goalsets.last() {
            Some(goalset) => goalset.goals.clone(),
            None => String::from(""),
        };

        println!("prompting for new goals...");
        let new_goals = openai::prompt(
            goals_prompt,
            latest_goals
                + "\n---last commit---\n"
                + commit_message.clone().as_str()
                + "\n---last diff---"
                + diff_string.clone().as_str(),
        );

        // write the new goals to file
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("goal_testing.txt")
            .expect("Failed to open file");

        output
            .write_all(new_goals.as_bytes())
            .expect("Failed to write to file");
    }

    if flags.stdout {
        println!("{}", commit_message);
    } else {
        let mut output_file = std::fs::File::create("diff.txt").expect("Failed to create file");
        output_file
            .write_all(commit_message.as_str().as_bytes())
            .expect("Failed to write to file");
    }
}
