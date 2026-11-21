//! Snapshot framing and the byte-level reader and writer for its payload.
//!
//! A Snapshot is the 4 bytes `GGLS`, the format version as a little-endian
//! `u32`, then a zlib stream (with its checksum) holding the payload. Every
//! payload number is little-endian. Version 2 lays the payload out as:
//!
//! - width and height, `u32` each
//! - Generation, `u64`
//! - Rule birth and survival masks, `u16` each, bit `n` set when a count of
//!   `n` qualifies
//! - Mutation rate and magnitude, `f64` each
//! - PRNG algorithm version, `u32`, and PRNG state, `u64`
//! - Allele Tree: slot count, `u32`; per slot its id version, `u32`, and a
//!   retained flag, `u8`; a retained slot then holds its hue, `f64`,
//!   Dominance Level, `u8`, a source flag, `u8`, that is 0 for a Founding
//!   Allele and 1 for an Allele that mutated from another, followed in that
//!   case by the slot it mutated from, `u32`, and then its origin
//!   Generation, `u64`; then the free slot count, `u32`, and the free slots,
//!   `u32` each, in the order they were freed
//! - live Cell count, `u32`; per live Cell, in row-major order, its row-major
//!   index and its two Allele slots, `u32` each
//! - population at each Generation `0..=generation`, `u32` each

use miniz_oxide::inflate::TINFLStatus;
use std::fmt;

/// Version of the Snapshot format written by [`crate::World::snapshot`].
pub const SNAPSHOT_VERSION: u32 = 2;

const MAGIC: [u8; 4] = *b"GGLS";

/// Largest payload accepted on import, so corrupt data cannot demand
/// unbounded memory.
const MAX_PAYLOAD_BYTES: usize = 256 << 20;

/// Reasons a Snapshot cannot be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// The bytes do not start with the Snapshot marker.
    NotASnapshot,
    /// The format version is not one this build reads.
    UnsupportedVersion(u32),
    /// The compressed payload is damaged.
    Corrupt,
    /// The data ends before the Snapshot does.
    Truncated,
    /// The payload has bytes left over after the World.
    TrailingBytes,
    /// The PRNG algorithm version is not one this build runs.
    UnsupportedRng(u32),
    /// The payload decodes but does not describe a valid World.
    Invalid(String),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotASnapshot => write!(f, "not a snapshot: the data does not start with GGLS"),
            Self::UnsupportedVersion(version) => write!(
                f,
                "snapshot format version {version} is not supported, this build reads version {SNAPSHOT_VERSION}"
            ),
            Self::Corrupt => write!(f, "snapshot data is corrupt: the compressed payload is damaged"),
            Self::Truncated => write!(f, "snapshot data is truncated"),
            Self::TrailingBytes => write!(f, "snapshot payload has unexpected trailing bytes"),
            Self::UnsupportedRng(version) => write!(
                f,
                "snapshot PRNG algorithm version {version} is not supported, this build runs version {}",
                crate::Rng::VERSION
            ),
            Self::Invalid(reason) => write!(f, "snapshot does not describe a valid world: {reason}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// A [`SnapshotError::Invalid`] giving `reason`.
pub(crate) fn invalid(reason: impl fmt::Display) -> SnapshotError {
    SnapshotError::Invalid(reason.to_string())
}

/// Wraps `payload` as a compressed Snapshot of the current version.
pub(crate) fn seal(payload: &[u8]) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&miniz_oxide::deflate::compress_to_vec_zlib(payload, 6));
    bytes
}

/// The payload of a Snapshot of the current version.
pub(crate) fn open(bytes: &[u8]) -> Result<Vec<u8>, SnapshotError> {
    if bytes.len() < MAGIC.len() || bytes[..MAGIC.len()] != MAGIC {
        return Err(SnapshotError::NotASnapshot);
    }
    let mut header = Reader::new(&bytes[MAGIC.len()..]);
    let version = header.u32()?;
    if version != SNAPSHOT_VERSION {
        return Err(SnapshotError::UnsupportedVersion(version));
    }
    miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(header.rest, MAX_PAYLOAD_BYTES).map_err(
        |error| match error.status {
            TINFLStatus::FailedCannotMakeProgress => SnapshotError::Truncated,
            TINFLStatus::HasMoreOutput => {
                SnapshotError::Invalid(format!("payload is larger than {MAX_PAYLOAD_BYTES} bytes"))
            }
            _ => SnapshotError::Corrupt,
        },
    )
}

/// Appends little-endian numbers to a payload.
#[derive(Default)]
pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a count or index that the World keeps below 2^32.
    pub(crate) fn index(&mut self, value: usize) {
        self.u32(u32::try_from(value).expect("counts and indices fit in u32"));
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
}

/// Reads little-endian numbers from a payload, failing on truncation.
pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], SnapshotError> {
        let (head, rest) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or(SnapshotError::Truncated)?;
        self.rest = rest;
        Ok(*head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, SnapshotError> {
        self.take().map(u8::from_le_bytes)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, SnapshotError> {
        self.take().map(u16::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, SnapshotError> {
        self.take().map(u32::from_le_bytes)
    }

    pub(crate) fn index(&mut self) -> Result<usize, SnapshotError> {
        self.u32().map(|value| value as usize)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, SnapshotError> {
        self.take().map(u64::from_le_bytes)
    }

    pub(crate) fn f64(&mut self) -> Result<f64, SnapshotError> {
        self.take().map(f64::from_le_bytes)
    }

    /// Fails as truncated unless at least `count` items of at least
    /// `item_bytes` each remain, so a corrupt count cannot cause a huge
    /// allocation.
    pub(crate) fn expect_items(&self, count: u64, item_bytes: usize) -> Result<(), SnapshotError> {
        let needed = count.checked_mul(item_bytes as u64);
        if needed.is_some_and(|needed| needed <= self.rest.len() as u64) {
            Ok(())
        } else {
            Err(SnapshotError::Truncated)
        }
    }

    /// Succeeds only when the whole payload has been read.
    pub(crate) fn finish(self) -> Result<(), SnapshotError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(SnapshotError::TrailingBytes)
        }
    }
}
