//! 错误类型定义

use thiserror::Error;

/// 分包协议错误类型
#[derive(Debug, Clone, Error)]
pub enum FramingError {
    // ============ 协议错误 ============
    #[error("invalid version: expected 0x01, got 0x{0:02x}")]
    InvalidVersion(u8),
    #[error("invalid frame type: 0x{0:02x}")]
    InvalidFrameType(u8),
    #[error("sequence mismatch: expected {expect}, got {got}")]
    InvalidSeq { expect: u16, got: u16 },
    #[error("total frames conflict: first={first}, got {conflict}")]
    InvalidTotal { first: u16, conflict: u16 },
    #[error("empty payload")]
    EmptyPayload,
    #[error("incomplete header: need 5 bytes, got {0}")]
    IncompleteHeader(usize),
    // ============ 重组错误 ============
    #[error("incomplete frames: received {received}/{total}")]
    IncompleteFrames { received: u16, total: u16 },
    #[error("buffer too small")]
    BufferTooSmall,
    #[error("data checksum mismatch")]
    ChecksumMismatch,
    // ============ IO 错误 ============
    #[error("IO error: {0}")]
    IoError(String),
    // ============ GGWave 错误 ============
    #[error("GGWave encode error: {0}")]
    GGWaveEncode(String),
    #[error("GGWave decode error: {0}")]
    GGWaveDecode(String),
    #[error("WAV error: {0}")]
    WavError(String),
}

/// 错误结果类型别名
pub type Result<T> = std::result::Result<T, FramingError>;

impl From<std::io::Error> for FramingError {
    fn from(e: std::io::Error) -> Self {
        FramingError::IoError(e.to_string())
    }
}
