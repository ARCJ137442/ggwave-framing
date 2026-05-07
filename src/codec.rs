//! GGWave 编码/解码封装
//!
//! 不修改 ggwave-rs，仅在其基础上封装分层协议
//! 注意：GGWave 期望 UTF-8 文本，因此二进制 frame 数据需要 base64 编码

use crate::error::{FramingError, Result};
use ggwave_rs::{GGWave, protocols};
use hound::WavReader;
use std::io::Cursor;

/// GGWave 编解码器
pub struct GGWaveCodec {
    gg: GGWave,
    protocol: ggwave_rs::ProtocolId,
}

impl GGWaveCodec {
    /// 创建新的编解码器（使用 AUDIBLE_FAST 协议）
    pub fn new() -> Result<Self> {
        Self::with_protocol(protocols::AUDIBLE_FAST)
    }

    /// 使用指定协议创建编解码器
    pub fn with_protocol(protocol: ggwave_rs::ProtocolId) -> Result<Self> {
        let gg = GGWave::new()
            .map_err(|e| FramingError::GGWaveEncode(e.to_string()))?;
        Ok(Self { gg, protocol })
    }

    /// 编码 frame 数据为音频
    ///
    /// 自动将二进制 frame 数据转换为 base64，然后编码为 UTF-8 文本
    pub fn encode_frame(&self, frame: &[u8]) -> Result<Vec<u8>> {
        let b64 = base64_encode(frame);
        self.gg
            .encode(&b64, self.protocol, 50)
            .map_err(|e| FramingError::GGWaveEncode(e.to_string()))
    }

    /// 编码为 WAV 格式
    pub fn encode_frame_to_wav(&self, frame: &[u8]) -> Result<Vec<u8>> {
        let b64 = base64_encode(frame);
        self.gg
            .encode_to_wav(&b64, self.protocol, 50)
            .map_err(|e| FramingError::GGWaveEncode(e.to_string()))
    }

    /// 解码音频为 frame 数据
    ///
    /// 将解码的 UTF-8 文本从 base64 转换回二进制
    pub fn decode_frame(&self, audio: &[u8]) -> Result<Vec<u8>> {
        let b64 = self.gg
            .decode_to_string(audio, 2048)
            .map_err(|e| FramingError::GGWaveDecode(e.to_string()))?;
        base64_decode(&b64).map_err(|e| FramingError::GGWaveDecode(e))
    }

    /// 从 WAV 数据解码
    pub fn decode_frame_from_wav(&self, wav_data: &[u8]) -> Result<Vec<u8>> {
        let mut reader = WavReader::new(Cursor::new(wav_data))
            .map_err(|e| FramingError::WavError(e.to_string()))?;
        let spec = reader.spec();

        let samples_f32: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                if spec.bits_per_sample == 16 {
                    reader.samples::<i16>()
                        .filter_map(|s| s.ok())
                        .map(|s| s as f32 / 32767.0)
                        .collect()
                } else {
                    return Err(FramingError::WavError("Unsupported bit depth".into()));
                }
            }
            hound::SampleFormat::Float => {
                reader.samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect()
            }
        };

        let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
        for sample in samples_f32 {
            audio_bytes.extend_from_slice(&sample.to_le_bytes());
        }

        self.decode_frame(&audio_bytes)
    }
}

impl Default for GGWaveCodec {
    fn default() -> Self {
        Self::new().expect("failed to create GGWaveCodec")
    }
}

// Simple base64 encoding (URL-safe no-padding)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        }
        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3F] as char);
        }
    }
    result
}

// Simple base64 decoding
fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    fn char_to_val(c: char) -> std::result::Result<u8, String> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '-' => Ok(62),
            '_' => Ok(63),
            _ => Err(format!("Invalid base64 char: {}", c)),
        }
    }

    let chars: Vec<char> = s.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let v0 = char_to_val(chars[i])?;
        let v1 = if i + 1 < chars.len() { char_to_val(chars[i + 1])? } else { 0 };

        result.push((v0 << 2) | (v1 >> 4));

        if i + 2 < chars.len() {
            let v2 = char_to_val(chars[i + 2])?;
            result.push((v1 << 4) | (v2 >> 2));

            if i + 3 < chars.len() {
                let v3 = char_to_val(chars[i + 3])?;
                result.push((v2 << 6) | v3);
            }
        }

        i += 4;
    }

    Ok(result)
}
