//! 测试所有协议在帧封装协议下的可用性
//! 使用小数据（1帧）确保不 OOM

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;

const TEST_DATA: &[u8] = b"Hello, GGWave Framing Protocol! Testing with small data.";

fn test_protocol(name: &str, protocol_id: ggwave_rs::ProtocolId) -> Result<bool, Box<dyn std::error::Error>> {
    print!("  {:20} ... ", name);

    // 1. 创建 codec
    let codec = match GGWaveCodec::with_protocol(protocol_id) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ codec 创建失败: {:?}", e);
            return Ok(false);
        }
    };

    // 2. 分包
    let fragmenter = Fragmenter::new(TEST_DATA.to_vec(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();

    // 3. 编码
    let mut all_audio = Vec::new();
    let mut frame_infos = Vec::new();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        match codec.encode_frame(&frame) {
            Ok(audio) => {
                frame_infos.push((all_audio.len(), audio.len()));
                all_audio.extend_from_slice(&audio);
            }
            Err(e) => {
                println!("\n    ❌ 帧 {} 编码失败: {:?}", seq, e);
                return Ok(false);
            }
        }
    }

    let audio_mb = all_audio.len() as f64 / 1e6;
    print!("编码 ({} bytes = {:.2}MB) -> ", all_audio.len(), audio_mb);

    // 4. 解码（按帧边界切分）
    // 转换为 f32 字节
    let samples_f32: Vec<f32> = all_audio.chunks(4)
        .filter(|c| c.len() == 4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let mut audio_bytes = Vec::with_capacity(samples_f32.len() * 4);
    for s in &samples_f32 {
        audio_bytes.extend_from_slice(&s.to_le_bytes());
    }

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

    // 5. 重组
    let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);
    for frame_data in &decoded_frames {
        if let Err(e) = deframer.add_full_frame(frame_data) {
            println!("\n    ❌ 添加帧失败: {:?}", e);
            return Ok(false);
        }
    }

    if !deframer.is_complete() {
        println!("❌ 重组不完整");
        return Ok(false);
    }

    let result = deframer.extract()?;
    if result == TEST_DATA {
        println!("✅ {} 字节正确", result.len());
        Ok(true)
    } else {
        println!("❌ 数据不匹配 (期望 {} bytes, 得到 {} bytes)", TEST_DATA.len(), result.len());
        Ok(false)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 所有协议帧封装测试 ===");
    println!("测试数据: {} bytes (1 帧)\n", TEST_DATA.len());

    let protocols: &[(&str, ggwave_rs::ProtocolId)] = &[
        // AUDIBLE 系列
        ("AUDIBLE_NORMAL", protocols::AUDIBLE_NORMAL),
        ("AUDIBLE_FAST", protocols::AUDIBLE_FAST),
        ("AUDIBLE_FASTEST", protocols::AUDIBLE_FASTEST),
        // ULTRASOUND 系列
        ("ULTRASOUND_NORMAL", protocols::ULTRASOUND_NORMAL),
        ("ULTRASOUND_FAST", protocols::ULTRASOUND_FAST),
        // DT 系列
        ("DT_NORMAL", protocols::DT_NORMAL),
        ("DT_FAST", protocols::DT_FAST),
        ("DT_FASTEST", protocols::DT_FASTEST),
        // MT 系列（可能不支持）
        ("MT_NORMAL", protocols::MT_NORMAL),
        ("MT_FAST", protocols::MT_FAST),
        ("MT_FASTEST", protocols::MT_FASTEST),
    ];

    let mut results: Vec<(&str, bool)> = Vec::new();

    for (name, protocol_id) in protocols {
        match test_protocol(name, *protocol_id) {
            Ok(ok) => results.push((name, ok)),
            Err(e) => {
                println!("\n  ⚠️ {}: {}", name, e);
                results.push((name, false));
            }
        }
    }

    println!("\n=== 汇总 ===");
    let mut passed = 0;
    for (name, ok) in &results {
        if *ok {
            println!("  ✅ {}", name);
            passed += 1;
        } else {
            println!("  ❌ {}", name);
        }
    }
    println!("\n通过: {}/{}", passed, results.len());

    Ok(())
}
