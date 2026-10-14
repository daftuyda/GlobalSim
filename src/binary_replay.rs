//! GSIMRP01: length-prefixed metadata plus lossless little-endian f64 frames.
use crate::json::{self, Value};

pub const MAGIC: &[u8; 8] = b"GSIMRP01";

pub fn encode(reply: &mut Value, metadata: &mut String, packet: &mut Vec<u8>) {
    let frames = match reply {
        Value::Object(map) => map.remove("frames"),
        _ => None,
    };
    let frames = match &frames {
        Some(Value::ReplayFrames(frames)) => Some(frames.as_ref()),
        None => None,
        _ => unreachable!("binary output requires numeric replay storage"),
    };
    json::to_string_into(reply, metadata);
    let (frame_count, horse_count) = frames.map_or((0, 0), |f| (f.len(), f.horse_count()));
    let payload_size = frame_count * (8 + horse_count * 48);
    packet.clear();
    packet.reserve(32 + metadata.len() + payload_size);
    packet.extend_from_slice(MAGIC);
    packet.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
    packet.extend_from_slice(&(frame_count as u32).to_le_bytes());
    packet.extend_from_slice(&(horse_count as u32).to_le_bytes());
    packet.extend_from_slice(&(payload_size as u64).to_le_bytes());
    packet.extend_from_slice(metadata.as_bytes());
    if let Some(frames) = frames {
        for i in 0..frame_count {
            let (t, horses) = frames.frame(i);
            packet.extend_from_slice(&t.to_le_bytes());
            for horse in horses {
                for value in horse { packet.extend_from_slice(&value.to_le_bytes()); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay_frames::ReplayFrames;

    #[test]
    fn packet_preserves_every_float_bit_and_selected_frame() {
        let values = [-0., f64::from_bits(1), f64::MAX, f64::INFINITY,
            f64::from_bits(0x7ff8_0000_0000_1234), -1.];
        let mut frames = ReplayFrames::new(1);
        frames.capture(-0., |_| values);
        frames.capture(7., |_| [7.; 6]);
        frames.capture(2., |_| values);
        frames.select(vec![0, 2]);
        let metadata = json::obj(vec![("seed", json::num(17.)), ("text", Value::String("é🦀".into()))]);
        let mut reply = metadata.clone();
        if let Value::Object(map) = &mut reply {
            map.insert("frames".into(), Value::ReplayFrames(Box::new(frames)));
        }
        let mut text = String::new();
        let mut packet = Vec::new();
        encode(&mut reply, &mut text, &mut packet);
        assert_eq!(&packet[..8], MAGIC);
        assert_eq!(u64::from_le_bytes(packet[8..16].try_into().unwrap()) as usize, text.len());
        assert_eq!(u32::from_le_bytes(packet[16..20].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(packet[20..24].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(packet[24..32].try_into().unwrap()), 112);
        assert!(json::parse(&text).unwrap().bitwise_eq(&metadata));
        let expected: Vec<_> = [-0., 2.].into_iter().flat_map(|t|
            std::iter::once(t).chain(values)).flat_map(f64::to_le_bytes).collect();
        assert_eq!(&packet[32 + text.len()..], expected);
        // A subsequent error must not retain any preceding race samples.
        let mut error = json::obj(vec![("error", Value::String("bad request".into()))]);
        encode(&mut error, &mut text, &mut packet);
        assert!(packet[16..32].iter().all(|&b| b == 0));
        assert_eq!(packet.len(), 32 + text.len());
        assert!(json::parse(&text).unwrap().bitwise_eq(&error));
    }
}
