# techō

techō (手帳, てちょう) is a small terminal journal inspired by the everyday pleasure of a paper planner. Keep an experiment log on a Linux research machine, jot down a thought over SSH, or plan a day locally. Everything stays in daily Markdown files; no account or network connection is needed.

This is `0.1.0-alpha.2`, an early version for hands-on testing.

## Run

With a current stable Rust toolchain and a C linker installed:

```sh
cargo run --locked
# Or install this checkout, then run from anywhere:
cargo install --path . --locked
techo
```

Use an interactive UTF-8 terminal. Mouse input is optional; every primary operation works with the keyboard. On SSH, mouse support depends on the local terminal forwarding mouse events. `techo --no-mouse` leaves mouse selection to the terminal. When mouse capture is enabled, Shift+drag commonly selects terminal text.

On Windows with Visual C++ Build Tools, run `./dev.ps1` from PowerShell. This development helper opens the checkout's existing `logs` directory. Use a standalone terminal window for interactive testing.

```sh
techo --data-dir ./logs        # existing alpha.1 checkout journals
techo --date 2027-02-01        # open a specific date
techo --help
```

## Your day

Free memo is selected when techo opens: press Enter and start writing.

- **Schedule:** press `s`, then `n`; enter `HH:MM`, press Tab, and write the item. Ctrl+S saves. Entries are sorted by time and only existing entries occupy space. Enter edits the selected entry, including its time. A `↵` preview marker means there is more than one line; the editor shows the full text.
- **Todo:** press `t`, then `n` to add; Space checks an item, Enter edits it, and `d` opens a deletion confirmation.
- **Free memo:** press `f`, then Enter to write. This has most of the page's writing space. Up/Down or the wheel scrolls the saved memo.

Click a panel, including its border or empty area, to select it. Click a list row to select an entry. The active panel has a **`>` title marker and thick border**, so it stays recognizable without color. Tab/Shift+Tab also switches panels. Small terminals display the active panel alone when necessary.

In the editor, **Ctrl+S saves and Esc cancels**. Enter inserts a new line. Arrow keys, Home/End, Delete and Backspace move and edit the text; Ctrl+Home/End jumps to the beginning/end. Multiline paste is supported. Text scrolls to keep the cursor visible. For schedule entries, Tab switches between time and item fields. Buttons can also be clicked.

The paper day keeps the original **04:00–03:59** convention. A time such as `00:30 (+1)` belongs to the following morning of the opened journal date. Multiple items at the same time are allowed.

## Dates and calendar

Press `y`, or click the month-view title, to open the year calendar. Click a date to open it, or use arrows and Enter. `[` / `]` changes the year; PageUp/PageDown or the wheel moves between calendar pages. All twelve months appear together when there is enough space; smaller terminals paginate them. `t` selects today in the calendar; Esc returns without changing the open journal.

Press `g` to jump directly to a `YYYY-MM-DD` date. On the day page, `[` / `]` changes the day and Home opens today. The calendar distinguishes the selected date `[dd]`, today `(dd)`, and the open journal date `{dd}`; selected takes precedence when these coincide.

The date header includes an **approximate moon phase**, calculated offline for the selected date at 12:00 UTC. It uses a mean 29.530588-day cycle anchored to the 2000-01-06 18:14 UTC new moon in [NASA's phase tables](https://eclipse.gsfc.nasa.gov/phase/phases1901.html). It is a daily journal detail, not a precise astronomical event time; actual phases and local-day boundaries can differ.

Press `?` for help and `q` to quit outside the editor. Resizing below 32 columns × 14 rows displays a resize message while retaining the draft.

## Files and safe saving

The data directory is independent of the current working directory:

1. `--data-dir PATH`
2. `TECHO_DIR`
3. Linux: `$XDG_DATA_HOME/techo/journals`, otherwise `~/.local/share/techo/journals`; Windows: `%LOCALAPPDATA%/techo/journals`.

Each date has one `YYYY-MM-DD.md` file. Browsing an empty date does not create it; saving does. A directory lock prevents two new techo instances editing the same journal directory. Saves write and sync a temporary file before replacing the journal. Save errors keep the editor and draft open. If the file has changed externally, techo refuses to overwrite it; preserve the draft before reopening.

Existing alpha.1 journals are readable. On their first save, their exact previous contents are retained in `YYYY-MM-DD.md.alpha.bak`. The new Markdown format records `techo-format: 2` in frontmatter: multiline todo continuations are indented by two spaces; schedule body lines by four spaces; Free Memo is the final section and permits arbitrary headings and blank lines. Do not edit the same journal concurrently in another application. Alpha.1 should not be used to edit files saved in the new format.

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 scripts/terminal_smoke.py target/debug/techo  # Linux PTY acceptance
```

Windows checks: `./dev.ps1 test` and `./dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')`.

The [interaction feedback](docs/interaction-feedback.md) records the requested changes. [Implementation notes](docs/interaction-feedback-impl-notes.md) track decisions and validation.
