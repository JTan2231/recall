pub struct Goalset {
    pub goals: String,
    pub date: String,
    pub commit: String,
}

impl Goalset {
    fn new(goals: String, date: String, commit: String) -> Goalset {
        Goalset {
            goals,
            date,
            commit,
        }
    }
}

const GOALS_SOURCE: &str = ".recall";

pub fn read_goals() -> Vec<Goalset> {
    let contents =
        std::fs::read_to_string(GOALS_SOURCE).expect("Something went wrong reading the file");

    let mut goalsets = Vec::new();

    let sets = contents
        .split("###")
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();
    for set in sets.iter() {
        let lines = set.lines().filter(|s| !s.is_empty()).collect::<Vec<&str>>();

        let header = lines[0].split(" - ").collect::<Vec<&str>>();

        let date = header[0].to_string();
        let commit = header[1].to_string();
        let goals = lines[1..lines.len() - 1].join("\n");

        goalsets.push(Goalset::new(goals, date, commit));
    }

    goalsets
}

pub fn print_goals() {
    let goalsets = read_goals();

    for goalset in goalsets.iter() {
        println!("### {} - {}", goalset.date, goalset.commit);
        println!("{}", goalset.goals);
    }
}
