# Rusdle

A Wordle-inspired word puzzle game built with Rust and [egui](https://github.com/emilk/egui).

Guess the hidden 5-letter word in 6 tries. Each guess reveals which letters are
correct (green), present but misplaced (yellow), or absent (gray).

<img width="1465" height="776" alt="image" src="https://github.com/user-attachments/assets/9173e187-aa18-42cc-83a6-4b1958d65f65" />


---

## Features

- **Cross-platform** native GUI — Windows, macOS, Linux
- **Flip-tile animation** when a guess is submitted
- **Responsive layout** — grid and keyboard scale together with the window
- **Multiple profiles** with fully independent statistics
- **Persistent stats** — win %, streaks, guess distribution, fastest win, favourite starter word
- 2,314 possible answers and 14,855 valid guesses (the original Wordle word lists)

---

## Download

Grab the latest build from the [Releases page](../../releases/latest):

| Platform | File |
|---|---|
| Windows x64 | `rusdle-windows-x64.zip` — extract and run `rusdle.exe` |
| macOS (Apple Silicon + Intel) | `rusdle-macos-universal.dmg` — drag **Rusdle.app** to Applications |
| Linux x64 | `rusdle-linux-x64.tar.gz` — extract and run `./rusdle` |

> **macOS note:** If the system blocks the app with an "unidentified developer"
> warning, right-click the app → **Open**, then click **Open** in the dialog.

---

## Controls

| Input | Action |
|---|---|
| **A – Z** | Type a letter |
| **Enter** | Submit guess |
| **Backspace** | Delete last letter |

You can also click the on-screen keyboard with a mouse or touch.

---

## Profiles & Stats

Click **PROFILES** (top-left header button) to create or switch between profiles.
Each profile tracks its own:

- Games played / Win %
- Current & max win streak
- Guess distribution (1–6)
- Fastest winning game
- Total time played
- Favourite starter word

---

## Building from Source

Requires [Rust](https://rustup.rs) stable (1.70+).

**Linux** — install system dependencies first:

```bash
sudo apt-get install \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libgtk-3-dev libfontconfig1-dev
```

**All platforms:**

```bash
git clone https://github.com/<you>/rusdle
cd rusdle
cargo build --release
# Output: target/release/rusdle  (rusdle.exe on Windows)
```

---

## Releasing

Push a version tag — the GitHub Actions workflow builds all three platform
binaries and creates a GitHub Release automatically:

```bash
git tag v1.0.0
git push origin v1.0.0
```

This produces:

| Artifact | Contents |
|---|---|
| `rusdle-windows-x64.zip` | `rusdle.exe` with embedded icon |
| `rusdle-macos-universal.dmg` | `Rusdle.app` (arm64 + x86_64 universal) |
| `rusdle-linux-x64.tar.gz` | `rusdle` binary |

---

## License

MIT
