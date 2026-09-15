#!/usr/bin/env python3
"""Linux/macOS PTY acceptance against the real executable; no third-party packages.

Exercises terminal event decoding, mouse capture, paste, saving, reopening,
calendar navigation, and resize recovery with disposable journal files.
"""
import codecs
import fcntl
import os
from pathlib import Path
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unicodedata


class Screen:
    def __init__(self, width=120, height=40):
        self.width, self.height = width, height
        self.cells = [[" "] * width for _ in range(height)]
        self.x = self.y = 0
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def feed(self, raw):
        self.pending += self.decoder.decode(raw)
        while self.pending:
            if self.pending.startswith("\x1b["):
                match = re.match(r"\x1b\[([0-9;?:<>=]*)([@-~])", self.pending)
                if not match:
                    return
                params, code = match.groups()
                self.pending = self.pending[match.end():]
                if params.startswith(("?", "<", ">", "=")):
                    continue
                nums = [int(v or 0) for v in params.split(";")] if ":" not in params else [0]
                n = nums[0] or 1
                if code in "Hf":
                    self.y, self.x = n - 1, (nums[1] or 1) - 1 if len(nums) > 1 else 0
                elif code == "A": self.y = max(0, self.y - n)
                elif code == "B": self.y += n
                elif code == "C": self.x += n
                elif code == "D": self.x = max(0, self.x - n)
                elif code == "G": self.x = n - 1
                elif code == "d": self.y = n - 1
                elif code == "J" and nums[0] in (2, 3):
                    self.cells = [[" "] * self.width for _ in range(self.height)]
                elif code == "K" and 0 <= self.y < self.height:
                    start = 0 if nums[0] in (1, 2) else self.x
                    end = self.x + 1 if nums[0] == 1 else self.width
                    for i in range(max(0, start), min(end, self.width)):
                        self.cells[self.y][i] = " "
                continue
            if self.pending.startswith("\x1b]"):
                end = self.pending.find("\x07")
                if end < 0: return
                self.pending = self.pending[end + 1:]
                continue
            if self.pending == "\x1b": return
            c, self.pending = self.pending[0], self.pending[1:]
            if c == "\r": self.x = 0
            elif c == "\n": self.y += 1
            elif c >= " " and not unicodedata.combining(c):
                width = 2 if unicodedata.east_asian_width(c) in "WF" else 1
                if 0 <= self.y < self.height and 0 <= self.x < self.width:
                    old = self.cells[self.y][self.x]
                    if old and unicodedata.east_asian_width(old) in "WF" and self.x + 1 < self.width:
                        self.cells[self.y][self.x + 1] = " "
                    if not old and self.x > 0:
                        self.cells[self.y][self.x - 1] = " "
                    self.cells[self.y][self.x] = c
                    if width == 2 and self.x + 1 < self.width:
                        self.cells[self.y][self.x + 1] = ""
                self.x += width

    @property
    def text(self):
        return "\n".join("".join(row).rstrip() for row in self.cells)

    def location(self, text):
        for y, row in enumerate(self.cells):
            for x in range(self.width):
                if "".join(row[x:]).startswith(text): return x, y
        raise AssertionError(f"Not visible: {text}\n{self.text}")


class Session:
    def __init__(self, binary, directory, date):
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        self.before = termios.tcgetattr(self.slave)
        self.screen = Screen()
        env = dict(os.environ, TERM="xterm-256color", LANG="C.UTF-8")
        # Deliberately retain NO_COLOR to verify selection without color.
        env["NO_COLOR"] = "1"

        def setup():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        self.process = subprocess.Popen([str(binary), "--data-dir", str(directory), "--date", date],
                                        stdin=self.slave, stdout=self.slave, stderr=self.slave,
                                        env=env, preexec_fn=setup)
        self.pump(0.8)
        # An empty journal directory opens on the key reference; Esc closes it.
        if "techō · keys" in self.screen.text:
            self.send("\x1b")
            assert "techō · keys" not in self.screen.text, self.screen.text
        self.expect("free memo")

    def pump(self, duration=0.2):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select([self.master], [], [], max(0, deadline - time.monotonic()))
            if ready:
                raw = os.read(self.master, 65536)
                # techo asks whether the kitty keyboard protocol is supported
                # (CSI ? u, then a primary device attributes query). Answer the
                # attributes query alone, as a traditional terminal does, so it
                # neither waits two seconds nor switches to the new encoding.
                if b"\x1b[c" in raw:
                    os.write(self.master, b"\x1b[?62;22c")
                self.screen.feed(raw)

    def send(self, value):
        os.write(self.master, value.encode() if isinstance(value, str) else value)
        self.pump()
        assert self.process.poll() is None, self.screen.text

    def paste(self, value): self.send("\x1b[200~" + value + "\x1b[201~")

    def click(self, label):
        x, y = self.screen.location(label)
        self.send(f"\x1b[<0;{x + 1};{y + 1}M\x1b[<0;{x + 1};{y + 1}m")

    def expect(self, text): assert text in self.screen.text, f"Missing {text!r}:\n{self.screen.text}"

    def resize(self, width, height):
        self.screen = Screen(width, height)
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        os.kill(self.process.pid, signal.SIGWINCH)
        self.pump()
        assert self.process.poll() is None, self.screen.text

    def quit(self):
        os.write(self.master, b"q")
        assert self.process.wait(timeout=5) == 0
        restored = termios.tcgetattr(self.slave)
        mask = termios.ICANON | termios.ECHO | termios.ISIG
        assert restored[3] & mask == self.before[3] & mask, "terminal mode was not restored"

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        os.close(self.master)
        os.close(self.slave)


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/techo").resolve()
    snapshots = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else None
    if snapshots: snapshots.mkdir(parents=True, exist_ok=True)

    def snapshot(session, name):
        if snapshots: (snapshots / f"{name}.txt").write_text(session.screen.text, encoding="utf-8")

    with tempfile.TemporaryDirectory(prefix="techo-pty-") as directory:
        root = Path(directory)
        first = root / "2000-01-06.md"
        session = Session(binary, root, "2000-01-06")
        try:
            session.expect("New moon")
            assert not first.exists(), "browsing created a file"
            session.click("schedule")
            session.send("n")
            session.expect("╭ 09:00 ")
            session.expect("Tab time")
            session.paste("实验记录\nsecond line")
            session.send("\t")
            session.send("\x7f" * 5 + "09:30")
            snapshot(session, "schedule-editor")
            session.send(b"\x13")
            session.expect("09:30  实验记录")
            assert "### 09:30\n    实验记录\n    second line" in first.read_text()
            session.click("todo")
            session.send("n")
            session.paste("Read paper\n## TODO\nfollow up")
            session.send(b"\x13")
            session.send(" ")
            assert "- [x] Read paper\n  ## TODO\n  follow up" in first.read_text()
            session.click("free memo")
            session.send("e")
            memo = "日志\n## TODO\n\n" + "\n".join(f"Experiment line {i:02}" for i in range(60))
            session.paste(memo)
            session.expect("Experiment line 59")
            snapshot(session, "memo-editor-scrolled")
            session.send(b"\x13")
            assert first.read_text().endswith(memo)
            snapshot(session, "day")
            session.send("y")
            session.expect("January")
            session.expect("December")
            snapshot(session, "year")
            session.send("g")
            session.send("2024-02-29\r")
            session.expect("02-29 (木)")
            session.expect("2024-02")
            assert not (root / "2024-02-29.md").exists()
            session.send("y")
            session.resize(60, 24)
            session.expect("February")
            snapshot(session, "calendar-small")
            session.click("31")
            session.expect("01-31 (水)")
            session.resize(80, 1)
            session.resize(120, 40)
            # The month calendar, with the year in its title, is back on the wide page.
            session.expect("2024-01")
            session.send("g")
            session.send("2027-02-01\r")
            session.send("fe")
            session.paste("another date")
            session.send(b"\x13")
            assert (root / "2027-02-01.md").read_text().endswith("another date")
            session.send("g")
            session.send("2000-01-06\r")
            session.expect("实验记录")
            session.expect("Read paper")
            session.send("td")
            session.expect("Delete this item?")
            session.send("\x1b")
            assert "Read paper" in first.read_text()
            session.quit()
        finally:
            session.close()

        session = Session(binary, root, "2000-01-06")
        try:
            session.expect("实验记录")
            session.expect("Read paper")
            session.send("fe")
            session.expect("Experiment line 59")
            session.send("\x1b")
            session.quit()
        finally:
            session.close()
    print("PASS: Linux PTY mouse, multiline paste, save/reopen, calendar/leap-day jump, resize, and terminal restoration")


if __name__ == "__main__":
    main()
