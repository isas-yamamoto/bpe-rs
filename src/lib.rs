//! Bit Plane Encoder — compatible with original/source.

pub mod bitstream;
pub mod decoder;
pub mod encoder;
pub mod error;
pub mod header;
pub mod types;

pub(crate) mod ac;
pub(crate) mod adjust;
pub(crate) mod block;
pub(crate) mod dc;
pub(crate) mod image_io;
pub(crate) mod pattern;
pub(crate) mod rice;
pub(crate) mod stages;
pub(crate) mod wavelet;

pub use error::{BpeError, BpeResult};
pub use types::CodingPara;
