# GGWave Framing Protocol

> 在 GGWave 之上实现的分包协议，支持任意长度数据的可靠传输。

**状态**：✅ 已完成（2026-05-07）

## 功能特性

- **分包协议**：5字节帧头 + 95字节载荷，支持任意长度数据
- **单 WAV 文件**：所有帧编码为单一 WAV 文件，支持从单一文件完整解码还原
- **多协议支持**：AUDIBLE_FASTEST / AUDIBLE_FAST / AUDIBLE_NORMAL
- **Base64 编码**：解决 GGWave 文本协议与二进制数据的不兼容问题

## 协议规格

```
[版本+类型: 1B][序号: 2B][总帧数: 2B][载荷: N bytes]
```

- **版本**：`0x1`（固定）
- **帧类型**：`Data=0x1`，`Eof=0x2`，`Ack=0x3`
- **载荷**：最大 95 字节（REED-SOLOMON GF(256) 域限制）

## 使用示例

```rust
use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};

// 发送端
let data = std::fs::read("report.md")?;
let fragmenter = Fragmenter::new(data, MAX_PAYLOAD_SIZE);
let codec = GGWaveCodec::with_protocol(ggwave_rs::protocols::AUDIBLE_FASTEST)?;

// 编码每一帧
let mut frame_infos = Vec::new();
let mut all_audio = Vec::new();

for seq in 0..fragmenter.total_frames() {
    let frame = fragmenter.get_frame(seq);
    let audio = codec.encode_frame(&frame)?;
    frame_infos.push((all_audio.len(), audio.len()));
    all_audio.extend_from_slice(&audio);
}

// 写入单一 WAV 文件...
// （解码时按 frame_infos 记录的边界切分）
```

## 协议兼容性表（56 字节小数据测试）

| 协议族 | 协议 | 编码 | 解码 | 备注 |
|--------|------|------|------|------|
| AUDIBLE | NORMAL / FAST / FASTEST | ✅ | ✅ | 全部可用 |
| ULTRASOUND | NORMAL / FAST | ✅ | ✅ | 全部可用（载波频率 15-19.5kHz） |
| DT | FASTEST | ✅ | ✅ | 可用 |
| DT | NORMAL / FAST | ✅ | ❌ | 解码失败（DT 对音频特征要求更严格） |
| MT | 所有 | ❌ | - | 编码失败（不支持变长载荷） |

## 测试结果（20,123 字节《横纵分析报告》）

| 协议 | WAV 大小 | 帧数 | 结果 |
|------|----------|------|------|
| AUDIBLE_FASTEST | ~92MB | 212 | ✅ 全部正确 |
| AUDIBLE_FAST | ~170MB | 212 | ✅ 全部正确 |
| AUDIBLE_NORMAL | ~248MB | 212 | ✅ 全部正确 |

## 构建与测试

```bash
# 仅单元测试
cargo test --lib

# 带 WAV 支持的集成测试（必须串行）
cargo test --features wav -- --test-threads=1

# 运行示例
cargo run --example test_full_report_fastest --features wav
```

## 目录结构

```
ggwave-framing/
├── src/
│   ├── lib.rs         # 导出: FramingError, Deframer, Fragmenter, GGWaveCodec
│   ├── error.rs       # 错误类型
│   ├── protocol.rs    # FrameHeader, FrameType
│   ├── framer.rs      # Fragmenter, Deframer
│   ├── codec.rs       # GGWaveCodec (wav feature)
│   └── wav.rs         # WavFileWriter/Reader (wav feature)
├── examples/          # 示例程序
├── tests/             # 集成测试
└── docs/              # 架构文档
```

## 输出文件

- WAV 文件位于：`/storage/shared/Download/ggwave_framing_wavs/`
  - `AUDIBLE_FASTEST.wav`（92MB）
  - `AUDIBLE_FAST.wav`（170MB）
  - `AUDIBLE_NORMAL.wav`（248MB）

## 已知限制

1. **MT 协议不兼容**：Mono-tone 协议要求固定载荷，与分包协议冲突
2. **DT 协议超大**：每帧 2.4-6.9MB，212帧 WAV 达 520MB-1.5GB
3. **实时协议**：GGWave 设计为实时声波传输，20KB 数据需要 10-45 分钟音频

## 参考

- [GGWave 官方](https://github.com/ggerganov/ggwave)
- [ggwave-rs](https://github.com/Thoxy67/ggwave-rs)
- [执行报告](./GGWave_分包协议_执行报告.md)
