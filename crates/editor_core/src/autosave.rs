//! Numbered backups of the open map, written on a timer.
//!
//! Every [`Autosaver::interval`], a modified map with a path on disk is saved to `<map
//! dir>/autosave/<name>.<n>.map`. Once [`Autosaver::max_backups`] is reached, the oldest backups
//! are removed and the survivors are numbered `1..=n`.
//!
//! Autosaves wait until [`Document::in_transaction`] is false. The interval and backup limit are
//! fixed defaults.
//!
//! [`recovery`] keeps a second kind of copy, for crashes: in the profile, for untitled maps
//! too, every half minute or so, and offered back when the editor starts again.

use std::path::{Path, PathBuf};
use web_time::{Duration, Instant};

use crate::document::Document;
use crate::i18n::fill;

/// The gap between autosave attempts.
pub const INTERVAL: Duration = Duration::from_secs(10 * 60);

/// The maximum number of backups to retain.
pub const MAX_BACKUPS: usize = 50;

/// The shortest wait [`Autosaver::next_due`] asks for.
pub const MIN_RETRY: Duration = Duration::from_secs(1);

/// Writes numbered backups of a document on a timer.
pub struct Autosaver {
    interval: Duration,
    max_backups: usize,
    last_save: Instant,
    last_modification_count: u64,
}

impl Autosaver {
    /// A saver with the default interval and backup count.
    pub fn new(document: &Document) -> Self {
        Self::with(document, INTERVAL, MAX_BACKUPS)
    }

    /// A saver with an explicit interval and backup count, as the tests use.
    pub fn with(document: &Document, interval: Duration, max_backups: usize) -> Self {
        Self {
            interval,
            max_backups: max_backups.max(1),
            last_save: Instant::now(),
            last_modification_count: document.modification_count(),
        }
    }

    /// How long until [`Autosaver::trigger`] is worth calling again, for a frontend that
    /// sleeps until something happens rather than ticking: `None` while there is nothing
    /// new to back up (the next edit wakes the frontend anyway). Never under
    /// [`MIN_RETRY`], so a backup that cannot happen yet -- a drag held still, a write
    /// that failed -- is retried at that pace rather than in a busy loop.
    pub fn next_due(&self, document: &Document) -> Option<Duration> {
        if !document.is_modified()
            || document.modification_count() == self.last_modification_count
            || !document.path.as_deref().is_some_and(Path::is_file)
        {
            return None;
        }
        Some(
            self.interval
                .saturating_sub(self.last_save.elapsed())
                .max(MIN_RETRY),
        )
    }

    /// Writes a backup if one is due, returning the path written.
    ///
    /// `Autosaver::triggerAutosave`: a backup is due when the map has unsaved changes
    /// that differ from the last backup, the interval has passed since the last one, the
    /// map has a file on disk to sit beside, and no drag is in progress.
    pub fn trigger(&mut self, document: &Document) -> Option<PathBuf> {
        if !document.is_modified()
            || document.modification_count() == self.last_modification_count
            || self.last_save.elapsed() < self.interval
            || document.in_transaction()
        {
            return None;
        }

        let map_path = document.path.as_deref()?;
        if !map_path.is_file() {
            return None;
        }

        match autosave(document, map_path, self.max_backups) {
            Ok(path) => {
                self.last_save = Instant::now();
                self.last_modification_count = document.modification_count();
                Some(path)
            }
            Err(error) => {
                crate::complain!("{}", fill("Autosave failed: {}", &[&error.to_string()]));
                None
            }
        }
    }
}

/// Writes one backup and rotates the folder around it.
fn autosave(document: &Document, map_path: &Path, max_backups: usize) -> std::io::Result<PathBuf> {
    autosave_with(map_path, max_backups, |path| document.save_copy_to(path))
}

fn autosave_with(
    map_path: &Path,
    max_backups: usize,
    write: impl FnOnce(&Path) -> std::io::Result<()>,
) -> std::io::Result<PathBuf> {
    let dir = map_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("autosave");
    crate::files::create_dir_all(&dir)?;

    let basename = map_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("map");

    let mut backups = collect_backups(&dir, basename)?;
    let last = backups
        .last()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .and_then(|name| backup_number(name, basename))
        .unwrap_or(0);
    let next = last
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("backup number overflow"))?;
    let path = dir.join(backup_name(basename, next));
    // Preserve every known-good backup until the replacement is fully written.
    write(&path)?;
    backups.push(path);
    thin_backups(&mut backups, max_backups.max(1))?;
    clean_backups(&dir, basename, &mut backups)?;
    Ok(backups
        .last()
        .expect("at least the new backup remains")
        .clone())
}

/// The name of the `index`th backup: `<basename>.<index>.map`.
fn backup_name(basename: &str, index: usize) -> String {
    format!("{basename}.{index}.map")
}

/// Reads the number out of a backup's file name, or `None` if it is not one.
///
/// `makeBackupPathMatcher`: the name has to be `<basename>.<digits>.map` with the number
/// above zero. `<basename>.1-crash.map` is not a backup -- the part between the dots is
/// not all digits -- so a crash file left in the folder is stepped over.
fn backup_number(name: &str, basename: &str) -> Option<usize> {
    let middle = name
        .strip_prefix(basename)?
        .strip_prefix('.')?
        .strip_suffix(".map")?;
    if middle.is_empty() || !middle.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    middle.parse().ok().filter(|&number| number > 0)
}

/// Lists the folder's backups for this map, oldest first.
fn collect_backups(dir: &Path, basename: &str) -> std::io::Result<Vec<PathBuf>> {
    let mut found: Vec<(usize, PathBuf)> = Vec::new();
    for path in crate::files::read_dir(dir)? {
        if !crate::files::is_file(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(number) = backup_number(name, basename) {
            found.push((number, path));
        }
    }
    found.sort_by_key(|(number, _)| *number);
    Ok(found.into_iter().map(|(_, path)| path).collect())
}

/// Deletes the oldest backups after the new backup is safely on disk.
///
/// The input includes the newly written backup, so cleanup never sacrifices a good
/// copy to make room for a write that could fail.
fn thin_backups(backups: &mut Vec<PathBuf>, max_backups: usize) -> std::io::Result<()> {
    if backups.len() <= max_backups {
        return Ok(());
    }
    let surplus = backups.len() - max_backups;
    for path in backups.drain(..surplus) {
        crate::files::remove_file(&path)?;
    }
    Ok(())
}

/// Renumbers the remaining backups so they run `1..=n` with no gaps.
///
/// `cleanBackups`: the list is sorted ascending and the target number at each position
/// is never larger than the number already there, so renaming front to back never lands
/// on a file still waiting its turn.
fn clean_backups(dir: &Path, basename: &str, backups: &mut [PathBuf]) -> std::io::Result<()> {
    for (index, path) in backups.iter_mut().enumerate() {
        let wanted = dir.join(backup_name(basename, index + 1));
        if *path != wanted {
            crate::files::rename(path, &wanted)?;
            *path = wanted;
        }
    }
    Ok(())
}

/// Crash recovery: a copy of the open map in the profile, kept while it has unsaved
/// changes, so a crash or a killed process loses at most the last interval of work.
///
/// Unlike the numbered backups above, which only maps already saved somewhere get, and
/// which sit beside the map every ten minutes, the recovery copy covers untitled maps
/// too and is written after `interval` once editing pauses for [`QUIET`]. It lives in
/// `<profile>/recovery` as `<name>.map` (the map as a save would write it, with the
/// material paths, no imported models copied) and `<name>.meta` (where the map came from,
/// when the copy was made, and which running editor made it), each written atomically,
/// the map first. The copy goes away when the map is saved, closed or its changes
/// discarded. A copy whose editor is no longer running is offered on the next start.
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use web_time::{Duration, Instant};

    use crate::document::Document;

    /// How long editing has to pause before a copy is written, so a copy is not
    /// written in the middle of a run of edits.
    pub const QUIET: Duration = Duration::from_secs(3);

    /// The default gap between copies.
    pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(30);

    /// The profile folder copies are kept in.
    pub fn dir() -> PathBuf {
        crate::preferences::config_dir().join("recovery")
    }

    /// This running editor: its process id and a token that tells it from a later
    /// process given the same id.
    fn instance() -> &'static (u32, String) {
        static INSTANCE: OnceLock<(u32, String)> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            (pid, format!("{pid}-{nanos:x}"))
        })
    }

    /// Marks this editor as running in `dir`, once per process and folder:
    /// `instance-<pid>` holding the token. Stale marks of editors no longer running are
    /// removed.
    fn register(dir: &Path) {
        static DONE: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
        let Ok(mut done) = DONE.lock() else {
            return;
        };
        if done.iter().any(|d| d == dir) {
            return;
        }
        done.push(dir.to_path_buf());
        {
            let _ = crate::files::create_dir_all(dir);
            let (pid, token) = instance();
            let _ = crate::files::write_atomic(&dir.join(format!("instance-{pid}")), token);
            for path in crate::files::read_dir(dir).unwrap_or_default() {
                let Some(other) = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_prefix("instance-"))
                    .and_then(|n| n.parse::<u32>().ok())
                else {
                    continue;
                };
                if other != *pid && !alive(other) {
                    let _ = crate::files::remove_file(&path);
                }
            }
        }
    }

    /// Unregisters this editor at a clean exit.
    pub fn unregister(dir: &Path) {
        let (pid, _) = instance();
        let _ = crate::files::remove_file(&dir.join(format!("instance-{pid}")));
    }

    /// Whether a process with this id is running.
    fn alive(pid: u32) -> bool {
        if pid == std::process::id() {
            return true;
        }
        #[cfg(target_os = "linux")]
        {
            Path::new("/proc").join(pid.to_string()).exists()
        }
        #[cfg(all(unix, not(target_os = "linux")))]
        {
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        }
        #[cfg(windows)]
        {
            std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                .output()
                .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }

    /// Whether the editor that wrote a copy is still running: its process is, and its
    /// mark still holds the token the copy names.
    fn running(dir: &Path, pid: u32, token: &str) -> bool {
        alive(pid)
            && crate::files::read_to_string(&dir.join(format!("instance-{pid}")))
                .is_ok_and(|held| held.trim() == token)
    }

    fn now_unix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }

    fn modified_unix(path: &Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    /// FNV-1a, for a short stable name from a path.
    fn fnv(text: &str) -> u64 {
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// Keeps the recovery copy of one document.
    pub struct Recovery {
        dir: PathBuf,
        enabled: bool,
        interval: Duration,
        quiet: Duration,
        /// The name of this document's files, without extension.
        stem: String,
        /// The edit count last seen, and when it changed.
        seen: u64,
        edited: Instant,
        /// The edit count and time of the last copy written.
        written: Option<(u64, Instant)>,
        writer: Option<std::thread::JoinHandle<std::io::Result<()>>>,
    }

    impl Recovery {
        /// Copies for `document` in `dir`, on or off, every `interval` at most.
        pub fn new(dir: PathBuf, document: &Document, enabled: bool, interval: Duration) -> Self {
            if enabled {
                register(&dir);
            }
            let stem = Self::stem_for(document);
            Self {
                dir,
                enabled,
                interval: interval.max(Duration::from_secs(1)),
                quiet: QUIET,
                stem,
                seen: document.modification_count(),
                edited: Instant::now(),
                written: None,
                writer: None,
            }
        }

        /// The same with another pause before a copy, as the tests use.
        pub fn with_quiet(mut self, quiet: Duration, interval: Duration) -> Self {
            self.quiet = quiet;
            self.interval = interval;
            self
        }

        /// Waits for a write in progress, as the tests do before looking at the disk.
        pub fn settle(&mut self) {
            self.finish_writer();
        }

        /// The files' name: the map's own name and a hash of its full path, or
        /// "untitled", and this editor's token, so two editors never share files.
        fn stem_for(document: &Document) -> String {
            let (_, token) = instance();
            match document.path.as_deref() {
                Some(path) => {
                    let name: String = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("map")
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                        .take(40)
                        .collect();
                    let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
                    format!("{name}-{:016x}-{token}", fnv(&full.to_string_lossy()))
                }
                None => format!("untitled-{token}"),
            }
        }

        /// Turns copies on or off and sets the interval, as the preferences say.
        pub fn set_options(&mut self, enabled: bool, interval: Duration) {
            if enabled && !self.enabled {
                register(&self.dir);
            }
            if !enabled && self.enabled {
                self.discard();
            }
            self.enabled = enabled;
            self.interval = interval.max(Duration::from_secs(1));
        }

        fn map_file(&self) -> PathBuf {
            self.dir.join(format!("{}.map", self.stem))
        }

        fn meta_file(&self) -> PathBuf {
            self.dir.join(format!("{}.meta", self.stem))
        }

        /// Writes a copy if one is due, returning the copy's path. Call every frame.
        pub fn tick(&mut self, document: &Document) -> Option<PathBuf> {
            let count = document.modification_count();
            if count != self.seen {
                self.seen = count;
                self.edited = Instant::now();
            }
            if !self.enabled {
                return None;
            }
            // Saved under another name: the copy follows.
            let stem = Self::stem_for(document);
            if stem != self.stem {
                self.discard();
                self.stem = stem;
            }
            if !document.is_modified() {
                // Saved, or never changed: nothing to recover.
                if self.written.is_some() {
                    self.discard();
                }
                return None;
            }
            if document.in_transaction()
                || self.written.is_some_and(|(at, _)| at == count)
                || self.edited.elapsed() < self.quiet
                || self
                    .written
                    .is_some_and(|(_, when)| when.elapsed() < self.interval)
            {
                return None;
            }
            // A write still going: try again next frame.
            if self.writer.as_ref().is_some_and(|w| !w.is_finished()) {
                return None;
            }
            self.finish_writer();
            // The text is made here, the disk written on another thread: the map's text
            // takes tens of milliseconds for a large map, the write as long again.
            let map_file = self.map_file();
            let text = document.recovery_text(&map_file);
            let original = document.path.clone();
            let meta = format!(
                "path={}\nsaved={}\npid={}\ninstance={}\nname={}\n",
                original
                    .as_deref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                now_unix(),
                instance().0,
                instance().1,
                document
                    .path
                    .as_deref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
            );
            let (dir, meta_file) = (self.dir.clone(), self.meta_file());
            let copy = map_file.clone();
            self.writer = Some(std::thread::spawn(move || {
                crate::files::create_dir_all(&dir)?;
                crate::files::write_atomic(&copy, text)?;
                crate::files::write_atomic(&meta_file, meta)
            }));
            self.written = Some((count, Instant::now()));
            Some(map_file)
        }

        /// Waits for the last write and reports it if it failed.
        fn finish_writer(&mut self) {
            if let Some(writer) = self.writer.take() {
                match writer.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        // Written again at the next chance.
                        self.written = None;
                        crate::complain!(
                            "{}",
                            crate::i18n::fill("Recovery copy failed: {}", &[&error.to_string()])
                        );
                    }
                    Err(_) => self.written = None,
                }
            }
        }

        /// How long until [`Recovery::tick`] is worth calling, for a frontend that
        /// sleeps: `None` while there is nothing to copy.
        pub fn next_due(&self, document: &Document) -> Option<Duration> {
            if !self.enabled
                || !document.is_modified()
                || self
                    .written
                    .is_some_and(|(at, _)| at == document.modification_count())
            {
                return None;
            }
            let quiet = self.quiet.saturating_sub(self.edited.elapsed());
            let gap = self.written.map_or(Duration::ZERO, |(_, when)| {
                self.interval.saturating_sub(when.elapsed())
            });
            Some(quiet.max(gap).max(super::MIN_RETRY))
        }

        /// Removes this document's copy: it was saved, closed or its changes discarded.
        pub fn discard(&mut self) {
            if let Some(writer) = self.writer.take() {
                let _ = writer.join();
            }
            let _ = crate::files::remove_file(&self.meta_file());
            let _ = crate::files::remove_file(&self.map_file());
            self.written = None;
        }

        /// Takes over a restored copy's files, so the next copy replaces them.
        pub fn adopt(&mut self, orphan: &Orphan) {
            if orphan.stem != self.stem {
                let _ = crate::files::remove_file(&self.meta_file());
                let _ = crate::files::remove_file(&self.map_file());
                self.stem = orphan.stem.clone();
            }
            // Written again as soon as editing pauses.
            self.written = None;
        }
    }

    /// A copy left by an editor that is no longer running.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Orphan {
        dir: PathBuf,
        stem: String,
        /// The map it is a copy of, `None` for an untitled map.
        pub path: Option<PathBuf>,
        /// When the copy was made, in seconds since 1970.
        pub saved: u64,
        /// Whether the map's file changed on disk after the copy was made: restoring
        /// and saving would overwrite that.
        pub file_changed: bool,
    }

    impl Orphan {
        /// The copy's map file.
        pub fn map_file(&self) -> PathBuf {
            self.dir.join(format!("{}.map", self.stem))
        }

        /// How long ago the copy was made, in whole minutes.
        pub fn minutes_ago(&self) -> u64 {
            now_unix().saturating_sub(self.saved) / 60
        }

        /// The copy as a document: where it came from, marked as having unsaved changes.
        pub fn restore(&self, bounds: crate::world::WorldBounds) -> Result<Document, String> {
            let text = crate::files::read_to_string(&self.map_file()).map_err(|e| e.to_string())?;
            Document::from_recovery(&text, &self.map_file(), self.path.clone(), bounds)
                .map_err(|e| e.to_string())
        }

        /// Deletes the copy.
        pub fn discard(&self) {
            let _ = crate::files::remove_file(&self.dir.join(format!("{}.meta", self.stem)));
            let _ = crate::files::remove_file(&self.map_file());
        }
    }

    /// The copies in `dir` whose editor is no longer running, newest first. A copy
    /// another running editor is keeping is left to it.
    pub fn orphans(dir: &Path) -> Vec<Orphan> {
        let mut found = Vec::new();
        for meta in crate::files::read_dir(dir).unwrap_or_default() {
            if meta.extension().and_then(|e| e.to_str()) != Some("meta") {
                continue;
            }
            let Some(stem) = meta.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else {
                continue;
            };
            let Ok(text) = crate::files::read_to_string(&meta) else {
                continue;
            };
            let field = |key: &str| {
                text.lines()
                    .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
                    .unwrap_or("")
                    .to_string()
            };
            let pid: u32 = field("pid").parse().unwrap_or(0);
            if running(dir, pid, &field("instance")) {
                continue;
            }
            if !crate::files::is_file(&dir.join(format!("{stem}.map"))) {
                continue;
            }
            let saved = field("saved").parse().unwrap_or(0);
            let path = Some(field("path"))
                .filter(|p| !p.is_empty())
                .map(PathBuf::from);
            let file_changed = path
                .as_deref()
                .and_then(modified_unix)
                .is_some_and(|modified| modified > saved);
            found.push(Orphan {
                dir: dir.to_path_buf(),
                stem,
                path,
                saved,
                file_changed,
            });
        }
        found.sort_by_key(|orphan| std::cmp::Reverse(orphan.saved));
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    use std::fs;

    #[test]
    fn next_audit_failed_backup_write_preserves_every_existing_backup() {
        let dir = scratch("failure-preserves");
        let backups = dir.join("autosave");
        fs::create_dir_all(&backups).unwrap();
        fs::write(backups.join("test.1.map"), b"one").unwrap();
        fs::write(backups.join("test.3.map"), b"three").unwrap();
        let result = autosave_with(&dir.join("test.map"), 1, |path| {
            assert_eq!(path, backups.join("test.4.map"));
            assert_eq!(fs::read(backups.join("test.1.map")).unwrap(), b"one");
            assert_eq!(fs::read(backups.join("test.3.map")).unwrap(), b"three");
            Err(std::io::Error::other("simulated write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 2);
        assert_eq!(fs::read(backups.join("test.1.map")).unwrap(), b"one");
        assert_eq!(fs::read(backups.join("test.3.map")).unwrap(), b"three");
    }

    #[test]
    fn next_audit_single_backup_is_replaced_only_after_successful_write() {
        let dir = scratch("single-rotation");
        let (mut document, mut saver) = saved_document(&dir, 1);
        add_cube(&mut document, -16.0, 16.0);
        let first = saver.trigger(&document).unwrap();
        let original = fs::read(&first).unwrap();
        add_cube(&mut document, 0.0, 32.0);
        let second = saver.trigger(&document).unwrap();
        assert_eq!(first, second);
        assert_ne!(fs::read(&second).unwrap(), original);
        assert_eq!(fs::read_dir(dir.join("autosave")).unwrap().count(), 1);
        let restored = Document::open(second, WorldBounds::default()).unwrap();
        assert_eq!(restored.brush_count(), 2);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("heron-autosave-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch dir");
        dir
    }

    /// Adds one cube spanning `low..=high` on every axis and selects it.
    fn add_cube(document: &mut Document, low: f64, high: f64) {
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::new(low, low, low), Vec3d::new(high, high, high)),
            "material",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }

    /// A clean map saved to `<dir>/test.map`, with an autosaver tracking its current revision.
    fn saved_document(dir: &Path, max_backups: usize) -> (Document, Autosaver) {
        let mut document = Document::new(WorldBounds::default());
        document
            .save_as(dir.join("test.map"))
            .expect("the map saves");
        let saver = Autosaver::with(&document, Duration::ZERO, max_backups);
        (document, saver)
    }

    mod recovery_copies {
        use super::super::recovery::*;
        use super::*;

        /// A keeper for `document` in `dir` with no pause and no interval, so each tick
        /// with something new writes.
        fn keeper(dir: &Path, document: &Document) -> Recovery {
            Recovery::new(dir.to_path_buf(), document, true, DEFAULT_INTERVAL)
                .with_quiet(Duration::ZERO, Duration::ZERO)
        }

        fn copies(dir: &Path) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| !n.starts_with("instance-"))
                .collect();
            names.sort();
            names
        }

        /// A copy as an editor that crashed would have left it: its process long gone.
        fn crashed(dir: &Path, name: &str, path: Option<&Path>, text: &str, saved: u64) {
            fs::write(dir.join(format!("{name}.map")), text).unwrap();
            fs::write(
                dir.join(format!("{name}.meta")),
                format!(
                    "path={}\nsaved={saved}\npid=4000000000\ninstance=4000000000-1\nname=\n",
                    path.map(|p| p.display().to_string()).unwrap_or_default()
                ),
            )
            .unwrap();
        }

        #[test]
        fn a_changed_map_gets_a_copy_that_saving_removes_and_that_restores_whole() {
            let dir = scratch("recovery-cycle");
            let profile = dir.join("recovery");
            let (mut document, _) = saved_document(&dir, 5);
            let mut recovery = keeper(&profile, &document);
            assert!(recovery.tick(&document).is_none(), "nothing changed yet");
            add_cube(&mut document, -16.0, 16.0);
            let copy = recovery.tick(&document).expect("a copy");
            recovery.settle();
            assert_eq!(copies(&profile).len(), 2, "the map and what it is");
            assert!(recovery.tick(&document).is_none(), "nothing new since");
            // The copy is the map, with where it came from.
            let restored = Document::from_recovery(
                &fs::read_to_string(&copy).unwrap(),
                &copy,
                document.path.clone(),
                WorldBounds::default(),
            )
            .unwrap();
            assert_eq!(restored.to_map_text(), document.to_map_text());
            assert_eq!(restored.path, document.path);
            assert!(restored.is_modified(), "restored work is unsaved");
            // Not while a drag is under way.
            add_cube(&mut document, 32.0, 48.0);
            document.begin_transaction();
            assert!(recovery.tick(&document).is_none());
            document.commit_transaction();
            assert!(recovery.tick(&document).is_some());
            recovery.settle();
            // Saved: nothing to recover any more.
            document.save().unwrap();
            recovery.tick(&document);
            assert!(copies(&profile).is_empty(), "{:?}", copies(&profile));
        }

        #[test]
        fn an_untitled_map_is_copied_too_and_restores_untitled() {
            let dir = scratch("recovery-untitled");
            let mut document = Document::new(WorldBounds::default());
            let mut recovery = keeper(&dir, &document);
            add_cube(&mut document, 0.0, 8.0);
            let copy = recovery.tick(&document).expect("a copy");
            recovery.settle();
            assert!(copy
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("untitled-"));
            let restored = Document::from_recovery(
                &fs::read_to_string(&copy).unwrap(),
                &copy,
                None,
                WorldBounds::default(),
            )
            .unwrap();
            assert_eq!(restored.path, None);
            assert_eq!(map_model::node::brushes(restored.tree_world()).len(), 1);
            // Closing with the changes let go removes it.
            recovery.discard();
            assert!(copies(&dir).is_empty());
        }

        #[test]
        fn copies_are_offered_only_when_their_editor_is_gone() {
            let dir = scratch("recovery-orphans");
            let map = dir.join("hall.map");
            fs::write(&map, "").unwrap();
            let mut document = Document::new(WorldBounds::default());
            add_cube(&mut document, 0.0, 8.0);
            let text = document.to_map_text();
            // An old copy of hall.map, saved again on disk since; and an untitled one.
            crashed(&dir, "hall-1", Some(&map), &text, 1);
            crashed(&dir, "untitled-2", None, &text, 2);
            // This editor's own copy is not an orphan.
            let mut recovery = keeper(&dir, &document);
            recovery.tick(&document).unwrap();
            recovery.settle();
            let found = orphans(&dir);
            assert_eq!(found.len(), 2, "{found:?}");
            assert_eq!(found[0].path, None, "newest first");
            assert_eq!(found[1].path.as_deref(), Some(map.as_path()));
            assert!(found[1].file_changed, "hall.map was written after the copy");
            assert!(!found[0].file_changed);
            let restored = found[1].restore(WorldBounds::default()).unwrap();
            assert_eq!(restored.path.as_deref(), Some(map.as_path()));
            assert_eq!(map_model::node::brushes(restored.tree_world()).len(), 1);
            // Taken over by the editor that restored it: its next copy replaces it.
            recovery.adopt(&found[1]);
            add_cube(&mut document, 16.0, 24.0);
            recovery.tick(&document).unwrap();
            recovery.settle();
            assert_eq!(
                orphans(&dir).len(),
                1,
                "only the untitled one left to offer"
            );
            found[0].discard();
            assert!(orphans(&dir).is_empty());
        }

        /// A second editor running at the same time keeps its own copies: they are not
        /// offered to this one. The parent process stands in for it.
        #[cfg(unix)]
        #[test]
        fn another_running_editors_copies_are_left_to_it() {
            let dir = scratch("recovery-other-editor");
            let other = std::os::unix::process::parent_id();
            fs::write(dir.join(format!("instance-{other}")), "other-token").unwrap();
            fs::write(dir.join("its.map"), "").unwrap();
            fs::write(
                dir.join("its.meta"),
                format!("path=\nsaved=5\npid={other}\ninstance=other-token\nname=\n"),
            )
            .unwrap();
            assert!(orphans(&dir).is_empty());
            // Once it has stopped (its mark names another run), the copy is offered.
            fs::write(dir.join(format!("instance-{other}")), "a-later-run").unwrap();
            assert_eq!(orphans(&dir).len(), 1);
        }

        #[test]
        fn turned_off_there_are_no_copies() {
            let dir = scratch("recovery-off");
            let mut document = Document::new(WorldBounds::default());
            let mut recovery = keeper(&dir, &document);
            add_cube(&mut document, 0.0, 8.0);
            recovery.tick(&document).unwrap();
            recovery.settle();
            assert_eq!(copies(&dir).len(), 2);
            recovery.set_options(false, DEFAULT_INTERVAL);
            assert!(copies(&dir).is_empty(), "turning it off removes the copy");
            add_cube(&mut document, 16.0, 24.0);
            assert!(recovery.tick(&document).is_none());
            assert!(recovery.next_due(&document).is_none());
        }

        #[test]
        fn a_copy_waits_for_editing_to_pause_and_for_the_interval() {
            let dir = scratch("recovery-timing");
            let mut document = Document::new(WorldBounds::default());
            let mut recovery = Recovery::new(dir.clone(), &document, true, DEFAULT_INTERVAL)
                .with_quiet(Duration::from_secs(60), Duration::ZERO);
            add_cube(&mut document, 0.0, 8.0);
            assert!(recovery.tick(&document).is_none(), "editing has not paused");
            let due = recovery.next_due(&document).unwrap();
            assert!(due > Duration::from_secs(50), "{due:?}");
            let mut recovery = Recovery::new(dir.clone(), &document, true, DEFAULT_INTERVAL)
                .with_quiet(Duration::ZERO, Duration::from_secs(60));
            add_cube(&mut document, 16.0, 24.0);
            assert!(recovery.tick(&document).is_some());
            recovery.settle();
            add_cube(&mut document, 32.0, 40.0);
            assert!(
                recovery.tick(&document).is_none(),
                "the interval has not passed"
            );
        }
    }

    #[test]
    fn backup_number_reads_only_real_backups() {
        assert_eq!(backup_number("test.1.map", "test"), Some(1));
        assert_eq!(backup_number("test.20.map", "test"), Some(20));
        assert_eq!(backup_number("test.map", "test"), None);
        assert_eq!(backup_number("test.0.map", "test"), None);
        assert_eq!(backup_number("test.1-crash.map", "test"), None);
        assert_eq!(backup_number("other.1.map", "test"), None);
        assert_eq!(backup_number("test.1.txt", "test"), None);
    }

    #[test]
    fn nothing_is_written_before_the_interval_passes() {
        let dir = scratch("interval");
        let mut document = Document::new(WorldBounds::default());
        document.save_as(dir.join("test.map")).expect("it saves");
        let mut saver = Autosaver::with(&document, Duration::from_secs(600), MAX_BACKUPS);

        add_cube(&mut document, -16.0, 16.0);
        assert_eq!(saver.trigger(&document), None);
        assert!(!dir.join("autosave").exists());
    }

    #[test]
    fn the_next_backup_is_due_only_once_there_is_something_new() {
        let dir = scratch("due");
        let mut document = Document::new(WorldBounds::default());
        document.save_as(dir.join("test.map")).expect("it saves");
        let saver = Autosaver::with(&document, Duration::from_secs(600), MAX_BACKUPS);
        assert_eq!(
            saver.next_due(&document),
            None,
            "nothing to back up: no wake-up"
        );
        add_cube(&mut document, -16.0, 16.0);
        let due = saver
            .next_due(&document)
            .expect("an edit makes a backup due");
        assert!(
            due > Duration::from_secs(590) && due <= Duration::from_secs(600),
            "{due:?}"
        );

        let (mut document, mut saver) = saved_document(&scratch("due-now"), MAX_BACKUPS);
        add_cube(&mut document, -16.0, 16.0);
        assert_eq!(
            saver.next_due(&document),
            Some(MIN_RETRY),
            "an overdue backup is retried, never in a busy loop"
        );
        saver.trigger(&document).expect("a backup");
        assert_eq!(
            saver.next_due(&document),
            None,
            "backed up: nothing until the next edit"
        );

        let mut unsaved = Document::new(WorldBounds::default());
        let saver = Autosaver::with(&unsaved, Duration::ZERO, MAX_BACKUPS);
        add_cube(&mut unsaved, -16.0, 16.0);
        assert_eq!(
            saver.next_due(&unsaved),
            None,
            "a map with no file is never backed up"
        );
    }

    #[test]
    fn an_unchanged_map_is_not_backed_up() {
        let dir = scratch("unchanged");
        let (document, mut saver) = saved_document(&dir, MAX_BACKUPS);

        assert_eq!(saver.trigger(&document), None);
        assert!(!dir.join("autosave").exists());
    }

    #[test]
    fn a_backup_is_written_once_the_interval_has_passed() {
        let dir = scratch("write");
        let (mut document, mut saver) = saved_document(&dir, MAX_BACKUPS);

        add_cube(&mut document, -16.0, 16.0);
        let written = saver.trigger(&document).expect("a backup");
        assert_eq!(written, dir.join("autosave/test.1.map"));
        assert!(written.is_file());
    }

    #[test]
    fn a_second_backup_waits_for_another_change() {
        let dir = scratch("second");
        let (mut document, mut saver) = saved_document(&dir, MAX_BACKUPS);

        add_cube(&mut document, -16.0, 16.0);
        saver.trigger(&document).expect("the first backup");
        assert_eq!(saver.trigger(&document), None);
        assert!(!dir.join("autosave/test.2.map").exists());

        add_cube(&mut document, 0.0, 32.0);
        assert_eq!(
            saver.trigger(&document),
            Some(dir.join("autosave/test.2.map"))
        );
    }

    #[test]
    fn no_backup_while_a_drag_is_in_progress() {
        let dir = scratch("transaction");
        let (mut document, mut saver) = saved_document(&dir, MAX_BACKUPS);

        add_cube(&mut document, -16.0, 16.0);
        document.begin_transaction();
        assert_eq!(saver.trigger(&document), None);
        document.cancel_transaction();
    }

    #[test]
    fn gaps_are_compacted_and_the_new_backup_goes_last() {
        let dir = scratch("gaps");
        let autosave = dir.join("autosave");
        fs::create_dir_all(&autosave).unwrap();
        fs::write(autosave.join("test.1.map"), "one").unwrap();
        fs::write(autosave.join("test.3.map"), "three").unwrap();

        let (mut document, mut saver) = saved_document(&dir, 3);
        add_cube(&mut document, -16.0, 16.0);
        saver.trigger(&document).expect("a backup");

        assert!(autosave.join("test.1.map").is_file());
        assert!(autosave.join("test.2.map").is_file());
        assert!(autosave.join("test.3.map").is_file());
        assert_eq!(
            fs::read_to_string(autosave.join("test.1.map")).unwrap(),
            "one"
        );
        assert_eq!(
            fs::read_to_string(autosave.join("test.2.map")).unwrap(),
            "three"
        );
    }

    #[test]
    fn the_oldest_backups_are_deleted_once_the_folder_is_full() {
        let dir = scratch("thin");
        let autosave = dir.join("autosave");
        fs::create_dir_all(&autosave).unwrap();
        for number in 1..=5 {
            fs::write(
                autosave.join(format!("test.{number}.map")),
                number.to_string(),
            )
            .unwrap();
        }

        let (mut document, mut saver) = saved_document(&dir, 3);
        add_cube(&mut document, -16.0, 16.0);
        saver.trigger(&document).expect("a backup");

        // test.1, test.2, test.3 gone; test.4 and test.5 renumbered to 1 and 2; the new
        // one is test.3.
        let names: Vec<_> = fs::read_dir(&autosave)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names.len(), 3);
        assert_eq!(
            fs::read_to_string(autosave.join("test.1.map")).unwrap(),
            "4"
        );
        assert_eq!(
            fs::read_to_string(autosave.join("test.2.map")).unwrap(),
            "5"
        );
        assert!(autosave.join("test.3.map").is_file());
    }

    #[test]
    fn a_crash_file_in_the_folder_is_left_alone() {
        let dir = scratch("crash");
        let autosave = dir.join("autosave");
        fs::create_dir_all(&autosave).unwrap();
        fs::write(autosave.join("test.1.map"), "one").unwrap();
        fs::write(autosave.join("test.1-crash.map"), "crash").unwrap();

        let (mut document, mut saver) = saved_document(&dir, MAX_BACKUPS);
        add_cube(&mut document, -16.0, 16.0);
        saver.trigger(&document).expect("a backup");

        assert_eq!(
            fs::read_to_string(autosave.join("test.1-crash.map")).unwrap(),
            "crash"
        );
        assert!(autosave.join("test.2.map").is_file());
    }
    #[test]
    fn provisional_text_is_not_autosaved_and_confirmed_text_is() {
        let dir = scratch("text-preview");
        let (mut document, mut saver) = saved_document(&dir, MAX_BACKUPS);
        add_cube(&mut document, -16., 16.);
        let before = document.to_map_text();
        assert!(document.begin_text_preview());
        document.update_text_preview(
            "O8",
            64.,
            16.,
            Vec3d::new(128., 0., 0.),
            &crate::grid::Grid::default(),
        );
        assert!(saver.trigger(&document).is_none());
        assert!(!dir.join("autosave").exists());
        document.cancel_text_preview();
        let backup = saver
            .trigger(&document)
            .expect("original pending edit can be backed up");
        assert_eq!(
            Document::open(backup, WorldBounds::default())
                .unwrap()
                .to_map_text(),
            before
        );
        document.begin_text_preview();
        document.update_text_preview(
            "O8",
            64.,
            16.,
            Vec3d::new(128., 0., 0.),
            &crate::grid::Grid::default(),
        );
        assert!(saver.trigger(&document).is_none());
        document.finish_text_preview("O8");
        let backup = saver
            .trigger(&document)
            .expect("confirmed text is backed up");
        assert_eq!(
            Document::open(backup, WorldBounds::default())
                .unwrap()
                .to_map_text(),
            document.to_map_text()
        );
    }
}
