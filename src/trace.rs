//! Opt-in stage-boundary dumps for cross-checking against the C reference
//! (original/source). Inert unless `BPE_TRACE_DIR` is set: no allocation,
//! no I/O, no behavior change on the normal encode/decode path.

use std::io::Write;

/// Write one `i32` per line to `$BPE_TRACE_DIR/<name>`, in the same
/// row-major iteration order the C side dumps its arrays. No-op if
/// `BPE_TRACE_DIR` isn't set.
pub(crate) fn dump_i32_flat(name: &str, values: impl Iterator<Item = i32>) {
    if let Ok(dir) = std::env::var("BPE_TRACE_DIR") {
        if let Ok(mut f) = std::fs::File::create(format!("{}/{}", dir, name)) {
            for v in values {
                let _ = writeln!(f, "{}", v);
            }
        }
    }
}

/// Truncate (or create) `$BPE_TRACE_DIR/<name>` to empty. Call once before a
/// sequence of `append_f32_flat` calls (e.g. one per decoded segment), to
/// match the C side's per-segment append pattern without stale data from a
/// previous run leaking in.
pub(crate) fn truncate_trace_file(name: &str) {
    if let Ok(dir) = std::env::var("BPE_TRACE_DIR") {
        let _ = std::fs::File::create(format!("{}/{}", dir, name));
    }
}

/// Write one `f32` per line to `$BPE_TRACE_DIR/<name>` (create/overwrite).
/// No-op if `BPE_TRACE_DIR` isn't set.
pub(crate) fn dump_f32_flat(name: &str, values: impl Iterator<Item = f32>) {
    if let Ok(dir) = std::env::var("BPE_TRACE_DIR") {
        if let Ok(mut f) = std::fs::File::create(format!("{}/{}", dir, name)) {
            for v in values {
                let _ = writeln!(f, "{:.9e}", v);
            }
        }
    }
}

/// Append one `f32` per line to `$BPE_TRACE_DIR/<name>`. No-op if
/// `BPE_TRACE_DIR` isn't set.
pub(crate) fn append_f32_flat(name: &str, values: impl Iterator<Item = f32>) {
    if let Ok(dir) = std::env::var("BPE_TRACE_DIR") {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{}/{}", dir, name))
        {
            for v in values {
                let _ = writeln!(f, "{:.9e}", v);
            }
        }
    }
}
