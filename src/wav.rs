//! WAV 文件读写支持
//!
//! 功能：
//! - 写入多个音频帧到单个 WAV 文件
//! - 从 WAV 文件读取音频帧序列

use crate::error::{FramingError, Result};
use hound::{WavReader, WavSpec, WavWriter};
use std::path::Path;

/// WAV 文件写入器
///
/// 将多个音频帧追加到单个 WAV 文件
pub struct WavFileWriter {
    writer: WavWriter<std::io::BufWriter<std::fs::File>>,
    sample_rate: u32,
    samples_written: u32,
}

impl WavFileWriter {
    /// 创建新的 WAV 文件
    ///
    /// # Arguments
    /// * `path` - 文件路径
    /// * `sample_rate` - 采样率（默认 48000）
    pub fn create(path: &Path, sample_rate: u32) -> Result<Self> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = WavWriter::create(path, spec)
            .map_err(|e| FramingError::WavError(e.to_string()))?;

        Ok(Self {
            writer,
            sample_rate,
            samples_written: 0,
        })
    }

    /// 写入音频样本
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<()> {
        for &sample in samples {
            self.writer
                .write_sample(sample)
                .map_err(|e| FramingError::WavError(e.to_string()))?;
            self.samples_written += 1;
        }
        Ok(())
    }

    /// 写入 f32 样本（转换为 i16）
    pub fn write_samples_f32(&mut self, samples: &[f32]) -> Result<()> {
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            self.writer
                .write_sample(sample_i16)
                .map_err(|e| FramingError::WavError(e.to_string()))?;
            self.samples_written += 1;
        }
        Ok(())
    }

    /// 完成写入
    pub fn finalize(self) -> Result<u32> {
        let duration = self.samples_written / self.sample_rate;
        self.writer
            .finalize()
            .map_err(|e| FramingError::WavError(e.to_string()))?;
        Ok(duration)
    }
}

/// WAV 文件读取器
///
/// 从 WAV 文件中提取音频数据
pub struct WavFileReader {
    reader: WavReader<std::io::BufReader<std::fs::File>>,
}

impl WavFileReader {
    /// 打开 WAV 文件
    pub fn open(path: &Path) -> Result<Self> {
        let reader = WavReader::open(path)
            .map_err(|e| FramingError::IoError(e.to_string()))?;
        Ok(Self { reader })
    }

    /// 获取 WAV 规格
    pub fn spec(&self) -> WavSpec {
        self.reader.spec()
    }

    /// 获取所有音频样本（i16）
    pub fn samples_i16(&mut self) -> Vec<i16> {
        self.reader
            .samples::<i16>()
            .filter_map(|s| s.ok())
            .collect()
    }

    /// 获取所有音频样本（f32）
    pub fn samples_f32(&mut self) -> Vec<f32> {
        self.reader
            .samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / 32767.0)
            .collect()
    }

    /// 获取音频时长（秒）
    pub fn duration_secs(&self) -> f64 {
        let spec = self.reader.spec();
        self.reader.duration() as f64 / spec.sample_rate as f64
    }
}
