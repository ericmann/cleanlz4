//! Streaming decoder state machine (SPEC section 6.1, Streaming).
//!
//! The decoder accepts input in chunks of any size and writes into
//! caller-provided buffers. It buffers only the unit in progress: a frame
//! header (at most 19 bytes), a block header, one block's data with its
//! checksum, or the content checksum. At most one decoded block waits for
//! delivery. Nothing is allocated from a header field.

use crate::error::{Error, Result};
use crate::frame::{
    is_magic_prefix, parse_block_header, parse_frame_descriptor, read_u32_le, BlockHeader,
    FrameDescriptor, FrameState, END_MARK, FRAME_MAGIC,
};
use crate::{DecodeOptions, TrailingData};

/// What a call to [`Decoder::decode`] achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Input bytes taken.
    pub consumed: usize,
    /// Output bytes written.
    pub produced: usize,
    pub status: Status,
}

/// Why a call returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The input is used up and more is needed.
    NeedsInput,
    /// Decoded bytes are waiting and the output buffer is full.
    OutputFull,
    /// A frame ended and all its bytes are delivered.
    FrameDone,
    /// All input is consumed at a frame boundary after `end_of_input`.
    Done,
}

/// Where the state machine is.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// Collecting a frame's magic number in `buf`.
    Boundary,
    /// Collecting a regular frame's header (the magic is already in `buf`).
    Header,
    /// Collecting a 4-byte block size word.
    BlockHeader,
    /// Collecting a block's data and its checksum, `need` bytes in all.
    BlockData { header: BlockHeader, need: usize },
    /// Collecting the 4-byte content checksum.
    Trailer,
    /// After the last frame; nothing more to do.
    Finished,
}

/// What one step of the machine says.
enum Step {
    Continue,
    NeedsInput,
}

/// A streaming decoder.
#[derive(Debug)]
pub struct Decoder {
    options: DecodeOptions,
    stage: Stage,
    /// The unit being collected.
    buf: Vec<u8>,
    /// Decoded bytes not yet delivered, and how many were.
    pending: Vec<u8>,
    pending_pos: usize,
    frame: Option<FrameState>,
    descriptor: Option<FrameDescriptor>,
    /// Regular frames decoded so far.
    frames_done: usize,
    /// A frame ended and `FrameDone` is still to be reported.
    frame_done_pending: bool,
    error: Option<Error>,
}

impl Decoder {
    /// A decoder for a flow of frames under `options`.
    pub fn new(options: DecodeOptions) -> Self {
        Decoder {
            options,
            stage: Stage::Boundary,
            buf: Vec::new(),
            pending: Vec::new(),
            pending_pos: 0,
            frame: None,
            descriptor: None,
            frames_done: 0,
            frame_done_pending: false,
            error: None,
        }
    }

    /// The current or last regular frame's descriptor, once its header is parsed.
    pub fn descriptor(&self) -> Option<&FrameDescriptor> {
        self.descriptor.as_ref()
    }

    /// Whether block and content checksums are verified.
    pub fn checksums_verified(&self) -> bool {
        self.options.verify_checksums
    }

    /// Decode as much as `input` and `output` allow.
    ///
    /// `end_of_input` says no input follows this call's `input`. The status is
    /// the first of: `OutputFull` when decoded bytes wait and `output` is full;
    /// `FrameDone` when a frame ended in this call and its bytes are all
    /// delivered (the call returns there); `Done` when `end_of_input` is set,
    /// all input is consumed and the decoder is at a frame boundary;
    /// otherwise `NeedsInput`. After `Done` every call returns `Done`; after
    /// an error every call returns that error, and bytes written to `output`
    /// by the failing call are not reported.
    pub fn decode(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        end_of_input: bool,
    ) -> Result<Progress> {
        if let Some(e) = self.error {
            return Err(e);
        }
        let mut pos = 0usize;
        let mut produced = 0usize;
        loop {
            // Deliver decoded bytes first.
            if self.pending_pos < self.pending.len() {
                let waiting = self.pending.get(self.pending_pos..).unwrap_or(&[]);
                let room = output.get_mut(produced..).unwrap_or(&mut []);
                let n = waiting.len().min(room.len());
                for (d, s) in room.iter_mut().zip(waiting.iter()) {
                    *d = *s;
                }
                produced = produced.saturating_add(n);
                self.pending_pos = self.pending_pos.saturating_add(n);
                if self.pending_pos < self.pending.len() {
                    return Ok(Progress {
                        consumed: pos,
                        produced,
                        status: Status::OutputFull,
                    });
                }
            }
            if self.pending_pos >= self.pending.len() && !self.pending.is_empty() {
                self.pending.clear();
                self.pending_pos = 0;
            }
            if self.frame_done_pending {
                self.frame_done_pending = false;
                return Ok(Progress {
                    consumed: pos,
                    produced,
                    status: Status::FrameDone,
                });
            }
            if matches!(self.stage, Stage::Finished) {
                return Ok(Progress {
                    consumed: pos,
                    produced,
                    status: Status::Done,
                });
            }
            match self.step(input, &mut pos, end_of_input) {
                Ok(Step::Continue) => {}
                Ok(Step::NeedsInput) => {
                    return Ok(Progress {
                        consumed: pos,
                        produced,
                        status: Status::NeedsInput,
                    });
                }
                Err(e) => {
                    self.error = Some(e);
                    return Err(e);
                }
            }
        }
    }

    /// Move bytes from `input` into `buf` until it holds `need`; true when it does.
    fn gather(&mut self, input: &[u8], pos: &mut usize, need: usize) -> bool {
        let want = need.saturating_sub(self.buf.len());
        let rest = input.get(*pos..).unwrap_or(&[]);
        let take = want.min(rest.len());
        if let Some(bytes) = rest.get(..take) {
            self.buf.extend_from_slice(bytes);
        }
        *pos = pos.saturating_add(take);
        self.buf.len() >= need
    }

    /// The input ran out while a unit was incomplete.
    fn starved(&self, end_of_input: bool) -> Result<Step> {
        if end_of_input {
            Err(Error::TruncatedInput)
        } else {
            Ok(Step::NeedsInput)
        }
    }

    /// A frame ended: report it and look for the next.
    fn end_frame(&mut self) {
        self.frame = None;
        self.frame_done_pending = true;
        self.frames_done = self.frames_done.saturating_add(1);
        self.buf.clear();
        self.stage = Stage::Boundary;
    }

    fn step(&mut self, input: &[u8], pos: &mut usize, end_of_input: bool) -> Result<Step> {
        match self.stage {
            Stage::Boundary => self.step_boundary(input, pos, end_of_input),
            Stage::Header => {
                loop {
                    match parse_frame_descriptor(&self.buf) {
                        Ok((desc, _len)) => {
                            self.descriptor = Some(desc);
                            self.frame = Some(FrameState::new(desc, self.options.verify_checksums));
                            self.buf.clear();
                            self.stage = Stage::BlockHeader;
                            return Ok(Step::Continue);
                        }
                        Err(Error::TruncatedInput) => {
                            // One more byte, then try again.
                            let have = self.buf.len();
                            if !self.gather(input, pos, have.saturating_add(1)) {
                                return self.starved(end_of_input);
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            Stage::BlockHeader => {
                if !self.gather(input, pos, 4) {
                    return self.starved(end_of_input);
                }
                let word = read_u32_le(&self.buf, 0).ok_or(Error::TruncatedInput)?;
                self.buf.clear();
                let desc = self.descriptor.ok_or(Error::TruncatedInput)?;
                if word == END_MARK {
                    if desc.content_checksum {
                        self.stage = Stage::Trailer;
                    } else {
                        self.finish_frame(None)?;
                    }
                    return Ok(Step::Continue);
                }
                let header = parse_block_header(word);
                if header.size > desc.block_max_size.bytes() {
                    return Err(Error::BlockSizeExceeded);
                }
                let need = header
                    .size
                    .saturating_add(if desc.block_checksum { 4 } else { 0 });
                self.stage = Stage::BlockData { header, need };
                Ok(Step::Continue)
            }
            Stage::BlockData { header, need } => {
                if !self.gather(input, pos, need) {
                    return self.starved(end_of_input);
                }
                let mut frame = self.frame.take().ok_or(Error::TruncatedInput)?;
                let data = self.buf.get(..header.size).ok_or(Error::TruncatedInput)?;
                if frame.descriptor().block_checksum {
                    let stored =
                        read_u32_le(&self.buf, header.size).ok_or(Error::TruncatedInput)?;
                    frame.verify_block_checksum(data, stored)?;
                }
                frame.decode_data_block(header, data, &mut self.pending)?;
                self.frame = Some(frame);
                self.buf.clear();
                self.stage = Stage::BlockHeader;
                Ok(Step::Continue)
            }
            Stage::Trailer => {
                if !self.gather(input, pos, 4) {
                    return self.starved(end_of_input);
                }
                let stored = read_u32_le(&self.buf, 0).ok_or(Error::TruncatedInput)?;
                self.finish_frame(Some(stored))?;
                Ok(Step::Continue)
            }
            Stage::Finished => Ok(Step::Continue),
        }
    }

    fn finish_frame(&mut self, stored_content_checksum: Option<u32>) -> Result<()> {
        let mut frame = self.frame.take().ok_or(Error::TruncatedInput)?;
        frame.finish(stored_content_checksum)?;
        self.end_frame();
        Ok(())
    }

    /// At a frame boundary: collect the magic number byte by byte.
    fn step_boundary(&mut self, input: &[u8], pos: &mut usize, end_of_input: bool) -> Result<Step> {
        while self.buf.len() < 4 {
            let Some(&b) = input.get(*pos) else {
                return if end_of_input {
                    if self.buf.is_empty() {
                        self.stage = Stage::Finished;
                        Ok(Step::Continue)
                    } else {
                        Err(Error::TruncatedInput)
                    }
                } else {
                    Ok(Step::NeedsInput)
                };
            };
            let mut candidate = self.buf.clone();
            candidate.push(b);
            let acceptable = if candidate.len() < 4 {
                is_magic_prefix(&candidate)
            } else {
                read_u32_le(&candidate, 0) == Some(FRAME_MAGIC)
            };
            if !acceptable {
                // Not a frame: the byte stays unconsumed.
                if self.frames_done == 0 {
                    return Err(Error::BadMagic);
                }
                return match self.options.trailing_data {
                    TrailingData::Error => Err(Error::TrailingData),
                    TrailingData::ReturnRemainder => {
                        self.stage = Stage::Finished;
                        Ok(Step::Continue)
                    }
                };
            }
            self.buf.push(b);
            *pos = pos.saturating_add(1);
        }
        self.stage = Stage::Header;
        Ok(Step::Continue)
    }
}
