# techō

techō (手帳, てちょう) is a small terminal journal inspired by the everyday pleasure of a paper planner. Keep an experiment log on a Linux research machine, jot down a thought over SSH, or plan a day locally. Everything stays in daily Markdown files; no account or network connection is needed.

This is `0.1.0-alpha.2`, an early version for hands-on testing.

## Install and run

With a current stable Rust toolchain and a C linker:

```sh
cargo install --path . --locked
techo
```

```sh
techo --date 2027-02-01        # open a specific date
techo --data-dir ./logs        # a journal directory of your choice
techo --keys                   # show what the terminal sends for each key
techo --help
```

techō needs an interactive UTF-8 terminal. The mouse is optional; every operation works from the keyboard.

## A day is one page

On the left: the date, a ruled schedule, and free memo. On the right: todo, the month, and a line of words for the day. `s`, `t`, `f` or Tab move between panels; `n` starts a new item, Enter writes or edits, `d` removes. A note opens on the page where the cursor is: Ctrl+S saves, Esc cancels. `[` and `]` turn the page a day, `y` opens the year.

The first launch shows the key reference; `?` brings it back at any time.

## Files

Each day is one `YYYY-MM-DD.md` file, plain Markdown, in `~/.local/share/techo/journals` (Linux), `%LOCALAPPDATA%/techo/journals` (Windows), or wherever `TECHO_DIR` or `--data-dir` points. Browsing a date does not create a file; saving does, atomically.

## Documentation

- [Guide](docs/guide.md): the page, dates, colours and the small print.
- [Keys and terminals](docs/keys.md): every key, and what to expect on macOS, over SSH and in a multiplexer.
- [Files](docs/files.md): where journals live, the Markdown format, and how saving stays safe.
- [Development](docs/development.md): building, checks, and the design notes.

## License

MIT.
