//! 生成多协议 WAV 文件到下载目录

use ggwave_framing::{Fragmenter, GGWaveCodec, MAX_PAYLOAD_SIZE};
use ggwave_rs::protocols;
use std::path::Path;
use std::fs;

fn generate_wav(
    name: &str,
    protocol_id: ggwave_rs::ProtocolId,
    output_path: &Path,
    full_data: &[u8],
) -> Result<u64, Box<dyn std::error::Error>> {
    println!("\n=== 生成 {} ===", name);

    let codec = GGWaveCodec::with_protocol(protocol_id)?;
    let fragmenter = Fragmenter::new(full_data.to_vec(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    println!("  总帧数: {}", total_frames);

    // 编码
    let mut frame_infos: Vec<(usize, usize)> = Vec::new();
    let mut all_audio: Vec<u8> = Vec::new();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let audio = codec.encode_frame(&frame)?;
        frame_infos.push((all_audio.len(), audio.len()));
        all_audio.extend_from_slice(&audio);
        print!("\r  编码 {}/{} ({} bytes)", seq + 1, total_frames, all_audio.len());
    }
    println!();

    // 写入 WAV
    {
        use hound::{WavSpec, WavWriter};
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(output_path, spec)?;
        for chunk in all_audio.chunks(4) {
            if chunk.len() == 4 {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                writer.write_sample(sample_i16)?;
            }
        }
        writer.finalize()?;
    }

    let size = fs::metadata(output_path)?.len();
    println!("  WAV 大小: {} bytes ({:.1}MB)", size, size as f64 / 1e6);
    Ok(size)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let download_dir = Path::new("/data/data/com.termux/files/home/storage/shared/Download/ggwave_framing_wavs");

    // 读取报告
    let report_path = Path::new("/data/data/com.termux/files/home/A137442/gibber-link/GGWave_横纵分析报告.md");
    let full_data = fs::read(report_path)?;
    println!("报告大小: {} bytes", full_data.len());

    println!("\n=== 生成 GGWave 分包协议 WAV 文件 ===");
    println!("输出目录: {:?}\n", download_dir);

    let protocols = [
        ("AUDIBLE_FASTEST", protocols::AUDIBLE_FASTEST),
        ("AUDIBLE_FAST", protocols::AUDIBLE_FAST),
        ("AUDIBLE_NORMAL", protocols::AUDIBLE_NORMAL),
    ];

    for (name, protocol_id) in &protocols {
        let wav_path = download_dir.join(format!("{}.wav", name));
        match generate_wav(name, *protocol_id, &wav_path, &full_data) {
            Ok(size) => println!("  ✅ {}", name),
            Err(e) => println!("  ❌ {}: {}", name, e),
        }
        // 强制释放内存
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    println!("\n完成！文件位置: {:?}", download_dir);
    Ok(())
}
