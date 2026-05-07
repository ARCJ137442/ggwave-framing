//! 多协议端到端测试
//!
//! 测试所有 GGWave 协议变体的完整编解码流程：
//! - 每个协议生成一个包含所有帧的单一 WAV 文件
//! - 成功从单一 WAV 文件解码还原数据
//!
//! 协议列表：AUDIBLE_NORMAL/FAST/FASTEST, ULTRASOUND_NORMAL/FAST, DT_NORMAL/FAST, MT_NORMAL/FAST

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::path::PathBuf;

// 协议配置：(名称, ProtocolId, 预计比特率 B/s)
const PROTOCOLS: &[(&str, ggwave_rs::ProtocolId, f64)] = &[
    ("AUDIBLE_NORMAL", protocols::AUDIBLE_NORMAL, 16.0),
    ("AUDIBLE_FAST", protocols::AUDIBLE_FAST, 24.0),
    ("AUDIBLE_FASTEST", protocols::AUDIBLE_FASTEST, 48.0),
    ("ULTRASOUND_NORMAL", protocols::ULTRASOUND_NORMAL, 16.0),
    ("ULTRASOUND_FAST", protocols::ULTRASOUND_FAST, 24.0),
    ("DT_NORMAL", protocols::DT_NORMAL, 16.0),
    ("DT_FAST", protocols::DT_FAST, 24.0),
    ("MT_NORMAL", protocols::MT_NORMAL, 16.0),
    ("MT_FAST", protocols::MT_FAST, 24.0),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== GGWave Framing 多协议端到端测试 ===\n");

    // 项目目录
    let project_dir = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing");
    let frames_dir = project_dir.join("multi_protocol_frames");
    let report_path = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");

    // 创建测试目录
    std::fs::create_dir_all(&frames_dir)?;

    // 读取报告
    let full_data = std::fs::read(&report_path)?;
    println!("报告大小: {} bytes\n", full_data.len());

    let mut results = Vec::new();

    for (name, protocol_id, _bps) in PROTOCOLS {
        println!("--- 测试协议: {} ---", name);

        // 创建分包器
        let fragmenter = Fragmenter::new(full_data.clone(), MAX_PAYLOAD_SIZE);
        let total_frames = fragmenter.total_frames();
        println!("  总帧数: {}", total_frames);

        // 创建该协议的编解码器
        let codec = match GGWaveCodec::with_protocol(*protocol_id) {
            Ok(c) => c,
            Err(e) => {
                println!("  ⚠️  创建 codec 失败: {:?}\n", e);
                continue;
            }
        };

        // ========== 编码阶段 ==========
        // 追踪每帧的字节偏移和长度
        let mut frame_audio_info: Vec<(usize, usize)> = Vec::new(); // (byte_offset, byte_len)
        let mut all_audio_bytes: Vec<u8> = Vec::new();

        for seq in 0..total_frames {
            let frame = fragmenter.get_frame(seq);
            let audio = codec.encode_frame(&frame)?;

            let byte_offset = all_audio_bytes.len();
            let byte_len = audio.len();
            frame_audio_info.push((byte_offset, byte_len));

            all_audio_bytes.extend_from_slice(&audio);
            print!("\r  编码帧 {}/{} ({} bytes audio, 总计 {} bytes)",
                   seq + 1, total_frames, byte_len, all_audio_bytes.len());
        }
        println!();

        // 写入单一 WAV 文件
        let wav_path = frames_dir.join(format!("{}.wav", name));
        {
            use hound::{WavSpec, WavWriter};
            let spec = WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = WavWriter::create(&wav_path, spec)
                .map_err(|e| format!("创建 WAV 失败: {}", e))?;

            // all_audio_bytes 是 f32 little-endian 的字节表示
            // 每个 f32 样本 4 字节
            for chunk in all_audio_bytes.chunks(4) {
                if chunk.len() == 4 {
                    let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    writer.write_sample(sample_i16)?;
                }
            }
            writer.finalize()?;
        }
        println!("  WAV 文件大小: {} bytes", std::fs::metadata(&wav_path)?.len());

        // ========== 解码阶段 ==========
        print!("  解码中...");
        let mut all_decoded_frames: Vec<Vec<u8>> = Vec::new();

        // 读取 WAV 文件
        let wav_data = std::fs::read(&wav_path)?;
        let mut reader = hound::WavReader::new(std::io::Cursor::new(&wav_data))
            .map_err(|e| format!("读取 WAV 失败: {}", e))?;
        let spec = reader.spec();

        // 转换为 f32 字节
        let samples_f32: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                if spec.bits_per_sample == 16 {
                    reader.samples::<i16>()
                        .filter_map(|s| s.ok())
                        .map(|s| s as f32 / 32767.0)
                        .collect()
                } else {
                    return Err(format!("不支持的 bit depth: {}", spec.bits_per_sample).into());
                }
            }
            hound::SampleFormat::Float => {
                reader.samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect()
            }
        };

        // 转换为字节
        let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
        for sample in samples_f32 {
            audio_bytes.extend_from_slice(&sample.to_le_bytes());
        }

        // 按帧边界切分并解码
        let mut decode_errors = 0;
        for (seq, (byte_offset, byte_len)) in frame_audio_info.iter().enumerate() {
            if *byte_offset + *byte_len <= audio_bytes.len() {
                let frame_audio = &audio_bytes[*byte_offset..*byte_offset + *byte_len];
                match codec.decode_frame(frame_audio) {
                    Ok(decoded) => all_decoded_frames.push(decoded),
                    Err(e) => {
                        decode_errors += 1;
                        if decode_errors <= 3 {
                            println!("\n  ⚠️ 帧 {} 解码失败: {:?}", seq, e);
                        }
                    }
                }
            } else {
                decode_errors += 1;
                println!("\n  ⚠️ 帧 {} 字节范围超限", seq);
            }
            print!("\r  解码帧 {}/{} ({}/{} 成功)",
                   seq + 1, total_frames,
                   all_decoded_frames.len(), total_frames);
        }
        println!();

        // ========== 验证 ==========
        if decode_errors == 0 && all_decoded_frames.len() as u16 == total_frames {
            // 重组数据
            let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
            for (seq, frame_data) in all_decoded_frames.iter().enumerate() {
                if let Err(e) = deframer.add_full_frame(frame_data) {
                    println!("  ⚠️  添加帧 {} 失败: {:?}", seq, e);
                }
            }

            if deframer.is_complete() {
                let result = deframer.extract()?;
                if result == full_data {
                    println!("  ✅ SUCCESS: {} bytes 解码正确!\n", result.len());
                    results.push((*name, true, result.len()));
                } else {
                    println!("  ❌ FAIL: 数据不匹配! 期望 {} bytes, 得到 {} bytes\n",
                             full_data.len(), result.len());
                    results.push((*name, false, result.len()));
                }
            } else {
                let (recv, tot) = deframer.progress();
                println!("  ❌ INCOMPLETE: {}/{} 帧\n", recv, tot);
                results.push((*name, false, 0));
            }
        } else {
            println!("  ❌ DECODE_ERRORS: {} errors, {} frames decoded\n",
                     decode_errors, all_decoded_frames.len());
            results.push((*name, false, 0));
        }
    }

    // ========== 汇总 ==========
    println!("\n=== 测试汇总 ===");
    let mut success_count = 0;
    for (name, ok, bytes) in &results {
        if *ok {
            success_count += 1;
            println!("  ✅ {} - {} bytes", name, bytes);
        } else {
            println!("  ❌ {} - 失败", name);
        }
    }
    println!("\n通过: {}/{}", success_count, results.len());

    // 清理测试文件
    if success_count == results.len() {
        println!("\n清理测试文件...");
        std::fs::remove_dir_all(&frames_dir)?;
    }

    Ok(())
}
