//! 使用完整报告测试指定协议的多帧编解码
//! 用法: cargo run --example test_protocol_with_file --features wav -- <PROTOCOL_NAME>

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::fs;

fn test_protocol(name: &str, protocol_id: ggwave_rs::ProtocolId, data: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
    print!("  {:20} ... ", name);

    let codec = GGWaveCodec::with_protocol(protocol_id)?;
    let fragmenter = Fragmenter::new(data.to_vec(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();

    // 编码
    let mut all_audio = Vec::new();
    let mut frame_infos = Vec::new();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let audio = codec.encode_frame(&frame)?;
        frame_infos.push((all_audio.len(), audio.len()));
        all_audio.extend_from_slice(&audio);
    }

    print!("{}帧 ({} bytes) -> ", total_frames, all_audio.len());

    // 解码
    let samples_f32: Vec<f32> = all_audio.chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
    for s in &samples_f32 {
        audio_bytes.extend_from_slice(&s.to_le_bytes());
    }
    drop(samples_f32);
    drop(all_audio);

    let mut decoded_frames = Vec::new();
    for (seq, &(offset, len)) in frame_infos.iter().enumerate() {
        if offset + len <= audio_bytes.len() {
            let frame_audio = &audio_bytes[offset..offset + len];
            match codec.decode_frame(frame_audio) {
                Ok(decoded) => decoded_frames.push(decoded),
                Err(e) => {
                    println!("\n    ❌ 帧 {} 解码失败: {:?}", seq, e);
                    return Ok(false);
                }
            }
        }
    }

    // 重组
    let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
    for frame_data in &decoded_frames {
        deframer.add_full_frame(frame_data)?;
    }

    if !deframer.is_complete() {
        println!("❌ 重组不完整");
        return Ok(false);
    }

    let result = deframer.extract()?;
    if result == data {
        println!("✅ {} bytes 全部正确", result.len());
        Ok(true)
    } else {
        println!("❌ 数据不匹配");
        Ok(false)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法: cargo run --example test_protocol_with_file --features wav -- <PROTOCOL>");
        println!("协议: AUDIBLE_NORMAL, AUDIBLE_FAST, AUDIBLE_FASTEST, ULTRASOUND_FAST, DT_FASTEST");
        return Ok(());
    }

    let protocol_name = &args[1];
    let protocol_id = match protocol_name.as_str() {
        "AUDIBLE_NORMAL" => protocols::AUDIBLE_NORMAL,
        "AUDIBLE_FAST" => protocols::AUDIBLE_FAST,
        "AUDIBLE_FASTEST" => protocols::AUDIBLE_FASTEST,
        "ULTRASOUND_FAST" => protocols::ULTRASOUND_FAST,
        "DT_FASTEST" => protocols::DT_FASTEST,
        _ => {
            println!("未知协议: {}", protocol_name);
            return Ok(());
        }
    };

    let data = fs::read("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md")?;
    println!("=== 测试 {} ===", protocol_name);
    println!("数据大小: {} bytes\n", data.len());

    let result = test_protocol(protocol_name, protocol_id, &data)?;
    if result {
        println!("\n✅ {} 端到端测试通过", protocol_name);
    } else {
        println!("\n❌ {} 测试失败", protocol_name);
    }

    Ok(())
}
