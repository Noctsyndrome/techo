# Keys and terminals

`?` or F1 opens the key reference inside techō: every key it answers to, grouped by page, dates, year, note and journal. It scrolls on a short terminal. This page repeats that reference and explains what to expect from different terminals.

## Reference

| page | |
|---|---|
| `s` / `t` / `f` | schedule, todo or free memo |
| Tab / Shift+Tab | next / previous panel |
| Up / Down, `k` / `j` | move; Down past the last item starts a new one |
| PgUp / PgDn, `K` / `J` | move ten |
| Enter | write, or edit the item under the cursor |
| `n` | new item |
| Space | check a todo |
| `d` / Delete | delete the item under the cursor |
| `q` / Esc / Ctrl+C | quit |

| dates | |
|---|---|
| `[` / `]` | previous / next day |
| `T` / Home | today |
| `g` | go to a date, `YYYY-MM-DD` |
| `y` | the year |

| year | |
|---|---|
| arrows | move a day or a week |
| Enter | open the day |
| `[` / `]` | previous / next year |
| `,` / `.`, PgUp / PgDn | page the months |
| `t` / Home | today |
| Esc / `q` | back to the page |

| note | |
|---|---|
| Ctrl+S | save (also ⌘S on a Mac, where the terminal delivers it; see below) |
| Esc | cancel; nothing is kept |
| Enter | new line |
| Tab | schedule: the time; `9`, `930` and `09:30` are read |
| Ctrl+A / Ctrl+E | start / end of the line, as Home / End |
| Ctrl+Home / Ctrl+End | start / end of the note |

## How the keys are chosen

The keys are chosen to arrive through any terminal, over SSH and in a multiplexer: letters, arrows, and Ctrl with a letter. Ctrl+S saves, as in nano and micro. Where a key is missing from a keyboard there is a letter for it: `T` for Home, `,` and `.` for PgUp and PgDn in the year, `K` and `J` on the page, and in the note Ctrl+A and Ctrl+E for Home and End. Nothing is bound to Alt or Option, and Ctrl+Enter is not relied on, since most terminals cannot tell it from Enter.

Keys cannot be customised yet; the reference is the one place they are all listed.

## macOS

Terminals keep the Command key for themselves, so ⌘S cannot reach a terminal program by default; Ctrl+S is the save key there too. At start techō asks the terminal for the kitty keyboard protocol. In kitty, Ghostty, WezTerm and recent iTerm2 that makes ⌘S save as well, and the note's footer then says so. Terminal.app does not speak the protocol and stays on Ctrl+S. Terminal.app also scrolls its own window on Home and End unless they are mapped under Settings › Profiles › Keyboard; Ctrl+A and Ctrl+E need no mapping.

## When a key does not work

`techo --keys` prints what the terminal actually delivers for each key you press, and whether the kitty keyboard protocol is on. That tells whether the terminal or techō is not doing its part. Ctrl+C ends it.
