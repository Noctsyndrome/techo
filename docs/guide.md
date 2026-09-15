# Guide

## The page

A day is one page. On the left: the date, a ruled schedule with hour marks down its gutter, and free memo. On the right: todo, the month, and a line of words for the day. Panels are ruled boxes; the active one shows its title as a small tab in the month's colour. When a panel holds more than it shows, a small `⋯` sits on its bottom rule, and on its top rule once you have scrolled past the start. The first launch shows the key reference once; `?` brings it back.

**Schedule.** Items in time order, each with its time in the gutter; items at the same time gather under one label. `n` opens a small note on the page with the time as its title: on today's page it starts at the current time, next to an existing item it takes that item's time. Write, then Tab into the title if the time needs changing; `9`, `930` and `09:30` are all read. Enter on an item edits it, `d` removes it, Up/Down move by item. Down past the last item starts a new one, at the time now on today's page; the same works in todo. The paper day runs from 04:00 to 03:59.

**Todo.** `n` adds, Space checks, Enter edits, `d` removes. The note opens on the todo row.

**Free memo.** Enter writes in place. Up/Down or the wheel scrolls.

`s`, `t`, `f` or Tab move between panels; clicking a panel, a line, or a day works as well. Writing never leaves the page: Ctrl+S saves and Esc cancels, Enter adds a line, Ctrl+A and Ctrl+E reach the ends of the line, paste is supported. The footer keeps one muted line of the keys that matter for the focused panel or the open note; a message such as "Saved" takes its place for a moment and then the hints return.

## Dates

`[` and `]` turn the page a day; `T` or Home opens today; `g` goes to a `YYYY-MM-DD` date. `y`, or clicking the month, opens the year: arrows move, Enter opens a day, `[` / `]` change the year, `,` / `.` or PgUp/PgDn page through months on small terminals, `t` selects today, Esc returns. Days that already have a journal carry a small dot after their number.

## The small print

Each month's pages are printed in their own colour, as a planner's are: the rules, the panel titles, today's date and the month's traditional name on the calendar's title (`2026-09 · 長月` for September) all take that month's shade, twelve soft Japanese colours in all. What you write stays plain, and small print stays grey. In the year, every month wears its own colour like the tabs along a planner's edge.

The header line reads `09-11 (金) · day 254 · New moon`: the month and day as a planner prints them, the weekday, the day of the year, and an approximate moon phase in words. The phase is calculated offline for the date at 12:00 UTC from a mean 29.530588-day cycle anchored to the 2000-01-06 18:14 UTC new moon in [NASA's phase tables](https://eclipse.gsfc.nasa.gov/phase/phases1901.html). It is a daily detail, not an astronomical event time.

The words rotate daily from a small built-in list of public-domain lines. A `words.txt` in the journal directory, one line per entry, replaces it.

## Mouse

Mouse input is optional; every operation works with the keyboard. On SSH, mouse support depends on the local terminal forwarding mouse events. `techo --no-mouse` leaves mouse selection to the terminal. When mouse capture is enabled, Shift+drag commonly selects terminal text.
