//! The wall time of each phase of the check, which the hidden `--timings` option prints.

use std::io::Write;
use std::time::{Duration, Instant};

/// The wall time of each phase of the check, measured only when it is enabled.
pub(crate) struct Timings {
    /// When the current phase started, if the timings are measured.
    start: Option<Instant>,
    /// Each phase that ended, with its wall time.
    phases: Vec<(&'static str, Duration)>,
}

impl Timings {
    /// Timings that measure the phases if enabled, and else do nothing.
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            start: enabled.then(Instant::now),
            phases: Vec::new(),
        }
    }

    /// End the current phase, under the given name, and start the next one.
    pub(crate) fn end(&mut self, phase: &'static str) {
        if let Some(start) = &mut self.start {
            let now = Instant::now();
            self.phases.push((phase, now - *start));
            *start = now;
        }
    }

    /// Print the wall time of each phase, and their total, in milliseconds, if they were measured.
    pub(crate) fn print(&self) {
        if self.start.is_none() {
            return;
        }
        let width = self
            .phases
            .iter()
            .map(|(phase, _)| phase.len())
            .max()
            .unwrap_or(0);
        let mut stderr = std::io::stderr().lock();
        for (phase, time) in &self.phases {
            let _ = writeln!(stderr, "{phase:<width$} {:>10.1} ms", milliseconds(*time));
        }
        let total: Duration = self.phases.iter().map(|(_, time)| *time).sum();
        let _ = writeln!(
            stderr,
            "{:<width$} {:>10.1} ms",
            "total",
            milliseconds(total)
        );
    }
}

/// A duration in milliseconds.
fn milliseconds(time: Duration) -> f64 {
    time.as_secs_f64() * 1000.0
}
