//! 完整报告单协议端到端测试（使用真实帧边界偏移）
//! AUDIBLE_FASTEST + 20,123 字节报告

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 完整报告 AUDIBLE_FASTEST 端到端测试 ===\n");

    let output_dir = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing/test_output");
    fs::create_dir_all(output_dir)?;

    let report_path = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");
    let full_data = fs::read(report_path)?;
    println!("报告大小: {} bytes\n", full_data.len());

    let codec = GGWaveCodec::with_protocol(protocols::AUDIBLE_FASTEST)?;
    let fragmenter = Fragmenter::new(full_data.clone(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("总帧数: {}", total_frames);

    // ========== 编码阶段 ==========
    println!("\n--- 编码阶段 ---");

    // 记录每帧的 (byte_offset, byte_length)
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

    let total_audio_bytes = all_audio_bytes.len();
    let wav_file_bytes = total_audio_bytes / 2; // i16 = f32/2
    println!("  音频总大小: {} bytes ({:.1}MB)", total_audio_bytes, total_audio_bytes as f64 / 1e6);
    println!("  预计 WAV 文件: {} bytes ({:.1}MB)", wav_file_bytes, wav_file_bytes as f64 / 1e6);

    // 写入单一 WAV 文件
    let wav_path = output_dir.join("AUDIBLE_FASTEST_full.wav");
    {
        use hound::{WavSpec, WavWriter};
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&wav_path, spec)?;

        // all_audio_bytes 是 f32 LE，每 4 字节一个样本
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

    let actual_wav_size = fs::metadata(&wav_path)?.len();
    println!("  实际 WAV 大小: {} bytes ({:.1}MB)",
             actual_wav_size, actual_wav_size as f64 / 1e6);

    // ========== 解码阶段 ==========
    println!("\n--- 解码阶段 ---");

    // 加载 WAV 到内存
    let wav_data = fs::read(&wav_path)?;
    println!("  WAV 加载: {} bytes", wav_data.len());

    let mut reader = hound::WavReader::new(std::io::Cursor::new(&wav_data))?;
    let audio_spec = reader.spec();
    println!("  WAV spec: {}Hz, {}ch, {}bit",
             audio_spec.sample_rate, audio_spec.channels, audio_spec.bits_per_sample);

    // 转换为 f32 字节缓冲
    let samples_f32: Vec<f32> = match audio_spec.sample_format {
        hound::SampleFormat::Int => {
            if audio_spec.bits_per_sample == 16 {
                reader.samples::<i16>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / 32767.0)
                    .collect()
            } else {
                return Err(format!("不支持 bit depth: {}", audio_spec.bits_per_sample).into());
            }
        }
        hound::SampleFormat::Float => {
            reader.samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
    };

    let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
    for sample in &samples_f32 {
        audio_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    drop(samples_f32);
    drop(wav_data);
    println!("  音频字节缓冲: {} bytes ({:.1}MB)", audio_bytes.len(), audio_bytes.len() as f64 / 1e6);

    // 打印帧边界验证
    println!("  帧边界验证:");
    for i in 0..std::cmp::min(3, frame_audio_infos.len()) {
        println!("    帧 {}: offset={}, len={}", i, frame_audio_infos[i].0, frame_audio_infos[i].1);
    }
    if frame_audio_infos.len() > 3 {
        let last = frame_audio_infos.len() - 1;
        println!("    ... ({} frames)", frame_audio_infos.len() - 3);
        println!("    帧 {}: offset={}, len={}", last, frame_audio_infos[last].0, frame_audio_infos[last].1);
    }

    // 按记录的帧边界切分解码
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
            println!("\n  ⚠️ 帧 {} 越界: offset={} len={} total={}", seq, offset, len, audio_bytes.len());
        }
        print!("\r  解码帧 {}/{} ({}/{} 成功)",
               seq + 1, total_frames, decoded_frames.len(), total_frames);
    }
    println!();

    // ========== 验证 ==========
    println!("\n--- 验证阶段 ---");
    if decode_errors == 0 && decoded_frames.len() as u16 == total_frames {
        let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
        for (seq, frame_data) in decoded_frames.iter().enumerate() {
            if let Err(e) = deframer.add_full_frame(frame_data) {
                println!("  ⚠️  添加帧 {} 失败: {:?}", seq, e);
            }
        }

        if deframer.is_complete() {
            let result = deframer.extract()?;
            if result == full_data {
                println!("  ✅ SUCCESS: {} bytes 解码完全正确!", result.len());
            } else {
                println!("  ❌ 数据不匹配: 期望 {} bytes, 得到 {} bytes",
                         full_data.len(), result.len());
            }
        } else {
            println!("  ❌ 重组不完整");
        }
    } else {
        println!("  ❌ 解码错误: {} errors, {} frames decoded",
                 decode_errors, decoded_frames.len());
    }

    println!("\n清理测试文件...");
    fs::remove_file(&wav_path)?;
    fs::remove_dir_all(output_dir)?;
    println!("完成");

    Ok(())
}
