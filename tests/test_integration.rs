//! 端到端集成测试
//!
//! 测试分包编码/解码的完整流程

use ggwave_framing::{Fragmenter, Deframer, GGWaveCodec, MAX_PAYLOAD_SIZE};

#[test]
fn test_single_frame_encode_decode() {
    let test_data = b"Hello, GGWave Framing Protocol! This is a test of the fragmentation system.";

    let fragmenter = Fragmenter::new(test_data.to_vec(), MAX_PAYLOAD_SIZE);
    assert_eq!(fragmenter.total_frames(), 1);

    let codec = GGWaveCodec::new().expect("Failed to create codec");

    let frame = fragmenter.get_frame(0);
    let audio = codec.encode_frame(&frame).expect("Failed to encode");

    let decoded = codec.decode_frame(&audio).expect("Failed to decode");
    let decoded_text = String::from_utf8_lossy(&decoded);

    assert!(decoded_text.contains("Hello"));
}

#[test]
fn test_multi_frame_encode_decode() {
    let test_data: Vec<u8> = (0..500).map(|i| i as u8).collect();

    let fragmenter = Fragmenter::new(test_data.clone(), MAX_PAYLOAD_SIZE);
    let total_frames = fragmenter.total_frames();
    assert_eq!(total_frames, 6);

    let codec = GGWaveCodec::new().expect("Failed to create codec");
    let mut deframer = Deframer::new(total_frames, MAX_PAYLOAD_SIZE);

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let audio = codec.encode_frame(&frame).expect("Failed to encode");
        let decoded = codec.decode_frame(&audio).expect("Failed to decode");
        deframer.add_full_frame(&decoded).expect("Failed to add frame");
    }

    assert!(deframer.is_complete());
    let result = deframer.extract().expect("Failed to extract");
    assert_eq!(result, test_data);
}

#[test]
fn test_frame_header_integrity() {
    use ggwave_framing::{FrameHeader, FrameType};

    let test_data = b"Test data for header integrity check";

    let fragmenter = Fragmenter::new(test_data.to_vec(), 20);
    let total_frames = fragmenter.total_frames();

    for seq in 0..total_frames {
        let frame = fragmenter.get_frame(seq);
        let header = FrameHeader::decode(&frame).expect("Failed to decode header");

        assert_eq!(header.version, 0x01);
        assert_eq!(header.seq, seq as u16);
        assert_eq!(header.total, total_frames);

        if seq == total_frames - 1 {
            assert_eq!(header.frame_type, FrameType::Eof);
        } else {
            assert_eq!(header.frame_type, FrameType::Data);
        }
    }
}

#[test]
fn test_small_data() {
    let test_data = b"X";

    let fragmenter = Fragmenter::new(test_data.to_vec(), MAX_PAYLOAD_SIZE);
    assert_eq!(fragmenter.total_frames(), 1);

    let codec = GGWaveCodec::new().expect("Failed to create codec");
    let frame = fragmenter.get_frame(0);
    let audio = codec.encode_frame(&frame).expect("Failed to encode");
    let decoded = codec.decode_frame(&audio).expect("Failed to decode");

    let decoded_str = String::from_utf8_lossy(&decoded);
    assert!(decoded_str.contains("X"));
}

#[test]
fn test_exact_boundary() {
    let test_data: Vec<u8> = (0..95).map(|i| i as u8).collect();

    let fragmenter = Fragmenter::new(test_data.clone(), MAX_PAYLOAD_SIZE);
    assert_eq!(fragmenter.total_frames(), 1);

    let codec = GGWaveCodec::new().expect("Failed to create codec");
    let frame = fragmenter.get_frame(0);
    let audio = codec.encode_frame(&frame).expect("Failed to encode");
    let decoded = codec.decode_frame(&audio).expect("Failed to decode");

    assert_eq!(decoded.len(), 100); // 5 header + 95 payload
}
