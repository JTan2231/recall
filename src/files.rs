extern crate glob;

pub fn get_tracked_files() -> Vec<String> {
    let mut tracked_files = Vec::new();
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

    println!("{:?}", ignore_globs);

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

                file_list.push(path_str);
            }
        }
    }

    walk(std::path::Path::new("."), &mut tracked_files, &ignore_globs);

    tracked_files
}
