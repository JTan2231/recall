use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::str;

fn main() {
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
    let system_prompt = "You are the best summarizer in the world. Your task is, given a generated git diff, to summarize the changes. Be thorough, be concise. Be tasteful! Expect things to be there, but don't comment unless they're otherwise notable--or missing! Be definitive--speak with authority on what you're seeing. Be presumptuous and confident about the purpose of the changed code. Write as if you are the author of the repository in which the changes are taking place--_never_ be an outsider.";
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

    println!("Request body: {}", json_string);

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

    // Upgrade to TLS
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
}
