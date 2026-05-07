//! 编码横纵分析报告示例
//!
//! 将 GGWave_横纵分析报告.md 编码为多个 WAV 文件（每帧独立）
//! 使用 base64 编码处理二进制 frame 数据

use ggwave_framing::{Fragmenter, GGWaveCodec, MAX_PAYLOAD_SIZE};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 项目内输出目录
    let project_dir = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/ggwave-framing");
    let output_dir = project_dir.join("ggwave_frames_output");

    // 下载目录（最终输出）
    let download_dir = PathBuf::from("/data/data/com.termux/files/home/storage/shared/Download/ggwave_report_frames");

    // 报告路径
    let report_path = PathBuf::from("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");

    // 读取报告内容
    println!("Reading report from: {:?}", report_path);
    let data = std::fs::read(&report_path)?;
    println!("Report size: {} bytes", data.len());

    // 创建分包器
    let fragmenter = Fragmenter::new(data, MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("Total frames: {}", total_frames);

    // 估算传输时间
    let est_time = fragmenter.estimate_time(24.0);
    println!("Estimated time: {:.1}s (at 24 B/s)", est_time);

    // 创建 GGWave 编解码器
    let codec = GGWaveCodec::new()?;
    println!("GGWaveCodec initialized\n");

    // 创建输出目录
    std::fs::create_dir_all(&output_dir)?;

    // 编码每一帧为独立 WAV 文件
    println!("Encoding frames to individual WAV files...");

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);

        // 使用 base64 编码
        let wav_data = codec.encode_frame_to_wav(&frame)?;

        // 写入项目目录
        let frame_path = output_dir.join(format!("frame_{:03}.wav", seq));
        std::fs::write(&frame_path, &wav_data)?;

        print!("\rFrame {}/{} ({} bytes -> {} bytes WAV)",
               seq + 1, total_frames, frame.len(), wav_data.len());
    }
    println!("\n\nEncoding complete!");

    // 列出生成的文件
    println!("\nGenerated files in project:");
    let mut total_wav_size = 0u64;
    for seq in 0..total_frames {
        let frame_path = output_dir.join(format!("frame_{:03}.wav", seq));
        let size = std::fs::metadata(&frame_path)?.len();
        total_wav_size += size;
        if seq < 3 || seq >= total_frames - 2 {
            println!("  {}: {} bytes", frame_path.file_name().unwrap().to_string_lossy(), size);
        } else if seq == 3 {
            println!("  ... ({} more files)", total_frames - 5);
        }
    }
    println!("Total WAV size: {} bytes ({:.1} MB)", total_wav_size, total_wav_size as f64 / 1024.0 / 1024.0);

    // 复制到下载目录
    println!("\nCopying to download directory...");
    std::fs::create_dir_all(&download_dir)?;
    for seq in 0..total_frames {
        let src = output_dir.join(format!("frame_{:03}.wav", seq));
        let dst = download_dir.join(format!("frame_{:03}.wav", seq));
        std::fs::copy(&src, &dst)?;
    }
    println!("Copied {} frames to: {:?}", total_frames, download_dir);

    println!("\nTo decode, use decode_report example with the WAV files in:");
    println!("  Project: {:?}", output_dir);
    println!("  Download: {:?}", download_dir);

    Ok(())
}
