//! Numbered backups of the open map, written on a timer.
//!
//! Every [`Autosaver::interval`], a modified map with a path on disk is saved to `<map
//! dir>/autosave/<name>.<n>.map`. Once [`Autosaver::max_backups`] is reached, the oldest backups
//! are removed and the survivors are numbered `1..=n`.
//!
//! Autosaves wait until [`Document::in_transaction`] is false. The interval and backup limit are
//! fixed defaults.

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
    thin_backups(&mut backups, max_backups)?;
    clean_backups(&dir, basename, &mut backups)?;

    let path = dir.join(backup_name(basename, backups.len() + 1));
    document.save_copy_to(&path)?;
    Ok(path)
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

/// Deletes the oldest backups until there is room for one more.
///
/// `thinBackups`: with the folder already full, this drops enough from the front that
/// adding the new backup lands back on the limit.
fn thin_backups(backups: &mut Vec<PathBuf>, max_backups: usize) -> std::io::Result<()> {
    if backups.len() < max_backups {
        return Ok(());
    }
    let surplus = backups.len() - max_backups + 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    use std::fs;

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
}
