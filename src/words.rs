use std::collections::HashSet;
use std::sync::OnceLock;

// Embedded at compile time — no runtime file I/O needed
const ANSWERS_STR: &str = include_str!("../assets/words/answers.txt");
const VALID_STR:   &str = include_str!("../assets/words/valid.txt");

static ANSWERS_LIST: OnceLock<Vec<&'static str>> = OnceLock::new();
static VALID_SET:    OnceLock<HashSet<&'static str>> = OnceLock::new();

fn answers() -> &'static Vec<&'static str> {
    ANSWERS_LIST.get_or_init(|| ANSWERS_STR.lines().filter(|l| l.len() == 5).collect())
}

fn valid_set() -> &'static HashSet<&'static str> {
    VALID_SET.get_or_init(|| {
        let mut set = HashSet::new();
        for w in ANSWERS_STR.lines().filter(|l| l.len() == 5) { set.insert(w); }
        for w in VALID_STR.lines().filter(|l| l.len() == 5)   { set.insert(w); }
        set
    })
}

pub fn is_valid_word(word: &str) -> bool {
    valid_set().contains(word)
}

pub fn random_answer(seed: u64) -> &'static str {
    let list = answers();
    list[(seed as usize) % list.len()]
}
