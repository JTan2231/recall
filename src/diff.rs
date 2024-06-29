use std::collections::HashSet;

// where should this go?
fn green(c: char) -> String {
    format!("\x1b[32m{}\x1b[0m", c)
}

fn red(c: char) -> String {
    format!("\x1b[31m{}\x1b[0m", c)
}

pub struct Pair {
    pub first: i32,
    pub second: i32,
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

pub struct LCSChar {
    pub value: char,
    pub source_index: usize,
    pub changed_index: usize,
}

impl LCSChar {
    fn new(v: char, s: usize, c: usize) -> LCSChar {
        LCSChar {
            value: v,
            source_index: s,
            changed_index: c,
        }
    }
}

impl Clone for LCSChar {
    fn clone(&self) -> LCSChar {
        LCSChar {
            value: self.value,
            source_index: self.source_index,
            changed_index: self.changed_index,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffCharType {
    Addition,
    Deletion,
}

impl Clone for DiffCharType {
    fn clone(&self) -> DiffCharType {
        match self {
            DiffCharType::Addition => DiffCharType::Addition,
            DiffCharType::Deletion => DiffCharType::Deletion,
        }
    }
}

#[derive(Debug)]
pub struct DiffChar {
    pub value: char,
    pub index: usize,
    pub char_type: DiffCharType,
}

impl Clone for DiffChar {
    fn clone(&self) -> DiffChar {
        DiffChar {
            value: self.value,
            index: self.index,
            char_type: self.char_type.clone(),
        }
    }
}

pub struct Rope {
    pub addition: String,
    pub range: (usize, usize),
}

impl Clone for Rope {
    fn clone(&self) -> Rope {
        Rope {
            addition: self.addition.clone(),
            range: self.range,
        }
    }
}

pub struct Diff {
    pub source: String,
    pub changed: String,
    pub lcs: Vec<LCSChar>,
    pub diff: Vec<DiffChar>,
}

impl Diff {
    pub fn new(source: String, changed: String, lcs: Vec<LCSChar>) -> Diff {
        Diff {
            source,
            changed,
            lcs,
            diff: Vec::new(),
        }
    }

    pub fn build(&mut self) {
        let mut diff: Vec<DiffChar> = Vec::new();

        let mut source_idx = 0;
        let mut changed_idx = 0;
        let mut diff_idx = 0;

        let is_bounded = |s, c| s < self.source.len() && c < self.changed.len();

        while is_bounded(source_idx, changed_idx) {
            source_idx = self.parse_source_removal(source_idx, diff_idx, &mut |c, i| {
                diff.push(DiffChar {
                    value: c,
                    index: i,
                    char_type: DiffCharType::Deletion,
                });
            });
            changed_idx = self.parse_changed_addition(changed_idx, diff_idx, &mut |c, i| {
                diff.push(DiffChar {
                    value: c,
                    index: i,
                    char_type: DiffCharType::Addition,
                });
            });
            let (new_source_idx, new_changed_idx, new_diff_idx) =
                self.parse_common_subsequence(source_idx, changed_idx, diff_idx, &mut |_c, _i| {});

            source_idx = new_source_idx;
            changed_idx = new_changed_idx;
            diff_idx = new_diff_idx;
        }

        diff.sort_by(|a, b| b.char_type.cmp(&a.char_type).then(a.index.cmp(&b.index)));
        self.diff = diff;
    }

    pub fn to_string(&self) -> String {
        let mut source_idx = 0;
        let mut changed_idx = 0;
        let mut diff_idx = 0;

        let is_bounded = |s, c| s < self.source.len() && c < self.changed.len();

        let mut diff_string = String::new();
        while is_bounded(source_idx, changed_idx) {
            source_idx = self.parse_source_removal(source_idx, diff_idx, &mut |c, _i| {
                diff_string.push_str(&red(c))
            });
            changed_idx = self.parse_changed_addition(changed_idx, diff_idx, &mut |c, i| {
                diff_string.push_str(&green(c))
            });
            let (new_source_idx, new_changed_idx, new_diff_idx) =
                self.parse_common_subsequence(source_idx, changed_idx, diff_idx, &mut |c, i| {
                    diff_string.push(c)
                });

            source_idx = new_source_idx;
            changed_idx = new_changed_idx;
            diff_idx = new_diff_idx;
        }

        diff_string
    }

    pub fn print(&self) {
        println!("{}", self.to_string());
    }

    fn parse_source_removal(
        &self,
        mut source_idx: usize,
        diff_idx: usize,
        action: &mut dyn FnMut(char, usize) -> (),
    ) -> usize {
        while source_idx < self.source.len()
            && (diff_idx >= self.lcs.len() || source_idx < self.lcs[diff_idx].source_index)
        {
            if let Some(ch) = self.source.chars().nth(source_idx) {
                action(ch, source_idx);
            }

            source_idx += 1;
        }

        source_idx
    }

    fn parse_changed_addition(
        &self,
        mut changed_idx: usize,
        diff_idx: usize,
        action: &mut dyn FnMut(char, usize) -> (),
    ) -> usize {
        while changed_idx < self.changed.len()
            && (diff_idx >= self.lcs.len() || changed_idx < self.lcs[diff_idx].changed_index)
        {
            if let Some(ch) = self.changed.chars().nth(changed_idx) {
                action(ch, changed_idx);
            }

            changed_idx += 1;
        }

        changed_idx
    }

    fn parse_common_subsequence(
        &self,
        mut source_idx: usize,
        mut changed_idx: usize,
        mut diff_idx: usize,
        action: &mut dyn FnMut(char, usize) -> (),
    ) -> (usize, usize, usize) {
        while diff_idx < self.lcs.len()
            && source_idx == self.lcs[diff_idx].source_index
            && changed_idx == self.lcs[diff_idx].changed_index
        {
            if let Some(ch) = self.source.chars().nth(source_idx) {
                action(ch, 0);
            }

            source_idx += 1;
            changed_idx += 1;
            diff_idx += 1;
        }

        (source_idx, changed_idx, diff_idx)
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
                LCSChar::new(source.chars().nth(i - 1).unwrap(), i - 1, j - 1),
            );
            i -= 1;
            j -= 1;
        }
    }

    lcs.sort_by(|a, b| a.changed_index.cmp(&b.changed_index));

    let mut new_diff = Diff::new(source, changed, lcs);
    new_diff.build();

    new_diff
}
