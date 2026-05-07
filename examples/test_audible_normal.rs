//! AUDIBLE_NORMAL 完整报告端到端测试

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 完整报告 AUDIBLE_NORMAL 端到端测试 ===\n");

    let output_dir = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing/test_output");
    fs::create_dir_all(output_dir)?;

    let report_path = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");
    let full_data = fs::read(report_path)?;
    println!("报告大小: {} bytes\n", full_data.len());

    let codec = GGWaveCodec::with_protocol(protocols::AUDIBLE_NORMAL)?;
    let fragmenter = Fragmenter::new(full_data.clone(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("总帧数: {}", total_frames);

    // ========== 编码阶段 ==========
    println!("\n--- 编码阶段 ---");

    let mut frame_audio_infos: Vec<(usize, usize)> = Vec::new();
    let mut all_audio_bytes: Vec<u8> = Vec::new();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let audio = codec.encode_frame(&frame)?;

        let offset = all_audio_bytes.len();
        let len = audio.len();
        frame_audio_infos.push((offset, len));

        all_audio_bytes.extend_from_slice(&audio);
        print!("\r  编码帧 {}/{} ({} bytes audio, 总计 {} bytes)",
               seq + 1, total_frames, len, all_audio_bytes.len());
    }
    println!();

    let wav_file_bytes = all_audio_bytes.len() / 2;
    println!("  音频总大小: {} bytes ({:.1}MB)", all_audio_bytes.len(), all_audio_bytes.len() as f64 / 1e6);
    println!("  预计 WAV 文件: {} bytes ({:.1}MB)", wav_file_bytes, wav_file_bytes as f64 / 1e6);

    let wav_path = output_dir.join("AUDIBLE_NORMAL_full.wav");
    {
        use hound::{WavSpec, WavWriter};
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&wav_path, spec)?;

        for chunk in all_audio_bytes.chunks(4) {
            if chunk.len() == 4 {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                writer.write_sample(sample_i16)?;
            }
        }
        writer.finalize()?;
    }
    println!("  WAV 文件写入完成!");

    // ========== 解码阶段 ==========
    println!("\n--- 解码阶段 ---");

    let wav_data = fs::read(&wav_path)?;
    println!("  WAV 加载: {} bytes ({:.1}MB)", wav_data.len(), wav_data.len() as f64 / 1e6);

    let mut reader = hound::WavReader::new(std::io::Cursor::new(&wav_data))?;
    let samples_f32: Vec<f32> = match reader.spec().bits_per_sample {
        16 => reader.samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / 32767.0)
            .collect(),
        _ => return Err("不支持 bit depth".into()),
    };

    let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
    for sample in &samples_f32 {
        audio_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    drop(samples_f32);
    drop(wav_data);
    println!("  音频缓冲: {} bytes ({:.1}MB)", audio_bytes.len(), audio_bytes.len() as f64 / 1e6);

    let mut decoded_frames = Vec::new();
    let mut decode_errors = 0;

    for (seq, &(offset, len)) in frame_audio_infos.iter().enumerate() {
        if offset + len <= audio_bytes.len() {
            let frame_audio = &audio_bytes[offset..offset + len];
            match codec.decode_frame(frame_audio) {
                Ok(decoded) => decoded_frames.push(decoded),
                Err(e) => {
                    decode_errors += 1;
                    if decode_errors <= 3 {
                        println!("\n  ⚠️ 帧 {} 解码失败: {:?}", seq, e);
                    }
                }
            }
        } else {
            decode_errors += 1;
            println!("\n  ⚠️ 帧 {} 越界", seq);
        }
        print!("\r  解码帧 {}/{} ({}/{} 成功)",
               seq + 1, total_frames, decoded_frames.len(), total_frames);
    }
    println!();

    // ========== 验证 ==========
    println!("\n--- 验证阶段 ---");
    if decode_errors == 0 && decoded_frames.len() as u16 == total_frames {
        let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
        for frame_data in &decoded_frames {
            deframer.add_full_frame(frame_data)?;
        }

        if deframer.is_complete() {
            let result = deframer.extract()?;
            if result == full_data {
                println!("  ✅ SUCCESS: {} bytes 解码完全正确!", result.len());
            } else {
                println!("  ❌ 数据不匹配");
            }
        } else {
            println!("  ❌ 重组不完整");
        }
    } else {
        println!("  ❌ 解码错误: {} errors, {} frames", decode_errors, decoded_frames.len());
    }

    println!("\n清理测试文件...");
    fs::remove_file(&wav_path)?;
    fs::remove_dir_all(output_dir)?;
    println!("完成");

    Ok(())
}
