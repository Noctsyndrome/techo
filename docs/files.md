# Files and safe saving

## Where journals live

The data directory is independent of the current working directory:

1. `--data-dir PATH`
2. `TECHO_DIR`
3. Linux: `$XDG_DATA_HOME/techo/journals`, otherwise `~/.local/share/techo/journals`; Windows: `%LOCALAPPDATA%/techo/journals`.

Each date has one `YYYY-MM-DD.md` file. Browsing a date does not create it; saving does. A `words.txt` in the same directory, one line per entry, replaces the built-in daily words.

## Saving

A directory lock prevents two techō instances editing the same journal directory. Saves write and sync a temporary file before replacing the journal. Save errors keep the editor and draft open. If the file has changed externally, techō refuses to overwrite it; preserve the draft before reopening. Do not edit the same journal concurrently in another application.

## Format

The Markdown format records `techo-format: 2` in frontmatter: multiline todo continuations are indented by two spaces; schedule body lines by four spaces; Free Memo is the final section and permits arbitrary headings and blank lines.

Alpha.1 journals are readable. On their first save, their exact previous contents are retained in `YYYY-MM-DD.md.alpha.bak`.
