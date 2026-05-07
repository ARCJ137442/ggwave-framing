//! 项目内完整测试：编码和解码横纵分析报告
use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 项目内输出目录
    let project_dir = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing");
    let frames_dir = project_dir.join("test_frames");
    let output_md = project_dir.join("test_decoded_full.md");

    println!("=== GGWave Framing 完整报告测试 ===\n");

    // 创建测试目录
    std::fs::create_dir_all(&frames_dir)?;

    // 读取完整报告
    let report_path = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");
    let full_data = std::fs::read(&report_path)?;
    println!("Report size: {} bytes", full_data.len());

    // 创建分包器
    let fragmenter = Fragmenter::new(full_data.clone(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("Total frames: {}", total_frames);

    // 估算传输时间
    let est_time = fragmenter.estimate_time(24.0);
    println!("Estimated time: {:.1}s at 24 B/s\n", est_time);

    // 创建编解码器
    let codec = GGWaveCodec::new()?;
    println!("GGWaveCodec initialized\n");

    // 编码每一帧
    println!("Encoding frames...");
    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let wav_data = codec.encode_frame_to_wav(&frame)?;
        let frame_path = frames_dir.join(format!("frame_{:03}.wav", seq));
        std::fs::write(&frame_path, &wav_data)?;
        print!("\rFrame {}/{}", seq + 1, total_frames);
    }
    println!("\nEncoding complete!\n");

    // 解码验证
    println!("Decoding frames...");
    let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);

    for seq in 0..total_frames {
        let frame_path = frames_dir.join(format!("frame_{:03}.wav", seq));
        let wav_data = std::fs::read(&frame_path)?;
        let decoded = codec.decode_frame_from_wav(&wav_data)?;
        deframer.add_full_frame(&decoded)?;
        print!("\rFrame {}/{}", seq + 1, total_frames);
    }
    println!();

    // 检查结果
    if deframer.is_complete() {
        let result = deframer.extract()?;
        if result == full_data {
            println!("\n✅ SUCCESS: {} bytes decoded correctly!", result.len());

            // 保存解码结果
            std::fs::write(&output_md, &result)?;
            println!("Decoded data saved to: {:?}", output_md);

            // 对比原始文件
            println!("\nVerifying against original...");
            let decoded_data = std::fs::read(&output_md)?;
            if decoded_data == full_data {
                println!("✅ Verification PASSED: decoded == original");
            } else {
                println!("❌ Verification FAILED: decoded != original");
            }

            // 清理测试文件
            println!("\nCleaning up test files...");
            std::fs::remove_dir_all(&frames_dir)?;
            std::fs::remove_file(&output_md)?;

            println!("\n=== 完整测试通过！ ===");
        } else {
            println!("\n❌ FAIL: Data mismatch!");
            println!("Expected: {:?}...", &full_data[..50]);
            println!("Got:      {:?}...", &result[..50]);
        }
    } else {
        let (received, total) = deframer.progress();
        println!("\n❌ INCOMPLETE: {}/{} frames", received, total);
    }

    Ok(())
}
