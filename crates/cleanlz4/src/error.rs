//! The typed error enum and result alias (SPEC section 6.2).

/// Every way decoding can fail. No variant carries a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    UnsupportedVersion,
    ReservedBitSet,
    BadHeaderChecksum,
    InvalidBlockMaxSize,
    TruncatedInput,
    ZeroOffset,
    OffsetOutOfRange,
    EndOfBlockViolation,
    BlockChecksumMismatch,
    ContentChecksumMismatch,
    ContentSizeMismatch,
    DictionaryRequired,
    DictionaryMismatch,
    OutputLimitExceeded,
    TrailingData,
    BlockSizeExceeded,
}

impl Error {
    /// Every variant, once.
    pub const ALL: [Error; 17] = [
        Error::BadMagic,
        Error::UnsupportedVersion,
        Error::ReservedBitSet,
        Error::BadHeaderChecksum,
        Error::InvalidBlockMaxSize,
        Error::TruncatedInput,
        Error::ZeroOffset,
        Error::OffsetOutOfRange,
        Error::EndOfBlockViolation,
        Error::BlockChecksumMismatch,
        Error::ContentChecksumMismatch,
        Error::ContentSizeMismatch,
        Error::DictionaryRequired,
        Error::DictionaryMismatch,
        Error::OutputLimitExceeded,
        Error::TrailingData,
        Error::BlockSizeExceeded,
    ];
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Error::BadMagic => "the input does not start with a known magic number",
            Error::UnsupportedVersion => "the frame header names an unsupported format version",
            Error::ReservedBitSet => "a reserved bit in the frame header is set",
            Error::BadHeaderChecksum => "the frame header checksum does not match",
            Error::InvalidBlockMaxSize => "the frame header names an invalid maximum block size",
            Error::TruncatedInput => "the input ended before the data was complete",
            Error::ZeroOffset => "a match has an offset of zero",
            Error::OffsetOutOfRange => {
                "a match offset points before the start of the available data"
            }
            Error::EndOfBlockViolation => "the block breaks an end-of-block rule",
            Error::BlockChecksumMismatch => "a block checksum does not match",
            Error::ContentChecksumMismatch => "the content checksum does not match",
            Error::ContentSizeMismatch => "the decoded size differs from the declared content size",
            Error::DictionaryRequired => "the frame needs a dictionary and none was supplied",
            Error::DictionaryMismatch => {
                "the supplied dictionary does not match the frame's dictionary id"
            }
            Error::OutputLimitExceeded => "the decoded output exceeds the configured limit",
            Error::TrailingData => "unexpected data follows the last frame",
            Error::BlockSizeExceeded => "a block is larger than the frame's maximum block size",
        };
        f.write_str(text)
    }
}

impl std::error::Error for Error {}

/// Result alias used across the crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_variant_once() {
        assert_eq!(Error::ALL.len(), 17);
        for (i, a) in Error::ALL.iter().enumerate() {
            for b in Error::ALL.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
        // An exhaustive match fails to compile when a variant is added.
        for e in Error::ALL {
            match e {
                Error::BadMagic
                | Error::UnsupportedVersion
                | Error::ReservedBitSet
                | Error::BadHeaderChecksum
                | Error::InvalidBlockMaxSize
                | Error::TruncatedInput
                | Error::ZeroOffset
                | Error::OffsetOutOfRange
                | Error::EndOfBlockViolation
                | Error::BlockChecksumMismatch
                | Error::ContentChecksumMismatch
                | Error::ContentSizeMismatch
                | Error::DictionaryRequired
                | Error::DictionaryMismatch
                | Error::OutputLimitExceeded
                | Error::TrailingData
                | Error::BlockSizeExceeded => {}
            }
        }
    }

    #[test]
    fn every_variant_has_a_distinct_display_message() {
        let mut seen = std::collections::HashSet::new();
        for e in Error::ALL {
            let m = e.to_string();
            assert!(!m.is_empty());
            assert!(seen.insert(m));
        }
    }

    #[test]
    fn error_implements_std_error() {
        fn takes(_: &dyn std::error::Error) {}
        takes(&Error::BadMagic);
    }
}
