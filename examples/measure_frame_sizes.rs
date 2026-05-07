//! 测量每个协议的每帧音频字节量
//! 用小数据测试，不生成大文件

use ggwave_framing::{Fragmenter, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;

const TEST_DATA: &[u8] = b"Hello, World! This is a test message.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protocols: &[( &str, ggwave_rs::ProtocolId)] = &[
        ("AUDIBLE_NORMAL", protocols::AUDIBLE_NORMAL),
        ("AUDIBLE_FAST", protocols::AUDIBLE_FAST),
        ("AUDIBLE_FASTEST", protocols::AUDIBLE_FASTEST),
        ("ULTRASOUND_NORMAL", protocols::ULTRASOUND_NORMAL),
        ("ULTRASOUND_FAST", protocols::ULTRASOUND_FAST),
        ("DT_NORMAL", protocols::DT_NORMAL),
        ("DT_FAST", protocols::DT_FAST),
        ("DT_FASTEST", protocols::DT_FASTEST),
        ("MT_NORMAL", protocols::MT_NORMAL),
        ("MT_FAST", protocols::MT_FAST),
        ("MT_FASTEST", protocols::MT_FASTEST),
    ];

    for (name, protocol_id) in protocols {
        match GGWaveCodec::with_protocol(*protocol_id) {
            Ok(codec) => {
                let fragmenter = Fragmenter::new(TEST_DATA.to_vec(), MAX_PAYLOAD_SIZE);
                let frame = fragmenter.get_frame(0);
                match codec.encode_frame(&frame) {
                    Ok(audio) => {
                        let audio_secs = audio.len() as f64 / 4f64 / 48000f64; // f32 @ 48kHz
                        println!("{}: {} bytes PCM = {:.2}s per frame, {} frames total",
                            name, audio.len(), audio_secs, fragmenter.total_frames());
                    }
                    Err(e) => println!("{}: encode failed: {:?}", name, e),
                }
            }
            Err(e) => println!("{}: codec failed: {:?}", name, e),
        }
    }

    // 用完整报告估算总大小
    println!("\n--- 完整报告 (20,123 bytes) 估算 ---");
    let full_data = std::fs::read("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md")?;
    let fragmenter = Fragmenter::new(full_data.clone(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("总帧数: {}", total_frames);

    for (name, protocol_id) in protocols {
        match GGWaveCodec::with_protocol(*protocol_id) {
            Ok(codec) => {
                let frame = fragmenter.get_frame(0);
                match codec.encode_frame(&frame) {
                    Ok(audio) => {
                        let per_frame_bytes = audio.len();
                        let total_bytes = per_frame_bytes * total_frames as usize;
                        let total_secs = per_frame_bytes as f64 / 4f64 / 48000f64 * total_frames as f64;
                        let mb = total_bytes as f64 / 1e6;
                        println!("{}: 每帧{}bytes, 总计约{} bytes ({}MB) = {}s",
                            name, per_frame_bytes, total_bytes, mb, total_secs);
                    }
                    Err(e) => println!("{}: encode failed: {:?}", name, e),
                }
            }
            Err(e) => println!("{}: codec failed: {:?}", name, e),
        }
    }

    Ok(())
}
