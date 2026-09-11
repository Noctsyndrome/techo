use crate::{calendar::parse_date, journal::Journal};
use chrono::NaiveDate;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct Store {
    pub dir: PathBuf,
    _lock: File,
}

impl Store {
    pub fn open(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(".techo.lock"))?;
        lock.try_lock().map_err(|e| {
            io::Error::other(format!(
                "Journal directory is already in use or cannot be locked: {e}"
            ))
        })?;
        Ok(Self { dir, _lock: lock })
    }
    pub fn path(&self, date: NaiveDate) -> PathBuf {
        self.dir.join(format!("{date}.md"))
    }
    pub fn load(&self, date: NaiveDate) -> io::Result<(Journal, Option<String>)> {
        let raw = read_optional(&self.path(date))?;
        let journal = raw
            .as_deref()
            .map(Journal::from_markdown)
            .transpose()?
            .unwrap_or_default();
        Ok((journal, raw))
    }
    pub fn save(
        &self,
        date: NaiveDate,
        journal: &Journal,
        expected: &Option<String>,
    ) -> io::Result<String> {
        let path = self.path(date);
        if &read_optional(&path)? != expected {
            return Err(io::Error::other(
                "File changed outside techo. Draft retained; copy it before reopening the journal.",
            ));
        }
        // One backup preserves the exact pre-upgrade content, including unsupported legacy formatting.
        if let Some(raw) = expected
            && !raw.lines().any(|l| l == "techo-format: 2")
        {
            let backup = path.with_extension("md.alpha.bak");
            if !backup.exists() {
                atomic_write(&backup, raw)?;
            }
        }
        let raw = journal.to_markdown(date);
        atomic_write(&path, &raw)?;
        Ok(raw)
    }
    /// Dates that already have a journal file: the ink visible from the side of the book.
    pub fn written_dates(&self) -> HashSet<NaiveDate> {
        fs::read_dir(&self.dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| {
                        let name = entry.file_name().into_string().ok()?;
                        parse_date(name.strip_suffix(".md")?)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Optional `words.txt` beside the journals: one line per day, replacing the built-in words.
    pub fn words(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("words.txt"))
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn atomic_write(path: &Path, raw: &str) -> io::Result<()> {
    let tmp = path.with_extension(format!("md.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        file.write_all(raw.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn default_dir() -> io::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("TECHO_DIR").filter(|s| !s.is_empty()) {
        return Ok(dir.into());
    }
    if cfg!(windows) {
        if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(dir).join("techo/journals"));
        }
    } else if let Some(dir) =
        std::env::var_os("XDG_DATA_HOME").filter(|p| Path::new(p).is_absolute())
    {
        return Ok(PathBuf::from(dir).join("techo/journals"));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|p| PathBuf::from(p).join(".local/share/techo/journals"))
        .ok_or_else(|| io::Error::other("Cannot find home directory; pass --data-dir PATH"))
}

#[cfg(test)]
pub fn test_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static ID: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "techo-test-{}-{}",
        std::process::id(),
        ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_are_atomic_and_conflicts_are_not_overwritten() {
        let dir = test_dir();
        let store = Store::open(dir.clone()).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        assert!(Store::open(dir.clone()).is_err());
        let mut j = Journal {
            free_memo: "first".into(),
            ..Journal::default()
        };
        let first = store.save(date, &j, &None).unwrap();
        j.free_memo = "second".into();
        store.save(date, &j, &Some(first.clone())).unwrap();
        assert!(store.save(date, &j, &Some(first)).is_err());
        assert_eq!(store.load(date).unwrap().0.free_memo, "second");
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn legacy_backup_and_read_only_date_browsing() {
        let dir = test_dir();
        let store = Store::open(dir.clone()).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        store.load(date).unwrap();
        assert!(!store.path(date).exists());
        let raw = "## TODO\n\n## Schedule\n\n## Free Memo\nold notes\n";
        fs::write(store.path(date), raw).unwrap();
        let (j, expected) = store.load(date).unwrap();
        assert!(store.written_dates().contains(&date));
        assert!(!store.written_dates().contains(&date.succ_opt().unwrap()));
        assert!(store.words().is_empty());
        fs::write(
            dir.join("words.txt"),
            "
  one 

two
",
        )
        .unwrap();
        assert_eq!(store.words(), ["one", "two"]);
        store.save(date, &j, &expected).unwrap();
        assert_eq!(
            fs::read_to_string(store.path(date).with_extension("md.alpha.bak")).unwrap(),
            raw
        );
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }
}
