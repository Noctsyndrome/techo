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

Use an interactive UTF-8 terminal. Mouse input is optional; every operation works with the keyboard. On SSH, mouse support depends on the local terminal forwarding mouse events. `techo --no-mouse` leaves mouse selection to the terminal. When mouse capture is enabled, Shift+drag commonly selects terminal text.

On Windows with Visual C++ Build Tools, run `./dev.ps1` from PowerShell. This development helper opens the checkout's `logs` directory. Use a standalone terminal window for interactive testing.

```sh
techo --data-dir ./logs        # checkout journals
techo --date 2027-02-01        # open a specific date
techo --help
```

## The page

A day is one page. On the left: the date, a ruled schedule with hour marks down its gutter, and free memo. On the right: todo, the month, and a line of words for the day. Panels are ruled boxes; the active one shows its title as a small tab in the month's colour. When a panel holds more than it shows, a small `⋯` sits on its bottom rule, and on its top rule once you have scrolled past the start. The first launch shows the key reference once; `?` brings it back.

**Schedule.** Items in time order, each with its time in the gutter; items at the same time gather under one label. `n` opens a small note on the page with the time as its title: on today's page it starts at the current time, next to an existing item it takes that item's time. Write, then Tab into the title if the time needs changing; `9`, `930` and `09:30` are all read. Enter on an item edits it, `d` removes it, Up/Down move by item. Down past the last item starts a new one, at the time now on today's page; the same works in todo. The paper day runs from 04:00 to 03:59.

**Todo.** `n` adds, Space checks, Enter edits, `d` removes. The note opens on the todo row.

**Free memo.** Enter writes in place. Up/Down or the wheel scrolls.

`s`, `t`, `f` or Tab move between panels; clicking a panel, a line, or a day works as well. Writing never leaves the page: Ctrl+S or Ctrl+Enter saves and Esc cancels, Enter adds a line, paste is supported. The footer keeps one muted line of the keys that matter for the focused panel or the open note; a message such as "Saved" takes its place for a moment and then the hints return.

## Dates

`[` and `]` turn the page a day; Home opens today; `g` goes to a `YYYY-MM-DD` date. `y`, or clicking the month, opens the year: arrows move, Enter opens a day, `[` / `]` change the year, PgUp/PgDn page through months on small terminals, `t` selects today, Esc returns. Days that already have a journal carry a small dot after their number.

Each month's pages are printed in their own colour, as a planner's are: the rules, the panel titles, today's date and the month's traditional name on the calendar's title (`2026-09 · 長月` for September) all take that month's shade, twelve soft Japanese colours in all. What you write stays plain, and small print stays grey. In the year, every month wears its own colour like the tabs along a planner's edge.

The header line reads `09-11 (金) · day 254 · New moon`: the month and day as a planner prints them, the weekday, the day of the year, and an approximate moon phase in words. The phase is calculated offline for the date at 12:00 UTC from a mean 29.530588-day cycle anchored to the 2000-01-06 18:14 UTC new moon in [NASA's phase tables](https://eclipse.gsfc.nasa.gov/phase/phases1901.html). It is a daily detail, not an astronomical event time.

The words rotate daily from a small built-in list of public-domain lines. A `words.txt` in the journal directory, one line per entry, replaces it.

## Files and safe saving

The data directory is independent of the current working directory:

1. `--data-dir PATH`
2. `TECHO_DIR`
3. Linux: `$XDG_DATA_HOME/techo/journals`, otherwise `~/.local/share/techo/journals`; Windows: `%LOCALAPPDATA%/techo/journals`.

Each date has one `YYYY-MM-DD.md` file. Browsing a date does not create it; saving does. A directory lock prevents two techo instances editing the same journal directory. Saves write and sync a temporary file before replacing the journal. Save errors keep the editor and draft open. If the file has changed externally, techo refuses to overwrite it; preserve the draft before reopening.

Alpha.1 journals are readable. On their first save, their exact previous contents are retained in `YYYY-MM-DD.md.alpha.bak`. The Markdown format records `techo-format: 2` in frontmatter: multiline todo continuations are indented by two spaces; schedule body lines by four spaces; Free Memo is the final section and permits arbitrary headings and blank lines. Do not edit the same journal concurrently in another application.

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 scripts/terminal_smoke.py target/debug/techo  # Linux/macOS PTY acceptance
```

Windows checks: `./dev.ps1 test` and `./dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')`. `./scripts/dev-window.ps1` rebuilds and reopens techo in a fresh Windows Terminal window, closing the previous one first, for quick rounds of hands-on testing.

[Design review](docs/design-review.md) sets the direction: a quiet page where anything not written by you has to earn its place. The [interaction feedback](docs/interaction-feedback.md) and [implementation notes](docs/interaction-feedback-impl-notes.md) record the alpha.2 round; [design review notes](docs/design-review-impl-notes.md) record this one.
