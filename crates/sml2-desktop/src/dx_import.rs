//! Background import/restore boundary, independent of the desktop UI.
use sml2_dx::{
    cache::{self, CacheKey},
    Bundle, ImportError, Presentation,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
};

pub enum Event {
    Progress(&'static str),
    FileChosen(Option<PathBuf>),
    /// Built on the worker thread: parsing every level's tables stays off the UI.
    Finished(Result<(CacheKey, Presentation), String>),
}

pub struct Worker {
    pub events: Receiver<Event>,
    pub cancelled: Arc<AtomicBool>,
}

pub fn cache_root() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "sml2-rs")
        .map(|dirs| dirs.cache_dir().join("dx"))
        .ok_or_else(|| "Cannot find the DX cache directory".into())
}

fn check(cancel: &AtomicBool) -> Result<(), ImportError> {
    if cancel.load(Ordering::Acquire) {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}

/// What the app holds for an installation: its identity and every level's tables.
pub fn presentation(
    result: Result<(CacheKey, Bundle), ImportError>,
) -> Result<(CacheKey, Presentation), String> {
    let (key, bundle) = result.map_err(|e| e.to_string())?;
    Ok((key, Presentation::new(&bundle).map_err(|e| e.to_string())?))
}

/// A cancelled or failed replacement never changes the selected installation.
pub fn import_path(
    root: &Path,
    assets: &Path,
    path: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(&'static str),
) -> Result<(CacheKey, Bundle), ImportError> {
    check(cancel)?;
    progress("Checking base assets…");
    let key = CacheKey::new(sml2_dx::base_fingerprint(assets)?);
    check(cancel)?;
    progress("Reading and verifying DX ROM…");
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    (&mut file)
        .take(sml2_dx::ROM_LEN as u64 + 1)
        .read_to_end(&mut bytes)?;
    check(cancel)?;
    progress("Extracting every level's DX graphics…");
    let bundle = sml2_dx::extract(&bytes)?;
    check(cancel)?;
    let payload = bundle.encode()?;
    progress("Validating and installing DX graphics…");
    cache::publish(
        root,
        &key,
        &payload,
        |bytes| Bundle::decode(bytes).map(|_| ()),
        cancel,
    )?;
    check(cancel)?;
    Ok((key, bundle))
}

pub fn restore(
    root: &Path,
    assets: &Path,
    key: CacheKey,
    cancel: &AtomicBool,
) -> Result<(CacheKey, Bundle), ImportError> {
    check(cancel)?;
    let expected = CacheKey::new(sml2_dx::base_fingerprint(assets)?);
    if key != expected {
        return Err(ImportError::InvalidBundle(
            "the installed DX graphics came from other base assets or an older importer; re-import the ROM"
                .into(),
        ));
    }
    let bytes = cache::load(root, &key, |bytes| Bundle::decode(bytes).map(|_| ()))?;
    check(cancel)?;
    Ok((key, Bundle::decode(&bytes)?))
}

impl Worker {
    /// rfd uses None for both cancellation and backend failures. Let the UI
    /// offer its own browser instead of turning either result into an error.
    pub fn choose_file(picker: impl FnOnce() -> Option<PathBuf> + Send + 'static) -> Self {
        let (sender, events) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        std::thread::spawn(move || {
            let _ = sender.send(Event::FileChosen(picker()));
        });
        Self { events, cancelled }
    }

    pub fn start(
        job: impl FnOnce(&AtomicBool, &dyn Fn(&'static str)) -> Result<(CacheKey, Presentation), String>
            + Send
            + 'static,
    ) -> Self {
        let (sender, events) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel = cancelled.clone();
        std::thread::spawn(move || {
            let progress = |text| {
                let _ = sender.send(Event::Progress(text));
            };
            let result = job(&cancel, &progress);
            let _ = sender.send(Event::Finished(result));
        });
        Self { events, cancelled }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_precedes_filesystem_work() {
        let cancel = AtomicBool::new(true);
        let p = Path::new("/nonexistent/dx-test");
        assert!(matches!(
            import_path(p, p, p, &cancel, |_| {}),
            Err(ImportError::Cancelled)
        ));
        assert!(matches!(
            restore(p, p, CacheKey::new("0".repeat(64)), &cancel),
            Err(ImportError::Cancelled)
        ));
    }
    #[test]
    fn worker_reports_progress_and_failure() {
        let worker = Worker::start(|_, progress| {
            progress("verifying");
            Err("unsupported ROM".into())
        });
        assert!(matches!(
            worker.events.recv().unwrap(),
            Event::Progress("verifying")
        ));
        assert!(
            matches!(worker.events.recv().unwrap(), Event::Finished(Err(e)) if e == "unsupported ROM")
        );
    }
}
