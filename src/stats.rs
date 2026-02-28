use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn epoch_day() -> u64 {
    now_secs() / 86400
}

fn save_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Rusdle").join("profiles.json"))
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Stats {
    pub games_played: u32,
    pub games_won: u32,
    pub current_streak: u32,
    pub max_streak: u32,
    /// Index 0 = won in 1 guess … index 5 = won in 6 guesses
    pub guess_dist: [u32; 6],
    /// Fastest winning game in seconds (None if never won)
    pub fastest_win_secs: Option<u64>,
    /// Sum of guesses used across all wins (for average calculation)
    pub total_guesses_on_wins: u32,
    /// Total seconds spent actively playing
    pub total_time_secs: u64,
    /// Epoch-day when the last game was completed (streak logic)
    pub last_epoch_day: Option<u64>,
    /// How many times each word was typed as the first guess
    pub first_word_freq: HashMap<String, u32>,
}

impl Stats {
    pub fn win_pct(&self) -> f32 {
        if self.games_played == 0 { 0.0 }
        else { self.games_won as f32 / self.games_played as f32 * 100.0 }
    }

    pub fn avg_guesses(&self) -> Option<f32> {
        if self.games_won == 0 { None }
        else { Some(self.total_guesses_on_wins as f32 / self.games_won as f32) }
    }

    /// Format fastest win as "M:SS"
    pub fn fastest_fmt(&self) -> Option<String> {
        self.fastest_win_secs.map(|s| format!("{}:{:02}", s / 60, s % 60))
    }

    /// Total time formatted as "Xh Ym" or "Ym Zs"
    pub fn total_time_fmt(&self) -> String {
        let h = self.total_time_secs / 3600;
        let m = (self.total_time_secs % 3600) / 60;
        let s = self.total_time_secs % 60;
        if h > 0      { format!("{}h {}m", h, m) }
        else if m > 0 { format!("{}m {}s", m, s) }
        else          { format!("{}s", s) }
    }

    /// Most frequently used first-guess word (uppercase)
    pub fn favorite_start(&self) -> Option<String> {
        self.first_word_freq
            .iter()
            .max_by_key(|(_, &v)| v)
            .map(|(k, _)| k.to_uppercase())
    }

    /// Record the outcome of a completed game.
    ///
    /// * `won`          – whether the player guessed correctly
    /// * `guesses_used` – number of rows used (1–6)
    /// * `elapsed_secs` – wall-clock seconds from first key-press to submission
    /// * `first_word`   – the first guess word (lowercase)
    pub fn record_game(
        &mut self,
        won: bool,
        guesses_used: u32,
        elapsed_secs: u64,
        first_word: Option<&str>,
    ) {
        self.games_played += 1;
        self.total_time_secs += elapsed_secs;

        let today     = epoch_day();
        let yesterday = today.saturating_sub(1);

        if won {
            self.games_won += 1;
            self.total_guesses_on_wins += guesses_used;

            let idx = guesses_used.saturating_sub(1).min(5) as usize;
            self.guess_dist[idx] += 1;

            self.current_streak = match self.last_epoch_day {
                Some(d) if d == yesterday => self.current_streak + 1,
                Some(d) if d == today    => self.current_streak, // same-day replay, no change
                _                        => 1,
            };
            self.max_streak = self.max_streak.max(self.current_streak);

            let t = elapsed_secs.max(1);
            self.fastest_win_secs = Some(self.fastest_win_secs.map_or(t, |p| p.min(t)));
        } else {
            if self.last_epoch_day.map_or(true, |d| d != today) {
                self.current_streak = 0;
            }
        }

        self.last_epoch_day = Some(today);

        if let Some(w) = first_word {
            *self.first_word_freq.entry(w.to_lowercase()).or_default() += 1;
        }
    }
}

// ── Profile ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub stats: Stats,
    pub created_epoch: u64,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), stats: Stats::default(), created_epoch: now_secs() }
    }
}

// ── ProfileManager ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct ProfileManager {
    pub active_name: String,
    pub profiles: Vec<Profile>,
}

impl Default for ProfileManager {
    fn default() -> Self { Self::new() }
}

impl ProfileManager {
    pub fn new() -> Self {
        let p = Profile::new("Player 1");
        let name = p.name.clone();
        Self { active_name: name, profiles: vec![p] }
    }

    pub fn active(&self) -> &Profile {
        self.profiles.iter().find(|p| p.name == self.active_name)
            .unwrap_or(&self.profiles[0])
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        let name = self.active_name.clone();
        if let Some(i) = self.profiles.iter().position(|p| p.name == name) {
            &mut self.profiles[i]
        } else {
            &mut self.profiles[0]
        }
    }

    pub fn switch_to(&mut self, name: &str) {
        if self.profiles.iter().any(|p| p.name == name) {
            self.active_name = name.to_string();
        }
    }

    pub fn create_profile(&mut self, name: &str) -> Result<(), &'static str> {
        let n = name.trim();
        if n.is_empty()  { return Err("Name cannot be empty"); }
        if n.len() > 20  { return Err("Max 20 characters"); }
        if self.profiles.iter().any(|p| p.name.to_lowercase() == n.to_lowercase()) {
            return Err("Name already taken");
        }
        let p = Profile::new(n);
        self.active_name = p.name.clone();
        self.profiles.push(p);
        Ok(())
    }

    /// Delete a profile by name. Returns false if it's the last profile.
    pub fn delete_profile(&mut self, name: &str) -> bool {
        if self.profiles.len() <= 1 { return false; }
        let was_active = self.active_name == name;
        self.profiles.retain(|p| p.name != name);
        if was_active {
            self.active_name = self.profiles[0].name.clone();
        }
        true
    }

    pub fn save(&self) {
        let Some(path) = save_path() else { return };
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn load() -> Self {
        save_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::new)
    }
}
