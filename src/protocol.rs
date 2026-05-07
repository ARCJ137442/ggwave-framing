//! 帧头协议定义
//!
//! 帧头格式（5 bytes）：
//! ```text
//! Byte 0: [版本 4bits | 类型 4bits]
//! Byte 1-2: 序号 (u16, little-endian)
//! Byte 3-4: 总帧数 (u16, little-endian)
//! ```

use crate::error::{FramingError, Result};

/// 协议版本号
const PROTOCOL_VERSION: u8 = 0x01;

/// 帧头大小（字节）
pub const HEADER_SIZE: usize = 5;

/// 最大帧载荷大小
pub const MAX_PAYLOAD_SIZE: usize = 95;

/// 帧类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// 数据帧
    Data = 0x1,
    /// 结束帧（最后一帧）
    Eof = 0x2,
    /// 确认帧（预留）
    Ack = 0x3,
}

impl FrameType {
    /// 从字节解析帧类型
    pub fn from_u8(v: u8) -> Result<Self> {
        match v {
            0x1 => Ok(FrameType::Data),
            0x2 => Ok(FrameType::Eof),
            0x3 => Ok(FrameType::Ack),
            _ => Err(FramingError::InvalidFrameType(v)),
        }
    }

    /// 转换为字节
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// 帧头结构
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    /// 协议版本
    pub version: u8,
    /// 帧类型
    pub frame_type: FrameType,
    /// 帧序号（0-indexed）
    pub seq: u16,
    /// 总帧数
    pub total: u16,
}

impl FrameHeader {
    /// 创建数据帧帧头
    pub fn new_data(seq: u16, total: u16) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            frame_type: FrameType::Data,
            seq,
            total,
        }
    }

    /// 创建结束帧帧头
    pub fn new_eof(seq: u16, total: u16) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            frame_type: FrameType::Eof,
            seq,
            total,
        }
    }

    /// 从字节 slice 解析帧头
    ///
    /// # Arguments
    /// * `data` - 至少 5 字节的数据
    ///
    /// # Errors
    /// * `FramingError::IncompleteHeader` - 数据不足 5 字节
    /// * `FramingError::InvalidVersion` - 版本号不是 0x01
    /// * `FramingError::InvalidFrameType` - 帧类型无效
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(FramingError::IncompleteHeader(data.len()));
        }

        // Byte 0: [版本 4bits | 类型 4bits]
        let version = (data[0] >> 4) & 0x0F;
        let type_nibble = data[0] & 0x0F;

        if version != PROTOCOL_VERSION {
            return Err(FramingError::InvalidVersion(version));
        }

        let frame_type = FrameType::from_u8(type_nibble)?;

        // Bytes 1-2: 序号（小端序）
        let seq = u16::from_le_bytes([data[1], data[2]]);

        // Bytes 3-4: 总帧数（小端序）
        let total = u16::from_le_bytes([data[3], data[4]]);

        Ok(Self {
            version,
            frame_type,
            seq,
            total,
        })
    }

    /// 编码为字节 Vec
    ///
    /// # Returns
    /// 正好 5 字节的 Vec
    pub fn encode(&self) -> Vec<u8> {
        // Byte 0: [版本 4bits | 类型 4bits]
        let byte0 = ((self.version & 0x0F) << 4) | (self.frame_type.to_u8() & 0x0F);

        // Bytes 1-2: 序号
        let seq_bytes = self.seq.to_le_bytes();

        // Bytes 3-4: 总帧数
        let total_bytes = self.total.to_le_bytes();

        vec![byte0, seq_bytes[0], seq_bytes[1], total_bytes[0], total_bytes[1]]
    }

    /// 获取帧类型的字符串表示
    pub fn type_name(&self) -> &'static str {
        match self.frame_type {
            FrameType::Data => "DATA",
            FrameType::Eof => "EOF",
            FrameType::Ack => "ACK",
        }
    }
}

/// 构建完整帧（帧头 + 载荷）
///
/// # Arguments
/// * `header` - 帧头
/// * `payload` - 数据载荷
///
/// # Returns
/// 帧头 + 载荷的 Vec
pub fn build_frame(header: &FrameHeader, payload: &[u8]) -> Vec<u8> {
    let mut frame = header.encode();
    frame.extend_from_slice(payload);
    frame
}

/// 从帧数据提取载荷
///
/// # Arguments
/// * `frame` - 完整帧数据（帧头 + 载荷）
///
/// # Returns
/// 载荷数据的 slice
///
/// # Errors
/// * `FramingError::IncompleteHeader` - 数据不足帧头大小
pub fn extract_payload(frame: &[u8]) -> Result<&[u8]> {
    if frame.len() < HEADER_SIZE {
        return Err(FramingError::IncompleteHeader(frame.len()));
    }
    Ok(&frame[HEADER_SIZE..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encode_decode() {
        let header = FrameHeader::new_data(42, 100);
        let encoded = header.encode();
        assert_eq!(encoded.len(), 5);

        let decoded = FrameHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.version, header.version);
        assert_eq!(decoded.frame_type, header.frame_type);
        assert_eq!(decoded.seq, header.seq);
        assert_eq!(decoded.total, header.total);
    }

    #[test]
    fn test_build_and_extract_frame() {
        let header = FrameHeader::new_data(0, 1);
        let payload = b"Hello, GGWave!";
        let frame = build_frame(&header, payload);

        let extracted = extract_payload(&frame).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn test_invalid_version() {
        let data = [0x20, 0x01, 0x00, 0x01, 0x00]; // version=0x02
        let result = FrameHeader::decode(&data);
        assert!(matches!(result, Err(FramingError::InvalidVersion(_))));
    }

    #[test]
    fn test_invalid_type() {
        let data = [0x10, 0x05, 0x00, 0x01, 0x00]; // type=0x05
        let result = FrameHeader::decode(&data);
        assert!(matches!(result, Err(FramingError::InvalidFrameType(_))));
    }
}
