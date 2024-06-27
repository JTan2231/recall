// where should this go?
fn green(c: char) -> String {
    format!("\x1b[32m{}\x1b[0m", c)
}

fn red(c: char) -> String {
    format!("\x1b[31m{}\x1b[0m", c)
}

struct Pair {
    first: i32,
    second: i32,
}

impl Pair {
    fn new(f: i32, s: i32) -> Pair {
        Pair {
            first: f,
            second: s,
        }
    }
}

impl Clone for Pair {
    fn clone(&self) -> Pair {
        Pair {
            first: self.first,
            second: self.second,
        }
    }
}

pub struct DiffChar {
    pub value: char,
    pub source_index: usize,
    pub changed_index: usize,
}

impl DiffChar {
    fn new(v: char, s: usize, c: usize) -> DiffChar {
        DiffChar {
            value: v,
            source_index: s,
            changed_index: c,
        }
    }
}

impl Clone for DiffChar {
    fn clone(&self) -> DiffChar {
        DiffChar {
            value: self.value,
            source_index: self.source_index,
            changed_index: self.changed_index,
        }
    }
}

pub struct Diff {
    pub source: String,
    pub changed: String,
    pub diff: Vec<DiffChar>,
}

impl Diff {
    fn new(s: String, c: String, d: Vec<DiffChar>) -> Diff {
        Diff {
            source: s,
            changed: c,
            diff: d,
        }
    }

    pub fn print(&self) {
        let mut diff_index = 0;
        let mut s = 0;
        let mut c = 0;
        while s < self.source.len() || c < self.source.len() {}
    }
}

// how do we get the smallest possible set of changes
// from the longest common subsequence?
pub fn diff(source: String, changed: String) -> Diff {
    let mut memo = vec![vec![Pair::new(0, 0); changed.len() + 1]; source.len() + 1];

    let mut max_spot = Pair::new(0, 0);
    for (i, c) in source.chars().enumerate() {
        for (j, d) in changed.chars().enumerate() {
            if c == d {
                memo[i + 1][j + 1] = Pair::new(memo[i][j].first + 1, i as i32);
                max_spot = Pair::new(i as i32 + 1, j as i32 + 1);
            } else {
                if memo[i][j + 1].first > memo[i + 1][j].first {
                    memo[i + 1][j + 1] = Pair::new(memo[i][j + 1].first, memo[i][j + 1].second);
                } else {
                    memo[i + 1][j + 1] = Pair::new(memo[i + 1][j].first, memo[i + 1][j].second);
                }
            }
        }
    }

    let mut lcs = Vec::new();

    // crawl the memo table to build the lcs
    let mut i = max_spot.first as usize;
    let mut j = max_spot.second as usize;
    while i > 0 && j > 0 {
        if memo[i][j].first == memo[i - 1][j].first {
            i -= 1;
        } else if memo[i][j].first == memo[i][j - 1].first {
            j -= 1;
        } else {
            lcs.insert(
                0,
                DiffChar::new(source.chars().nth(i - 1).unwrap(), i - 1, j - 1),
            );
            i -= 1;
            j -= 1;
        }
    }

    Diff::new(source, changed, lcs)
}
