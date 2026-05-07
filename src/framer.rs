//! 分包和重组核心逻辑
//!
//! 本模块实现：
//! - 发送端：数据 -> 分片
//! - 接收端：分片 -> 数据重组

use crate::error::{FramingError, Result};
use crate::protocol::{FrameHeader, HEADER_SIZE};

/// 分包器（发送端）
pub struct Fragmenter {
    total_frames: u16,
    max_payload: usize,
    data: Vec<u8>,
}

impl Fragmenter {
    /// 创建分包器
    ///
    /// # Arguments
    /// * `data` - 要发送的完整数据
    /// * `max_payload` - 每帧最大载荷（默认 95）
    pub fn new(data: impl Into<Vec<u8>>, max_payload: usize) -> Self {
        let data = data.into();
        let total_frames = ((data.len() + max_payload - 1) / max_payload) as u16;
        Self {
            total_frames,
            max_payload,
            data,
        }
    }

    /// 获取总帧数
    pub fn total_frames(&self) -> u16 {
        self.total_frames
    }

    /// 获取指定序号帧的完整数据（帧头 + 载荷）
    ///
    /// # Arguments
    /// * `seq` - 帧序号
    ///
    /// # Returns
    /// 完整帧数据
    ///
    /// # Panics
    /// * `seq >= self.total_frames` 时 panic
    pub fn get_frame(&self, seq: u16) -> Vec<u8> {
        assert!(seq < self.total_frames, "seq out of range");

        let start = (seq as usize) * self.max_payload;
        let end = std::cmp::min(start + self.max_payload, self.data.len());
        let payload = &self.data[start..end];

        let header = if seq == self.total_frames - 1 {
            // 最后一帧
            FrameHeader::new_eof(seq, self.total_frames)
        } else {
            FrameHeader::new_data(seq, self.total_frames)
        };

        let mut frame = header.encode();
        frame.extend_from_slice(payload);
        frame
    }

    /// 获取所有帧的迭代器
    pub fn frames(&self) -> impl Iterator<Item = Vec<u8>> + '_ {
        (0..self.total_frames).map(move |seq| self.get_frame(seq))
    }

    /// 计算传输估算时间（秒）
    ///
    /// # Arguments
    /// * `bytes_per_second` - GGWave 传输速率
    pub fn estimate_time(&self, bytes_per_second: f64) -> f64 {
        let total_bytes = self.total_frames as usize * (self.max_payload + HEADER_SIZE);
        total_bytes as f64 / bytes_per_second
    }
}

/// 重组器（接收端）
pub struct Deframer {
    total_frames: u16,
    max_payload: usize,
    frames: Vec<Option<Vec<u8>>>,  // 按序号存储
    received_count: u16,
}

impl Deframer {
    /// 创建重组器
    ///
    /// # Arguments
    /// * `total_frames` - 总帧数
    /// * `max_payload` - 每帧最大载荷
    pub fn new(total_frames: u16, max_payload: usize) -> Self {
        let frames = vec![None; total_frames as usize];
        Self {
            total_frames,
            max_payload,
            frames,
            received_count: 0,
        }
    }

    /// 获取总帧数
    pub fn total_frames(&self) -> u16 {
        self.total_frames
    }

    /// 获取已接收帧数
    pub fn received_count(&self) -> u16 {
        self.received_count
    }

    /// 添加一帧数据
    ///
    /// # Arguments
    /// * `header` - 已解析的帧头
    /// * `payload` - 载荷数据
    ///
    /// # Errors
    /// * `FramingError::InvalidSeq` - 序号超出范围
    /// * `FramingError::InvalidTotal` - 总帧数与预期不符
    /// * `FramingError::BufferTooSmall` - 载荷超出最大限制
    pub fn add_frame(&mut self, header: &FrameHeader, payload: &[u8]) -> Result<()> {
        // 验证序号范围
        if header.seq >= self.total_frames {
            return Err(FramingError::InvalidSeq {
                expect: self.total_frames - 1,
                got: header.seq,
            });
        }

        // 验证总帧数一致性（如果已有数据）
        if self.received_count > 0 && header.total != self.total_frames {
            return Err(FramingError::InvalidTotal {
                first: self.total_frames,
                conflict: header.total,
            });
        }

        // 更新总帧数（如果有变化）
        if header.total != self.total_frames && self.received_count == 0 {
            self.total_frames = header.total;
            self.frames.resize(header.total as usize, None);
        }

        // 验证载荷大小
        if payload.len() > self.max_payload {
            return Err(FramingError::BufferTooSmall);
        }

        // 如果这帧还没被添加过，才计入
        if self.frames[header.seq as usize].is_none() {
            self.frames[header.seq as usize] = Some(payload.to_vec());
            self.received_count += 1;
        }

        Ok(())
    }

    /// 添加完整帧（帧头 + 载荷）
    pub fn add_full_frame(&mut self, frame: &[u8]) -> Result<()> {
        let header = FrameHeader::decode(frame)?;
        let payload = &frame[HEADER_SIZE..];
        self.add_frame(&header, payload)
    }

    /// 检查是否收齐所有帧
    pub fn is_complete(&self) -> bool {
        self.received_count == self.total_frames
    }

    /// 检查进度（已收 / 总数）
    pub fn progress(&self) -> (u16, u16) {
        (self.received_count, self.total_frames)
    }

    /// 提取重组后的完整数据
    ///
    /// # Errors
    /// * `FramingError::IncompleteFrames` - 帧不完整
    pub fn extract(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            return Err(FramingError::IncompleteFrames {
                received: self.received_count,
                total: self.total_frames,
            });
        }

        let mut result = Vec::with_capacity(
            (self.total_frames as usize - 1) * self.max_payload + self.frames.last().unwrap().as_ref().unwrap().len()
        );

        for frame in &self.frames {
            if let Some(data) = frame {
                result.extend_from_slice(data);
            }
        }

        Ok(result)
    }

    /// 获取丢失的帧序号
    pub fn missing_frames(&self) -> Vec<u16> {
        (0..self.total_frames)
            .filter(|&seq| self.frames[seq as usize].is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DATA: &[u8] = b"Hello, World! This is a test of the GGWave framing protocol.";
    const PAYLOAD_SIZE: usize = 20;

    #[test]
    fn test_fragment_and_defragment() {
        // 分片
        let fragmenter = Fragmenter::new(TEST_DATA, PAYLOAD_SIZE);
        assert_eq!(fragmenter.total_frames(), 3); // ceil(60/20) = 3

        // 重组
        let mut deframer = Deframer::new(fragmenter.total_frames(), PAYLOAD_SIZE);

        for seq in 0..fragmenter.total_frames() {
            let frame = fragmenter.get_frame(seq);
            deframer.add_full_frame(&frame).unwrap();
        }

        assert!(deframer.is_complete());
        let result = deframer.extract().unwrap();
        assert_eq!(result.as_slice(), TEST_DATA);
    }

    #[test]
    fn test_missing_frames() {
        let fragmenter = Fragmenter::new(TEST_DATA, PAYLOAD_SIZE);
        let mut deframer = Deframer::new(fragmenter.total_frames(), PAYLOAD_SIZE);

        // 跳过第 2 帧
        for seq in 0..fragmenter.total_frames() {
            if seq != 2 {
                let frame = fragmenter.get_frame(seq);
                deframer.add_full_frame(&frame).unwrap();
            }
        }

        assert!(!deframer.is_complete());
        assert_eq!(deframer.missing_frames(), vec![2]);
    }

    #[test]
    fn test_estimate_time() {
        let fragmenter = Fragmenter::new(TEST_DATA, PAYLOAD_SIZE);
        let time = fragmenter.estimate_time(24.0); // AUDIBLE_FAST
        assert!(time > 0.0);
        println!("Estimated time: {:.2}s", time);
    }
}
