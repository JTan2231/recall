use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::str;
use tokenizers::tokenizer::Tokenizer;

fn main() {
    let tokenizer = Tokenizer::from_pretrained("gpt-4o", None);

    let extension_whitelist = [
        ".rs", ".py", ".js", ".ts", ".html", ".css", ".json", ".yaml", ".yml", ".toml", ".md",
    ];

    let diff_files: Vec<String> = extension_whitelist
        .iter()
        .map(|ext| format!("*{}", ext))
        .collect();

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

    if diff_string.is_empty() {
        eprintln!("No changes detected");
        return;
    }

    let host = "api.openai.com";
    let path = "/v1/chat/completions";
    let port = 443;
    let system_prompt = std::fs::read_to_string("system_prompt.txt").expect("Failed to read file");
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

    let args = env::args().collect::<Vec<String>>();
    if args.len() > 1 {
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--stdout" => {
                    println!("{}", response_content);
                    return;
                }
                _ => {
                    let mut output_file =
                        std::fs::File::create("diff.txt").expect("Failed to create file");
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
        }
    }
}
