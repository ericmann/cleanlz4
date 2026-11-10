//! Tuning probe for `limits.max_output_bytes`: decode one bomb (the oracle's
//! linked frame of 2 GiB of zeros, compressed in 4 MiB chunks) at each limit
//! given on the command line, and print the time to the error and the peak
//! allocated bytes. A limit is a byte count or the word `default`.
//!
//! The counting allocator needs `unsafe`, so it lives here, in the example only.

use cleanlz4::{decompress_frame_with, DecodeOptions, DEFAULT_MAX_OUTPUT_BYTES};
use oracle::frame::{compress_frame, Prefs};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grow(by: usize) {
    let now = CURRENT.fetch_add(by, Ordering::SeqCst) + by;
    PEAK.fetch_max(now, Ordering::SeqCst);
}

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counters only observe sizes (a resize counts only its change, as a system
// that resizes large blocks in place does).
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            grow(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                grow(new_size - layout.size());
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

const MIB: usize = 1 << 20;

fn main() {
    let limits: Vec<u64> = std::env::args()
        .skip(1)
        .map(|a| {
            if a == "default" {
                DEFAULT_MAX_OUTPUT_BYTES
            } else {
                a.parse().unwrap_or_else(|_| panic!("bad limit {a}"))
            }
        })
        .collect();
    let chunk = vec![0u8; 4 * MIB];
    let chunks: Vec<&[u8]> = (0..512).map(|_| chunk.as_slice()).collect();
    let prefs = Prefs {
        block_size_id: 7,
        linked: true,
        ..Prefs::default()
    };
    let frame = compress_frame(&chunks, &prefs);
    println!(
        "bomb: 2 GiB of zeros in a {}-byte linked frame",
        frame.len()
    );
    println!("| limit (bytes) | time to the error (ms) | peak allocated (bytes) | peak above the limit (bytes) | result |");
    println!("|---|---|---|---|---|");
    for limit in limits {
        let options = DecodeOptions {
            max_output_bytes: limit,
            ..DecodeOptions::default()
        };
        let base = CURRENT.load(Ordering::SeqCst);
        PEAK.store(base, Ordering::SeqCst);
        let start = Instant::now();
        let result = decompress_frame_with(&frame, &options).map(|o| o.data.len());
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
        println!(
            "| {limit} | {ms:.1} | {peak} | {} | {:?} |",
            peak as i64 - limit as i64,
            result
        );
    }
}
