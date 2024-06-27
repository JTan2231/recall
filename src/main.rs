use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

mod diff;
mod openai;
mod parser;

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
    let source = std::fs::read_to_string("diff.txt").expect("Failed to read file");
    let changed = std::fs::read_to_string("diff2.txt").expect("Failed to read file");

    let mut d = diff::diff(source.clone(), changed);
    d.build();

    let mut updated = source.clone();

    // iterate over the first 10 changes
    for c in d.diff.iter().take(10) {
        match c.char_type {
            diff::DiffCharType::Addition => {
                println!("+{}, {}", c.value, c.index);
                updated.insert(c.index, c.value);
            }
            diff::DiffCharType::Removal => {
                println!("-{}, {}", c.value, c.index);
                updated.remove(c.index);
            }
        }
    }

    println!("{}", updated);
}

fn testing() {
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
    if !flags.whitelist.is_empty() {
        diff_files.extend(extension_whitelist.iter().map(|s| format!("*{}", s)));
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
