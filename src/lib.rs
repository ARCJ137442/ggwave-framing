//! GGWave Framing Protocol
//!
//! 一个在 GGWave 之上实现的分包协议，支持任意长度数据的可靠传输

pub mod error;
pub mod framer;
pub mod protocol;

#[cfg(feature = "wav")]
pub mod codec;
#[cfg(feature = "wav")]
pub mod wav;

// 重新导出常用类型
pub use error::{FramingError, Result};
pub use framer::{Deframer, Fragmenter};
pub use protocol::{FrameHeader, FrameType, HEADER_SIZE, MAX_PAYLOAD_SIZE};

// 重新导出 GGWaveCodec（需要 wav feature）
#[cfg(feature = "wav")]
pub use codec::GGWaveCodec;
