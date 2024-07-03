use std::collections::BTreeSet;

// where should this go?
fn green(c: char) -> String {
    format!("\x1b[32m{}\x1b[0m", c)
}

fn red(c: char) -> String {
    format!("\x1b[31m{}\x1b[0m", c)
}

pub struct Pair<T, U> {
    pub first: T,
    pub second: U,
}

impl<T, U> Pair<T, U> {
    pub fn new(first: T, second: U) -> Pair<T, U> {
        Pair { first, second }
    }
}

impl<T, U> Clone for Pair<T, U>
where
    T: Clone,
    U: Clone,
{
    fn clone(&self) -> Pair<T, U> {
        Pair {
            first: self.first.clone(),
            second: self.second.clone(),
        }
    }
}

struct Index {
    line: usize,
    column: usize,
    flat: usize,
}

impl Clone for Index {
    fn clone(&self) -> Index {
        Index {
            line: self.line,
            column: self.column,
            flat: self.flat,
        }
    }
}

struct IndexedString {
    pub content: String,
    pub indices: Vec<Index>,
}

impl IndexedString {
    fn new(content: String) -> IndexedString {
        let mut indices = Vec::new();
        let mut line = 0;
        let mut column = 0;
        let mut flat = 0;
        for c in content.chars() {
            indices.push(Index { line, column, flat });

            flat += 1;
            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        IndexedString { content, indices }
    }

    fn len(&self) -> usize {
        self.content.len()
    }

    fn get_line(&self, line_number: usize) -> String {
        match self
            .indices
            .binary_search_by(|ind| ind.line.cmp(&line_number))
        {
            Ok(index) => {
                let mut line = String::new();
                let mut i = index;

                while i != usize::MAX && self.indices[i].line == line_number {
                    i = i.wrapping_sub(1);
                }

                i = i.wrapping_add(1);
                while i < self.indices.len() && self.indices[i].line == line_number {
                    line.push(self.content.chars().nth(i).unwrap());
                    i += 1;
                }

                line
            }
            Err(_index) => {
                eprintln!("line {} not found", line_number);
                String::new()
            }
        }
    }
}

pub struct LCSChar {
    pub value: char,
    pub source_index: Index,
    pub changed_index: Index,
}

impl LCSChar {
    fn new(v: char, s: Index, c: Index) -> LCSChar {
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
            source_index: self.source_index.clone(),
            changed_index: self.changed_index.clone(),
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

pub struct DiffChar {
    pub value: char,
    pub index: Index,
    pub char_type: DiffCharType,
}

impl Clone for DiffChar {
    fn clone(&self) -> DiffChar {
        DiffChar {
            value: self.value,
            index: self.index.clone(),
            char_type: self.char_type.clone(),
        }
    }
}

pub struct Diff {
    pub source: IndexedString,
    pub changed: IndexedString,
    pub lcs: Vec<LCSChar>,
    pub diff: Vec<DiffChar>,
}

impl Diff {
    pub fn new(source: IndexedString, changed: IndexedString, lcs: Vec<LCSChar>) -> Diff {
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
            changed_idx = self.parse_changed_addition(changed_idx, diff_idx, &mut |c, _i| {
                diff_string.push_str(&green(c))
            });
            let (new_source_idx, new_changed_idx, new_diff_idx) =
                self.parse_common_subsequence(source_idx, changed_idx, diff_idx, &mut |c, _i| {
                    diff_string.push(c)
                });

            source_idx = new_source_idx;
            changed_idx = new_changed_idx;
            diff_idx = new_diff_idx;
        }

        diff_string
    }

    pub fn to_pretty_string(&self) -> String {
        let mut source_idx = 0;
        let mut changed_idx = 0;
        let mut diff_idx = 0;

        let is_bounded = |s, c| s < self.source.len() && c < self.changed.len();

        let mut added_lines = BTreeSet::new();
        let mut removed_lines = BTreeSet::new();
        let mut remained_lines = BTreeSet::new();

        while is_bounded(source_idx, changed_idx) {
            source_idx = self.parse_source_removal(source_idx, diff_idx, &mut |_c, i| {
                removed_lines.insert(i.line);
            });
            changed_idx = self.parse_changed_addition(changed_idx, diff_idx, &mut |_c, i| {
                added_lines.insert(i.line);
            });
            let (new_source_idx, new_changed_idx, new_diff_idx) =
                self.parse_common_subsequence(source_idx, changed_idx, diff_idx, &mut |_c, i| {
                    remained_lines.insert(i.line);
                });

            source_idx = new_source_idx;
            changed_idx = new_changed_idx;
            diff_idx = new_diff_idx;
        }

        let line_count = std::cmp::max(
            self.source.indices.last().unwrap().line,
            self.changed.indices.last().unwrap().line,
        );

        let added_lines = added_lines.iter().collect::<Vec<&usize>>();
        let removed_lines = removed_lines.iter().collect::<Vec<&usize>>();
        let remained_lines = remained_lines.iter().collect::<Vec<&usize>>();

        let mut added_index = 0;
        let mut removed_index = 0;
        let mut remained_index = 0;

        let is_bounded = |i: usize, c: usize, v: &Vec<&usize>| i < line_count && c < v.len();
        let mut diff_string = String::new();

        let parse_changes = |line_index: usize,
                             index: &mut usize,
                             lines: &Vec<&usize>,
                             diff_string: &mut String,
                             change_type: DiffCharType,
                             source: &IndexedString| {
            let mut contiguous_index = line_index;
            let mut lines_skipping = 0;
            while is_bounded(contiguous_index, *index, lines) && *lines[*index] == contiguous_index
            {
                let color = match change_type {
                    DiffCharType::Addition => green,
                    DiffCharType::Deletion => red,
                };

                let char_type = match change_type {
                    DiffCharType::Addition => '+',
                    DiffCharType::Deletion => '-',
                };

                diff_string.push_str(&(color(char_type) + " "));
                let line = source.get_line(*lines[*index]);
                for c in line.chars() {
                    diff_string.push_str(&color(c));
                }

                *index += 1;
                contiguous_index += 1;
                if change_type == DiffCharType::Deletion {
                    lines_skipping += 1;
                }
            }

            lines_skipping
        };

        for i in 0..line_count {
            let lines_skipping = parse_changes(
                i,
                &mut removed_index,
                &removed_lines,
                &mut diff_string,
                DiffCharType::Deletion,
                &self.source,
            );

            remained_index += lines_skipping;

            parse_changes(
                i,
                &mut added_index,
                &added_lines,
                &mut diff_string,
                DiffCharType::Addition,
                &self.changed,
            );

            while is_bounded(i, remained_index, &remained_lines)
                && *remained_lines[remained_index] == i
            {
                diff_string.push(' ');
                let line = self.source.get_line(i);
                diff_string.push_str(&line);

                remained_index += 1;
            }
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
        action: &mut dyn FnMut(char, Index) -> (),
    ) -> usize {
        while source_idx < self.source.len()
            && (diff_idx >= self.lcs.len()
                || source_idx < self.lcs[diff_idx].source_index.flat as usize)
        {
            if let Some(ch) = self.source.content.chars().nth(source_idx) {
                action(ch, self.source.indices[source_idx].clone());
            }

            source_idx += 1;
        }

        source_idx
    }

    fn parse_changed_addition(
        &self,
        mut changed_idx: usize,
        diff_idx: usize,
        action: &mut dyn FnMut(char, Index) -> (),
    ) -> usize {
        while changed_idx < self.changed.len()
            && (diff_idx >= self.lcs.len()
                || changed_idx < self.lcs[diff_idx].changed_index.flat as usize)
        {
            if let Some(ch) = self.changed.content.chars().nth(changed_idx) {
                action(ch, self.changed.indices[changed_idx].clone());
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
        action: &mut dyn FnMut(char, Index) -> (),
    ) -> (usize, usize, usize) {
        while diff_idx < self.lcs.len()
            && source_idx == self.lcs[diff_idx].source_index.flat as usize
            && changed_idx == self.lcs[diff_idx].changed_index.flat as usize
        {
            if let Some(ch) = self.source.content.chars().nth(source_idx) {
                action(ch, self.source.indices[source_idx].clone());
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
    let mut memo = vec![
        vec![
            Pair::new(
                0,
                Index {
                    line: 0,
                    column: 0,
                    flat: 0
                }
            );
            changed.len() + 1
        ];
        source.len() + 1
    ];

    let source = IndexedString::new(source);
    let changed = IndexedString::new(changed);

    let source_bytes = source.content.as_bytes();
    let changed_bytes = changed.content.as_bytes();

    let mut max_spot = Pair::new(0, 0);
    for i in 0..source.content.len() {
        for j in 0..changed.content.len() {
            let c = source_bytes[i] as char;
            let d = changed_bytes[j] as char;
            if c == d {
                memo[i + 1][j + 1] = Pair::new(memo[i][j].first + 1, source.indices[i].clone());
                max_spot = Pair::new(i + 1, j + 1);
            } else {
                if memo[i][j + 1].first > memo[i + 1][j].first {
                    memo[i + 1][j + 1] =
                        Pair::new(memo[i][j + 1].first, memo[i][j + 1].second.clone());
                } else {
                    memo[i + 1][j + 1] =
                        Pair::new(memo[i + 1][j].first, memo[i + 1][j].second.clone());
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
                LCSChar::new(
                    source.content.chars().nth(i - 1).unwrap(),
                    source.indices[i - 1].clone(),
                    changed.indices[j - 1].clone(),
                ),
            );
            i -= 1;
            j -= 1;
        }
    }

    lcs.sort_by(|a, b| a.changed_index.flat.cmp(&b.changed_index.flat));

    let mut new_diff = Diff::new(source, changed, lcs);
    new_diff.build();

    new_diff
}
