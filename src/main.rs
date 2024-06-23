use std::env;
use std::process::Command;
use std::str;

fn main() {
    let args: Vec<String> = env::args().collect();

    let diff_string = Command::new("git")
        .arg("diff")
        .output()
        .expect("Failed to execute command");

    if diff_string.status.success() {
        let stdout = str::from_utf8(&diff_string.stdout).expect("Invalid UTF-8 sequence");
        println!("Command diff_string:\n{}", stdout);
    } else {
        let stderr = str::from_utf8(&diff_string.stderr).expect("Invalid UTF-8 sequence");
        eprintln!("Command failed with error:\n{}", stderr);
    }

    if args.len() > 1 {
        // ...options
    }
}
