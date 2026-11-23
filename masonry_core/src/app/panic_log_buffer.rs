// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A tracing buffer that records events during rewrite passes and dumps them
//! to a file only when the process panics.
//!
//! # Usage
//!
//! 1. At subscriber setup time, call [`register_panic_hook`] once and register
//!    a `tracing_subscriber::fmt::layer().with_writer(BufferWriter)`.
//! 2. In [`RenderRoot::run_rewrite_passes`](crate::app::RenderRoot), call
//!    [`start_frame_recording()`] and hold the returned
//!    [`FrameRecordingGuard`] for the duration of the call.
//! 3. If a panic occurs while a guard is live, the panic hook writes the
//!    buffer to the log path chosen during step 1.
//!
//! Only the *most recent* panic before process exit is preserved on disk:
//! each call to the panic hook truncates the existing log. This is fine for
//! the common case (panic crashes the process); callers using
//! `std::panic::catch_unwind` should be aware that earlier panics' buffers
//! will be overwritten.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, TryLockError};

/// Fast-path flag: `true` while a frame recording is active.
///
/// `Relaxed` is sufficient *for correctness of buffer accesses*: the [`Mutex`]
/// in [`EVENT_BUFFER`] provides the happens-before relationship. Both the
/// recording flip in [`FrameRecordingGuard::drop`] and the verifying load in
/// [`BufferWriter::write`] happen *under* the mutex, so once drop returns no
/// subsequent writer can push into the buffer for the just-ended frame.
///
/// The fast-path load in [`BufferWriter::write`] is *not* under the mutex; it
/// is a best-effort optimization to skip the slow path when no frame is
/// recording. On weak memory architectures a cross-thread writer may briefly
/// observe a stale `false` just after [`start_frame_recording`] flips the
/// flag, dropping events emitted in that window. This is acceptable because
/// rewrite passes run on the UI thread and cross-thread tracing during a
/// pass is rare.
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

/// Buffered event lines for the current frame. Bounded by both
/// [`MAX_BUFFER_LINES`] (entry count) and [`MAX_BUFFER_BYTES`] (total length);
/// when either cap is exceeded, the oldest entries are evicted.
static EVENT_BUFFER: OnceLock<Mutex<EventBuffer>> = OnceLock::new();

/// In-memory event buffer with a running byte total so eviction can enforce
/// [`MAX_BUFFER_BYTES`] without re-summing.
struct EventBuffer {
    lines: VecDeque<String>,
    bytes: usize,
}

impl EventBuffer {
    fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            bytes: 0,
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.bytes = 0;
    }

    fn push(&mut self, line: String) {
        self.bytes += line.len();
        self.lines.push_back(line);
        while self.lines.len() > MAX_BUFFER_LINES || self.bytes > MAX_BUFFER_BYTES {
            let Some(removed) = self.lines.pop_front() else {
                // Single line larger than MAX_BUFFER_BYTES: the loop popped
                // everything including the line we just pushed, leaving the
                // buffer empty. `bytes` is now stale, so reset it.
                self.bytes = 0;
                break;
            };
            self.bytes = self.bytes.saturating_sub(removed.len());
        }
    }
}

/// Path to write panic logs to; set once on first call to
/// [`register_panic_hook`].
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Guards one-time installation of the panic hook (and the path/closure
/// allocations that go with it). Subsequent [`register_panic_hook`] calls are
/// pure no-ops.
static HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

/// Maximum number of event lines retained per frame.
const MAX_BUFFER_LINES: usize = 10_000;

/// Maximum total bytes (sum of line lengths) retained per frame. A second
/// guard alongside [`MAX_BUFFER_LINES`] so a misbehaving widget that emits very
/// long log lines cannot balloon RSS within a single frame.
const MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;

// --- MARK: PANIC HOOK

/// Registers a panic hook that flushes buffered rewrite-pass events to a
/// timestamped file in `$TMPDIR` if the process panics.
///
/// Safe to call multiple times (e.g. in tests): only the first call computes
/// the path and installs the hook; subsequent calls are no-ops.
///
/// Pair this with a `tracing_subscriber::fmt::layer().with_writer(BufferWriter)`
/// registered on the global subscriber.
pub(crate) fn register_panic_hook() {
    EVENT_BUFFER.get_or_init(|| Mutex::new(EventBuffer::new()));

    HOOK_INSTALLED.get_or_init(|| {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("masonry-{id:016}-{pid}-panic.log"));
        // Sole writer; inside HOOK_INSTALLED's initializer.
        let _ = LOG_PATH.set(path);

        // Run the previous hook first so the panic message is emitted before
        // our footer, matching the order users expect.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            previous_hook(info);
            #[allow(clippy::print_stderr, reason = "Crash diagnostic pointer")]
            match flush_buffer_to_log() {
                Ok(Some(path)) => {
                    eprintln!("→ Pre-panic trace written to {}", path.display());
                }
                Ok(None) => {}
                Err(err) => {
                    eprintln!("→ Failed to write pre-panic trace: {err}");
                }
            }
        }));
    });
}

/// Flushes the current event buffer to [`LOG_PATH`].
///
/// - `Ok(Some(path))`: a non-empty buffer was written successfully.
/// - `Ok(None)`: nothing to do (no path registered, no buffer, lock would
///   block, or the buffer is empty).
/// - `Err(_)`: the I/O write itself failed.
fn flush_buffer_to_log() -> io::Result<Option<PathBuf>> {
    let Some(path) = LOG_PATH.get() else {
        return Ok(None);
    };
    let Some(lock) = EVENT_BUFFER.get() else {
        return Ok(None);
    };

    // `try_lock` (not `lock`) so the panic hook never deadlocks against
    // itself when a panic originates inside [`BufferWriter::write`] while it
    // holds the buffer mutex.
    let events = match lock.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return Ok(None),
    };

    if events.lines.is_empty() {
        return Ok(None);
    }

    let mut writer = BufWriter::new(File::create(path)?);
    for line in &events.lines {
        writeln!(writer, "{line}")?;
    }
    writer.flush()?;
    Ok(Some(path.clone()))
}

// --- MARK: FRAME GUARD

/// RAII guard returned by [`start_frame_recording`].
///
/// While this guard is live, [`BufferWriter`] will capture tracing events
/// into the in-memory buffer. On drop the recording stops and the buffer is
/// cleared. If a panic occurs before the guard is dropped, the panic hook
/// registered by [`register_panic_hook`] writes the buffer to disk.
///
/// Nesting is not supported: a second `start_frame_recording` call while a
/// guard is live shares the same buffer, and dropping the inner guard clears
/// it out from under the outer.
pub(crate) struct FrameRecordingGuard(());

impl Drop for FrameRecordingGuard {
    fn drop(&mut self) {
        // Take the lock first, then flip the flag *under* the lock. Any
        // writer that re-checks IS_RECORDING under the lock after we drop
        // will see `false` and skip; any writer already past its re-check
        // has already pushed and will be cleared below.
        if let Some(lock) = EVENT_BUFFER.get() {
            let mut events = match lock.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            IS_RECORDING.store(false, Ordering::Relaxed);
            events.clear();
        } else {
            IS_RECORDING.store(false, Ordering::Relaxed);
        }
    }
}

/// Starts buffering tracing events for the duration of one
/// `run_rewrite_passes` invocation.
///
/// In release builds this is a no-op: the per-frame fmt layer is not
/// registered, so there is nothing to record. The returned guard is still
/// held by the caller, which keeps the call site free of `cfg` gates.
///
/// Use `let _guard = start_frame_recording();` — a bare `let _ = ...` would
/// drop the guard immediately.
pub(crate) fn start_frame_recording() -> FrameRecordingGuard {
    #[cfg(debug_assertions)]
    {
        // Clear any leftover entries from a frame that was unwound past via
        // `catch_unwind`, so the next panic dump only contains this frame's
        // events.
        if let Some(lock) = EVENT_BUFFER.get() {
            let mut events = match lock.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            events.clear();
            IS_RECORDING.store(true, Ordering::Relaxed);
        }
    }
    FrameRecordingGuard(())
}

// --- MARK: BUFFER WRITER

/// A [`tracing_subscriber::fmt::MakeWriter`] that directs formatted events
/// into the in-memory buffer when a frame recording is active.
///
/// Use as `.with_writer(BufferWriter)` on a `fmt::layer()`. Writes are
/// no-ops when [`IS_RECORDING`] is `false`. When [`MAX_BUFFER_LINES`] or
/// [`MAX_BUFFER_BYTES`] is exceeded, the oldest entries are evicted.
#[derive(Debug)]
pub(crate) struct BufferWriter;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufferWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        Self
    }
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Fast path: avoid touching the mutex when no frame is recording.
        if !IS_RECORDING.load(Ordering::Relaxed) {
            return Ok(buf.len());
        }
        let Ok(line) = std::str::from_utf8(buf) else {
            return Ok(buf.len());
        };
        let line = line.trim_end();
        if line.is_empty() {
            return Ok(buf.len());
        }
        let Some(lock) = EVENT_BUFFER.get() else {
            return Ok(buf.len());
        };
        let mut events = match lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Re-check under the lock: FrameRecordingGuard::drop flips this flag
        // while holding the same mutex, so a `false` here means recording
        // ended before we acquired the lock.
        if !IS_RECORDING.load(Ordering::Relaxed) {
            return Ok(buf.len());
        }
        events.push(line.to_owned());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
