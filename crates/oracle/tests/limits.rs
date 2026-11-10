//! The output limit against the oracle's frames, with a counting allocator.
//! The allocator needs `unsafe`, so it lives in this test binary only.

use cleanlz4::{decompress_frame_with, DecodeOptions, Decoder, Error, Status};
use oracle::frame::{compress_frame, Prefs};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counters only observe sizes.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    // A resize counts only its size change, as a system that resizes large
    // blocks in place does.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                let by = new_size - layout.size();
                let now = CURRENT.fetch_add(by, Ordering::SeqCst) + by;
                PEAK.fetch_max(now, Ordering::SeqCst);
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// The two tests share the counters, so they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

const MIB: usize = 1 << 20;

fn bomb() -> Vec<u8> {
    let zeros = vec![0u8; 64 * MIB];
    let prefs = Prefs {
        block_size_id: 7,
        ..Prefs::default()
    };
    let chunks: Vec<&[u8]> = zeros.chunks(MIB).collect();
    compress_frame(&chunks, &prefs)
}

fn limit(n: usize) -> DecodeOptions {
    DecodeOptions {
        max_output_bytes: n as u64,
        ..DecodeOptions::default()
    }
}

#[test]
fn a_small_frame_expanding_past_the_limit_fails() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let f = bomb();
    assert!(f.len() < 2 * MIB, "the bomb is small: {} bytes", f.len());
    assert_eq!(
        decompress_frame_with(&f, &limit(MIB)).map(|o| o.data.len()),
        Err(Error::OutputLimitExceeded)
    );
    let mut d = Decoder::new(limit(MIB));
    let mut out = vec![0u8; 64 * 1024];
    let mut pos = 0;
    let err = loop {
        match d.decode(&f[pos..], &mut out, true) {
            Ok(p) => {
                pos += p.consumed;
                assert_ne!(p.status, Status::Done, "the bomb must not finish");
            }
            Err(e) => break e,
        }
    };
    assert_eq!(err, Error::OutputLimitExceeded);
}

#[test]
fn bomb_peak_allocation_is_bounded() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let f = bomb();
    let bound = MIB + 4 * MIB + 65_536 + f.len() + MIB;
    // One-shot.
    let base = CURRENT.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let r = decompress_frame_with(&f, &limit(MIB)).map(|o| o.data.len());
    assert_eq!(r, Err(Error::OutputLimitExceeded));
    let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
    assert!(peak <= bound, "one-shot peak {peak} over {bound}");
    // Streaming.
    let base = CURRENT.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let mut d = Decoder::new(limit(MIB));
    let mut out = vec![0u8; 64 * 1024];
    let mut pos = 0;
    while let Ok(p) = d.decode(&f[pos..], &mut out, true) {
        pos += p.consumed;
    }
    let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
    assert!(peak <= bound, "streaming peak {peak} over {bound}");
}

fn bomb_64(linked: bool) -> Vec<u8> {
    let zeros = vec![0u8; 4 * MIB];
    let chunks: Vec<&[u8]> = (0..16).map(|_| zeros.as_slice()).collect();
    let prefs = Prefs {
        block_size_id: 7,
        linked,
        ..Prefs::default()
    };
    compress_frame(&chunks, &prefs)
}

#[test]
fn bomb_peak_allocation_is_bounded_past_several_blocks() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let limit_bytes = 3 * 4 * MIB + 1;
    for linked in [true, false] {
        let f = bomb_64(linked);
        let bound = limit_bytes + 4 * MIB + 65_536 + f.len() + MIB;
        // One-shot.
        let base = CURRENT.load(Ordering::SeqCst);
        PEAK.store(base, Ordering::SeqCst);
        let r = decompress_frame_with(&f, &limit(limit_bytes)).map(|o| o.data.len());
        assert_eq!(r, Err(Error::OutputLimitExceeded), "linked {linked}");
        let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
        assert!(
            peak <= bound,
            "linked {linked}: one-shot peak {peak} over {bound}"
        );
        // Streaming.
        let base = CURRENT.load(Ordering::SeqCst);
        PEAK.store(base, Ordering::SeqCst);
        let mut d = Decoder::new(limit(limit_bytes));
        let mut out = vec![0u8; 64 * 1024];
        let mut pos = 0;
        let err = loop {
            match d.decode(&f[pos..], &mut out, true) {
                Ok(p) => pos += p.consumed,
                Err(e) => break e,
            }
        };
        assert_eq!(err, Error::OutputLimitExceeded, "linked {linked}");
        let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
        assert!(
            peak <= bound,
            "linked {linked}: streaming peak {peak} over {bound}"
        );
    }
}
