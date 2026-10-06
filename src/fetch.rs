use crate::git;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

/// Fetches from all remotes in the background: once at startup, then periodically.
pub struct Fetcher {
    done: Receiver<()>,
}

impl Fetcher {
    /// Starts fetching unless disabled by `--no-fetch` or `gud.autoFetch`, or there are no remotes.
    pub fn start(enabled: bool) -> Option<Self> {
        let (config_enabled, interval) = git::auto_fetch_config();
        if !enabled || !config_enabled || !git::has_remotes() {
            return None;
        }
        let (tx, done) = mpsc::channel();
        thread::spawn(move || {
            loop {
                // Failures (offline, credentials needed, ...) are ignored; just try again later.
                if git::fetch_quietly() && tx.send(()).is_err() {
                    return;
                }
                if interval == 0 {
                    return;
                }
                thread::sleep(Duration::from_secs(interval));
            }
        });
        Some(Self { done })
    }

    /// Whether a fetch has succeeded since the last call.
    pub fn fetched(&self) -> bool {
        self.done.try_iter().count() > 0
    }
}
