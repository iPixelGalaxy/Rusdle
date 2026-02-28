use crate::words::{is_valid_word, random_answer};

pub const WORD_LENGTH: usize = 5;
pub const MAX_GUESSES: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileState {
    Empty,
    Typing,
    Correct,   // Green: right letter, right position
    Present,   // Yellow: right letter, wrong position
    Absent,    // Gray: letter not in word
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub ch: char,
    pub state: TileState,
}

impl Default for Tile {
    fn default() -> Self {
        Self { ch: ' ', state: TileState::Empty }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    Playing,
    Won,
    Lost,
}

pub struct GameState {
    pub answer: String,
    pub guesses: Vec<[Tile; WORD_LENGTH]>,
    pub current_row: usize,
    pub current_input: String,
    pub status: GameStatus,
    pub message: Option<String>,
    pub message_timer: f32,
    /// Win/loss message held back until the flip animation finishes
    pub deferred_message: Option<String>,
    // Per-letter keyboard state
    pub keyboard: std::collections::HashMap<char, TileState>,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        let answer = random_answer(seed).to_string();
        Self {
            answer,
            guesses: (0..MAX_GUESSES).map(|_| Default::default()).collect(),
            current_row: 0,
            current_input: String::new(),
            status: GameStatus::Playing,
            message: None,
            message_timer: 0.0,
            deferred_message: None,
            keyboard: std::collections::HashMap::new(),
        }
    }

    pub fn reset(&mut self, seed: u64) {
        *self = GameState::new(seed);
    }

    pub fn type_letter(&mut self, ch: char) {
        if self.status != GameStatus::Playing { return; }
        if self.current_input.len() < WORD_LENGTH {
            self.current_input.push(ch.to_ascii_lowercase());
            self.update_current_row_display();
        }
    }

    pub fn delete_letter(&mut self) {
        if self.status != GameStatus::Playing { return; }
        if !self.current_input.is_empty() {
            self.current_input.pop();
            self.update_current_row_display();
        }
    }

    pub fn submit_guess(&mut self) {
        if self.status != GameStatus::Playing { return; }
        if self.current_input.len() != WORD_LENGTH {
            self.show_message("Not enough letters");
            return;
        }
        if !is_valid_word(&self.current_input) {
            self.show_message("Not in word list");
            return;
        }

        let result = evaluate_guess(&self.current_input, &self.answer);
        self.guesses[self.current_row] = result;

        // Update keyboard state
        for tile in &self.guesses[self.current_row] {
            let ch = tile.ch;
            let new_state = tile.state;
            let current = self.keyboard.entry(ch).or_insert(TileState::Absent);
            // Upgrade state: Correct > Present > Absent
            match (*current, new_state) {
                (TileState::Correct, _) => {}
                (_, TileState::Correct) => *current = TileState::Correct,
                (TileState::Present, _) => {}
                (_, TileState::Present) => *current = TileState::Present,
                _ => *current = new_state,
            }
        }

        let won = self.guesses[self.current_row]
            .iter()
            .all(|t| t.state == TileState::Correct);

        self.current_row += 1;
        self.current_input.clear();

        if won {
            self.status = GameStatus::Won;
            let msgs = ["Genius!", "Magnificent!", "Impressive!", "Splendid!", "Great!", "Phew!"];
            let msg = msgs[(self.current_row - 1).min(msgs.len() - 1)];
            // Defer so the toast appears after the flip animation finishes
            self.deferred_message = Some(msg.to_string());
        } else if self.current_row >= MAX_GUESSES {
            self.status = GameStatus::Lost;
            self.deferred_message = Some(self.answer.to_uppercase());
        }
    }

    fn update_current_row_display(&mut self) {
        if self.current_row >= MAX_GUESSES { return; }
        let row = &mut self.guesses[self.current_row];
        for i in 0..WORD_LENGTH {
            if let Some(ch) = self.current_input.chars().nth(i) {
                row[i] = Tile { ch, state: TileState::Typing };
            } else {
                row[i] = Tile { ch: ' ', state: TileState::Empty };
            }
        }
    }

    fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 2.5;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.message_timer > 0.0 {
            self.message_timer -= dt;
            if self.message_timer <= 0.0 {
                self.message = None;
            }
        }
    }
}

fn evaluate_guess(guess: &str, answer: &str) -> [Tile; WORD_LENGTH] {
    let guess_chars: Vec<char> = guess.chars().collect();
    let answer_chars: Vec<char> = answer.chars().collect();
    let mut result: [Tile; WORD_LENGTH] = Default::default();
    let mut answer_used = [false; WORD_LENGTH];

    // First pass: find correct positions (green)
    for i in 0..WORD_LENGTH {
        result[i].ch = guess_chars[i];
        if guess_chars[i] == answer_chars[i] {
            result[i].state = TileState::Correct;
            answer_used[i] = true;
        }
    }

    // Second pass: find present letters (yellow)
    for i in 0..WORD_LENGTH {
        if result[i].state == TileState::Correct {
            continue;
        }
        let mut found = false;
        for j in 0..WORD_LENGTH {
            if !answer_used[j] && guess_chars[i] == answer_chars[j] {
                result[i].state = TileState::Present;
                answer_used[j] = true;
                found = true;
                break;
            }
        }
        if !found {
            result[i].state = TileState::Absent;
        }
    }

    result
}
