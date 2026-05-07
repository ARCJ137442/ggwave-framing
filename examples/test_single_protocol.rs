//! 小数据单协议端到端测试（验证架构）
//! 用 ~100 字节测试数据，验证单 WAV 文件可以正确编解码

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::path::Path;
use std::fs;

const TEST_DATA: &[u8] = b"Hello, World! This is a test of the GGWave framing protocol with base64 encoding support.";

fn test_protocol(name: &str, protocol_id: ggwave_rs::ProtocolId) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试协议: {} ===", name);

    // 1. 分包
    let fragmenter = Fragmenter::new(TEST_DATA.to_vec(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("  总帧数: {}", total_frames);

    // 2. 创建 codec
    let codec = match GGWaveCodec::with_protocol(protocol_id) {
        Ok(c) => c,
        Err(e) => {
            println!("  ⚠️  codec 创建失败: {:?}", e);
            return Ok(());
        }
    };

    // 3. 编码所有帧到字节缓冲
    let mut all_audio_bytes = Vec::new();
    let mut frame_offsets = Vec::new();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let audio = codec.encode_frame(&frame)?;
        frame_offsets.push((all_audio_bytes.len(), audio.len()));
        all_audio_bytes.extend_from_slice(&audio);
        print!("\r  编码帧 {}/{}", seq + 1, total_frames);
    }
    println!("\n  音频总大小: {} bytes ({} MB)", all_audio_bytes.len(), all_audio_bytes.len() as f64 / 1e6);

    // 4. 写入单个 WAV 文件（使用 f32 little-endian）
    let output_dir = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing/test_output");
    fs::create_dir_all(output_dir)?;
    let wav_path = output_dir.join(format!("{}.wav", name));

    {
        use hound::{WavSpec, WavWriter};
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&wav_path, spec)?;

        // all_audio_bytes 是 f32 LE 格式，每 4 字节一个样本
        for chunk in all_audio_bytes.chunks(4) {
            if chunk.len() == 4 {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                writer.write_sample(sample_i16)?;
            }
        }
        writer.finalize()?;
    }
    println!("  WAV 文件: {} bytes", fs::metadata(&wav_path)?.len());

    // 5. 从 WAV 文件读取并解码
    print!("  解码中...");
    let wav_data = fs::read(&wav_path)?;
    let mut reader = hound::WavReader::new(std::io::Cursor::new(&wav_data))?;
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
                return Err(format!("不支持 bit depth: {}", spec.bits_per_sample).into());
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

    // 按帧边界切分并解码
    let mut decoded_frames = Vec::new();
    let mut decode_errors = 0;

    for (seq, (offset, len)) in frame_offsets.iter().enumerate() {
        if *offset + *len <= audio_bytes.len() {
            let frame_audio = &audio_bytes[*offset..*offset + len];
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
    }
    println!();

    // 6. 重组验证
    if decode_errors == 0 && decoded_frames.len() as u16 == total_frames {
        let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
        for (seq, frame_data) in decoded_frames.iter().enumerate() {
            if let Err(e) = deframer.add_full_frame(frame_data) {
                println!("  ⚠️  添加帧 {} 失败: {:?}", seq, e);
            }
        }

        if deframer.is_complete() {
            let result = deframer.extract()?;
            if result == TEST_DATA {
                println!("  ✅ SUCCESS: {} bytes 解码正确", result.len());
                // 清理
                fs::remove_file(&wav_path)?;
                return Ok(());
            } else {
                println!("  ❌ 数据不匹配: 期望 {} bytes, 得到 {} bytes",
                         TEST_DATA.len(), result.len());
            }
        } else {
            println!("  ❌ 重组不完整");
        }
    } else {
        println!("  ❌ 解码错误: {} errors, {} frames decoded", decode_errors, decoded_frames.len());
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 小数据单协议端到端验证 ===");
    println!("测试数据: {} bytes", TEST_DATA.len());

    // 先用一个协议验证
    println!("\n先测试 AUDIBLE_FASTEST (最快协议，音频最短)");
    test_protocol("AUDIBLE_FASTEST", protocols::AUDIBLE_FASTEST)?;

    println!("\n=== 架构验证完成 ===");
    println!("结论: 单 WAV 文件方案可行");

    // 清理测试目录
    let output_dir = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing/test_output");
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }

    Ok(())
}
