const DECODE_FAILED: &'static str = "Tried to decode invalid hex string";

pub fn decode_unsafe(hex: &str) -> Vec<u8> {
  hex
    .split_whitespace()
    .flat_map(|chunk| {
      (0..chunk.len())
        .step_by(2)
        .map(move |i| u8::from_str_radix(&chunk[i..=i+1], 16).expect(DECODE_FAILED))
    }).collect()
}
