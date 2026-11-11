//! One scratch buffer per frame: the fresh allocations made while decoding do
//! not grow with the number of blocks. The counting allocator needs `unsafe`,
//! so it lives in this test binary only.

use cleanlz4::{decompress_frame, DecodeOptions, Decoder, Status};
use oracle::block::compress_default;
use oracle::frame::{compress_frame, Prefs};
use oracle::synth::word_salad;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

struct Counting;

static FRESH: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counter only observes fresh allocations.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        FRESH.fetch_add(1, Ordering::SeqCst);
        LIVE.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(new_size, Ordering::SeqCst);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// The tests share the counter, so they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// How many fresh allocations the closure makes.
fn fresh<T>(f: impl FnOnce() -> T) -> (usize, T) {
    let before = FRESH.load(Ordering::SeqCst);
    let r = f();
    (FRESH.load(Ordering::SeqCst) - before, r)
}

/// Slack allowed between a 16-block and a 64-block frame; a `Vec` per block
/// would add 48 or more.
const SLACK: usize = 4;

fn regular_frame(blocks: usize, linked: bool) -> (Vec<u8>, Vec<u8>) {
    let data = word_salad(7, blocks * 65_536);
    let chunks: Vec<&[u8]> = data.chunks(65_536).collect();
    let prefs = Prefs {
        block_size_id: 4,
        auto_flush: true,
        linked,
        ..Prefs::default()
    };
    (compress_frame(&chunks, &prefs), data)
}

fn legacy_frame(blocks: usize) -> (Vec<u8>, Vec<u8>) {
    let data = word_salad(9, blocks * 4096);
    let mut f = 0x184C_2102u32.to_le_bytes().to_vec();
    for chunk in data.chunks(4096) {
        let block = compress_default(chunk);
        f.extend_from_slice(&(block.len() as u32).to_le_bytes());
        f.extend_from_slice(&block);
    }
    (f, data)
}

fn one_shot(frame: &[u8], data: &[u8]) -> usize {
    let (n, out) = fresh(|| decompress_frame(frame).expect("decodes"));
    assert_eq!(out, data);
    n
}

fn streaming(frame: &[u8], data: &[u8]) -> usize {
    let mut out = Vec::with_capacity(data.len());
    let mut buf = vec![0u8; 65_536];
    let (n, ()) = fresh(|| {
        let mut d = Decoder::new(DecodeOptions::default());
        let mut pos = 0;
        loop {
            let p = d
                .decode(frame.get(pos..).unwrap_or(&[]), &mut buf, true)
                .expect("decodes");
            pos += p.consumed;
            out.extend_from_slice(buf.get(..p.produced).unwrap_or(&[]));
            if p.status == Status::Done {
                break;
            }
        }
    });
    assert_eq!(out, data);
    n
}

fn check(label: &str, few: usize, many: usize) {
    eprintln!("{label}: 16 blocks {few} fresh allocations, 64 blocks {many}");
    assert!(
        many <= few + SLACK,
        "{label}: 64 blocks made {many}, 16 blocks {few}"
    );
}

#[test]
fn one_shot_fresh_allocations_do_not_grow_with_the_block_count() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for linked in [false, true] {
        let (f16, d16) = regular_frame(16, linked);
        let (f64, d64) = regular_frame(64, linked);
        let few = one_shot(&f16, &d16);
        let many = one_shot(&f64, &d64);
        check(&format!("one-shot linked={linked}"), few, many);
    }
}

#[test]
fn streaming_fresh_allocations_do_not_grow_with_the_block_count() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for linked in [false, true] {
        let (f16, d16) = regular_frame(16, linked);
        let (f64, d64) = regular_frame(64, linked);
        let few = streaming(&f16, &d16);
        let many = streaming(&f64, &d64);
        check(&format!("streaming linked={linked}"), few, many);
    }
}

#[test]
fn legacy_fresh_allocations_do_not_grow_with_the_block_count() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (f16, d16) = legacy_frame(16);
    let (f64, d64) = legacy_frame(64);
    check(
        "legacy one-shot",
        one_shot(&f16, &d16),
        one_shot(&f64, &d64),
    );
    check(
        "legacy streaming",
        streaming(&f16, &d16),
        streaming(&f64, &d64),
    );
}

/// One block of a large legacy frame, big enough to dominate the readings.
const BIG_BLOCK: usize = 1 << 20;

/// A legacy frame of three big blocks, with no end mark.
fn big_legacy_frame() -> (Vec<u8>, Vec<u8>) {
    let data = word_salad(11, 3 * BIG_BLOCK);
    let mut f = 0x184C_2102u32.to_le_bytes().to_vec();
    for chunk in data.chunks(BIG_BLOCK) {
        let block = compress_default(chunk);
        f.extend_from_slice(&(block.len() as u32).to_le_bytes());
        f.extend_from_slice(&block);
    }
    (f, data)
}

/// Streams `input` and returns the live bytes after the last call that
/// delivered output before the first `FrameDone`, the live bytes when
/// `FrameDone` is reported, and the decoded bytes.
fn live_around_frame_end(input: &[u8], capacity: usize) -> (usize, usize, Vec<u8>) {
    // Allocated before counting starts mattering, so it adds a constant.
    let mut out = Vec::with_capacity(capacity);
    let mut buf = vec![0u8; BIG_BLOCK];
    let mut d = Decoder::new(DecodeOptions::default());
    let mut pos = 0;
    let mut last_delivery = 0;
    let mut at_done = None;
    loop {
        let p = d
            .decode(input.get(pos..).unwrap_or(&[]), &mut buf, true)
            .expect("decodes");
        pos += p.consumed;
        out.extend_from_slice(buf.get(..p.produced).unwrap_or(&[]));
        let live = LIVE.load(Ordering::SeqCst);
        if p.status == Status::FrameDone && at_done.is_none() {
            at_done = Some(live);
        }
        if at_done.is_none() && p.produced > 0 {
            last_delivery = live;
        }
        if p.status == Status::Done {
            break;
        }
    }
    (last_delivery, at_done.expect("a frame ended"), out)
}

fn assert_freed(label: &str, last_delivery: usize, at_done: usize) {
    eprintln!("{label}: live {last_delivery} at last delivery, {at_done} at FrameDone");
    assert!(
        at_done + BIG_BLOCK / 2 <= last_delivery,
        "{label}: scratch outlived its frame: {last_delivery} live then, {at_done} at FrameDone"
    );
}

#[test]
fn streaming_legacy_scratch_is_freed_at_the_end_mark() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (mut f, data) = big_legacy_frame();
    f.extend_from_slice(&0u32.to_le_bytes());
    let (last, done, out) = live_around_frame_end(&f, data.len());
    assert_eq!(out, data);
    assert_freed("end mark", last, done);
}

#[test]
fn streaming_legacy_scratch_is_freed_at_the_next_magic() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (mut f, data) = big_legacy_frame();
    let (second, second_data) = regular_frame(2, false);
    f.extend_from_slice(&second);
    let mut want = data;
    want.extend_from_slice(&second_data);
    let (last, done, out) = live_around_frame_end(&f, want.len());
    assert_eq!(out, want);
    assert_freed("next magic", last, done);
}
