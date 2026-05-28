use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone)]
pub struct Watcher {
    pub path: PathBuf,
    pub interval: Duration,
    last_mtime: Option<SystemTime>,
    last_check: Option<Instant>,
    enabled: bool,
}

impl Watcher {
    pub fn new(path: PathBuf) -> Self {
        Self::with_interval(path, Duration::from_millis(500))
    }

    pub fn with_interval(path: PathBuf, interval: Duration) -> Self {
        Self {
            path,
            interval,
            last_mtime: None,
            last_check: None,
            enabled: true,
        }
    }

    pub fn disabled(path: PathBuf) -> Self {
        Self {
            path,
            interval: Duration::from_millis(500),
            last_mtime: None,
            last_check: None,
            enabled: false,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    pub fn poll(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        let now = Instant::now();
        if let Some(last) = self.last_check {
            if now.duration_since(last) < self.interval {
                return false;
            }
        }

        let mtime = match std::fs::metadata(&self.path).and_then(|m| m.modified()) {
            Ok(m) => m,
            Err(_) => return false,
        };

        self.last_check = Some(now);

        match self.last_mtime {
            // First successful poll establishes the baseline; no change to report yet.
            None => {
                self.last_mtime = Some(mtime);
                false
            }
            Some(prev) if prev == mtime => false,
            Some(_) => {
                self.last_mtime = Some(mtime);
                true
            }
        }
    }

    pub fn last_seen(&self) -> Option<SystemTime> {
        self.last_mtime
    }

    pub fn rebaseline(&mut self) {
        self.last_mtime = None;
        self.last_check = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use std::fs;
    use std::io::Write;
    use std::thread;
    use tempfile::tempdir;

    fn write_file(path: &std::path::Path, body: &str) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    fn set_mtime_secs(path: &std::path::Path, secs: i64) {
        set_file_mtime(path, FileTime::from_unix_time(secs, 0)).unwrap();
    }

    #[test]
    fn new_uses_500ms_interval_and_is_enabled() {
        let w = Watcher::new(PathBuf::from("/tmp/x"));
        assert_eq!(w.interval, Duration::from_millis(500));
        assert!(w.is_enabled());
    }

    #[test]
    fn disabled_constructor_yields_disabled() {
        let w = Watcher::disabled(PathBuf::from("/tmp/x"));
        assert!(!w.is_enabled());
    }

    #[test]
    fn poll_first_call_returns_false_and_records_mtime() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(!w.poll());
        assert!(w.last_seen().is_some());
    }

    #[test]
    fn poll_returns_false_when_mtime_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(!w.poll());
        assert!(!w.poll());
    }

    #[test]
    fn poll_returns_true_after_mtime_changes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(!w.poll());

        set_mtime_secs(&path, 1_700_000_500);
        assert!(w.poll());
        assert!(!w.poll());
    }

    #[test]
    fn poll_returns_false_when_disabled_even_if_file_changes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        w.set_enabled(false);
        assert!(!w.poll());

        set_mtime_secs(&path, 1_700_000_900);
        assert!(!w.poll());
        assert!(w.last_seen().is_none());
    }

    #[test]
    fn poll_returns_false_when_path_does_not_exist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nope.md");

        let mut w = Watcher::with_interval(path, Duration::from_millis(0));
        assert!(!w.poll());
        assert!(w.last_seen().is_none());
    }

    #[test]
    fn poll_respects_interval() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_secs(60));
        assert!(!w.poll());

        set_mtime_secs(&path, 1_700_000_500);
        assert!(!w.poll());
    }

    #[test]
    fn rebaseline_clears_last_mtime() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(!w.poll());
        assert!(w.last_seen().is_some());

        w.rebaseline();
        assert!(w.last_seen().is_none());
    }

    #[test]
    fn last_seen_returns_recorded_mtime() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "hello");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(w.last_seen().is_none());
        let _ = w.poll();
        let seen = w.last_seen().expect("mtime recorded after first poll");
        let expected = FileTime::from_unix_time(1_700_000_000, 0);
        let seen_ft = FileTime::from_system_time(seen);
        assert_eq!(seen_ft, expected);
    }

    #[test]
    fn set_enabled_toggles_state() {
        let mut w = Watcher::new(PathBuf::from("/tmp/x"));
        assert!(w.is_enabled());
        w.set_enabled(false);
        assert!(!w.is_enabled());
        w.set_enabled(true);
        assert!(w.is_enabled());
    }

    #[test]
    fn interval_zero_allows_back_to_back_polls() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.md");
        write_file(&path, "x");
        set_mtime_secs(&path, 1_700_000_000);

        let mut w = Watcher::with_interval(path.clone(), Duration::from_millis(0));
        assert!(!w.poll());
        // tiny pause to avoid any platform-level resolution oddities, though Duration::ZERO permits immediate polls
        thread::sleep(Duration::from_millis(1));
        set_mtime_secs(&path, 1_700_000_001);
        assert!(w.poll());
    }
}
