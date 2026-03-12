# `music-cli`

Lightweight terminal music player (CLI + interactive TUI).

`music-cli` scans a folder on disk, builds a local library (stored as JSON in your OS config
directory), and lets you play music either via simple terminal playback or a full-screen
browser UI.

## Features

- Play a single audio file: `music-cli play <FILE>`
- Local library scanning: set a root folder (`path`) and scan it (`refresh`)
- Library playback: `playlist` (simple terminal UI with playback controls)
- Full-screen interactive browser: `browse` (TUI)
- Fuzzy search across **title**, **artist**, and **album**
- Sorting by **title**, **artist**, **album**, or **duration**
- Shuffle + repeat modes + volume, persisted between runs
- Supported extensions: **mp3**, **flac**, **wav**, **ogg**

## Install

This is a Rust project. You can run it directly with Cargo, or build a release binary.

```bash
# Build a release binary
cargo build --release
./target/release/music-cli --help
```

Optional: install locally from the repo:

```bash
cargo install --path .
music-cli --help
```

## Quick start

1) Point `music-cli` at your music folder:

```bash
music-cli path /path/to/your/music
```

2) Scan it to build/update the library:

```bash
music-cli refresh
```

3) Start playback:

```bash
# Full-screen interactive browser (recommended)
music-cli browse

# Or: simple “play through the library” mode
music-cli playlist
```

## Commands

`music-cli --help` shows the full help text. These are the available subcommands:

- `play <FILE>`: play one audio file directly (does not use the library)
- `path <DIR>`: set the root music directory
- `refresh`: scan the configured root directory and rebuild the library
- `playlist`: play through the library (simple terminal UI)
- `list`: print the library as a list
- `search <QUERY>`: fuzzy search the library (title/artist/album)
- `select <INDEX>`: play one library entry by index (**0-based**, as printed by `search`)
- `sort [title|artist|album|duration]`: print the library sorted by a chosen field
- `browse`: open the interactive full-screen TUI browser/player
- `volume [0..100]`: set volume (or show current volume if omitted)
- `shuffle [true|false]`: toggle shuffle (or set it explicitly if provided)
- `loop [off|all|one]`: cycle repeat mode (or set it explicitly if provided)

### Examples

```bash
music-cli list
music-cli search "pink floyd wall"
music-cli select 42

music-cli volume 70
music-cli shuffle true
music-cli loop all

music-cli sort artist
```

## `browse` (TUI) key bindings

### Normal mode

- Navigate: `↑/↓` or `j/k`
- Play selected: `Enter`
- Pause/resume: `Space` or `p`
- Next/previous: `n` / `b` (also `→` / `←`)
- Toggle shuffle: `r`
- Cycle sort field: `o`
- Refresh library scan: `F5` or `u`
- Search: `/` or `Ctrl+f`
- Settings: `s`
- Quit: `q`, `Esc`, or `Ctrl+c`

### Search mode

- Type to search (fuzzy)
- Clear query: `Ctrl+u`
- Backspace: delete last character
- Navigate results: `↑/↓`
- Play selected: `Enter`
- Pause/resume: `Ctrl+Space`
- Exit search: `Esc`

### Settings modal

- Open/close: `s` (or close with `Esc`)
- Navigate fields: `↑/↓` or `j/k`
- **Volume**: `Enter` to edit, `←/→` adjusts by 5, digits type a value, `Enter` confirm, `Esc` cancel
- **Repeat**: `Enter`/`→` cycles forward, `←` cycles backward
- **Music path**: `Enter` to edit, type a path, `Enter` to confirm (validated), `Esc` cancel, `Ctrl+u` clear

## Data storage

`music-cli` stores its state (library + settings like volume/shuffle/repeat/path) in:

- `<config dir>/music-cli/db.json`

The *config dir* is your OS config directory as reported by `dirs::config_dir()` (it differs
across platforms).

If the file becomes corrupted, `music-cli` will try to recover what it can; otherwise it
backs it up as `db.json.bak` and starts with defaults.

## Supported audio files

Library scanning includes files with these extensions (case-insensitive):

- `mp3`, `flac`, `wav`, `ogg`

Metadata (title/artist/album/duration) is read when available; otherwise the filename is
used as the title.

## Development

```bash
cargo test
cargo run -- --help
```

## Troubleshooting

- “No music path set”: run `music-cli path <DIR>` (or set it in `browse` → Settings → Music path)
- “Library is empty”: run `music-cli refresh`
- `select` fails with “Invalid index …”: use `music-cli search <QUERY>` to find the correct **0-based** index
