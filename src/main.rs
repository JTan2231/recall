use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

struct Flags {
    whitelist: Vec<String>,
    only_whitelist: bool,
    blacklist: Vec<String>,
    check_diff: bool,
    stdout: bool,
}

impl Flags {
    fn new() -> Self {
        Self {
            whitelist: Vec::new(),
            only_whitelist: false,
            blacklist: Vec::new(),
            check_diff: false,
            stdout: false,
        }
    }
}

fn main() {
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

    let host = "api.openai.com";
    let path = "/v1/chat/completions";
    let port = 443;
    let system_prompt =
        std::fs::read_to_string("src/system_prompt.txt").expect("Failed to read file");
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": diff_string
            }
        ]
    });

    let json = serde_json::json!(body);
    let json_string = serde_json::to_string(&json).expect("Failed to serialize JSON");

    let authorization_token =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let request = format!(
        "POST {} HTTP/1.1\r\n\
        Host: {}\r\n\
        Content-Type: application/json\r\n\
        Content-Length: {}\r\n\
        Authorization: Bearer {}\r\n\
        Connection: close\r\n\r\n\
        {}",
        path,
        host,
        json_string.len(),
        authorization_token,
        json_string
    );

    let stream = TcpStream::connect((host, port)).expect("Failed to connect");

    let connector = native_tls::TlsConnector::new().expect("Failed to create TLS connector");
    let mut stream = connector
        .connect(host, stream)
        .expect("Failed to establish TLS connection");

    stream
        .write_all(request.as_bytes())
        .expect("Failed to write to stream");
    stream.flush().expect("Failed to flush stream");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("Failed to read from stream");

    let response_body = response.split("\r\n\r\n").collect::<Vec<&str>>()[1];
    let mut remaining = response_body;
    let mut decoded_body = String::new();
    while !remaining.is_empty() {
        if let Some(index) = remaining.find("\r\n") {
            let (size_str, rest) = remaining.split_at(index);
            let size = usize::from_str_radix(size_str.trim(), 16).unwrap_or(0);

            if size == 0 {
                break;
            }

            let chunk = &rest[2..2 + size];
            decoded_body.push_str(chunk);

            remaining = &rest[2 + size + 2..];
        } else {
            break;
        }
    }

    let response_json: serde_json::Value =
        serde_json::from_str(&decoded_body).expect("Failed to parse JSON");
    let response_content = &response_json["choices"][0]["message"]["content"];

    if flags.stdout {
        println!("{}", response_content);
    } else {
        let mut output_file = std::fs::File::create("diff.txt").expect("Failed to create file");
        output_file
            .write_all(
                response_content
                    .as_str()
                    .expect("Failed to convert to string")
                    .as_bytes(),
            )
            .expect("Failed to write to file");
    }
}
