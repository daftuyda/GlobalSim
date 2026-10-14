//! Minimal hand-rolled JSON value parser/serializer -- just enough to load
//! course_data.json/skill_data.json and speak the race-spec stdin/stdout
//! contract, with zero external dependencies (so the final binary needs no
//! crates.io access to build, matching how the rest of this crate was kept
//! dependency-free while the build toolchain was still being set up).
//! Not a general-purpose JSON library: no streaming, no arbitrary-precision
//! numbers (all numbers are f64, which is exactly what every field in this
//! contract needs), no comments/trailing-comma leniency.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ReplayFrame {
    pub t: f64,
    /// Position, speed, HP, lane, rushing mode and front-blocker index.
    pub horses: Vec<[f64; 6]>,
}

/// Complete replay-array JSON, constructible only through ReplayFrameEncoder.
#[derive(Clone, Debug)]
pub struct EncodedReplayFrames {
    json: String,
}

impl EncodedReplayFrames {
    pub fn into_buffer(self) -> String { self.json }
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    ReplayFrames(Box<crate::replay_frames::ReplayFrames>),
    /// Immutable parsed request data shared across worker requests.
    Shared(Arc<Value>),
    /// Output-only frame, serialized directly from its contiguous runner samples.
    RaceFrame(Box<ReplayFrame>),
    /// Output-only, complete array produced by the replay encoder.
    EncodedReplayFrames(EncodedReplayFrames),
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    fn resolved(&self) -> &Self {
        match self {
            Self::Shared(value) => value.resolved(),
            _ => self,
        }
    }

    /// Compare parsed data exactly, including signed zero and all float bits.
    /// Object key order is irrelevant; array order remains significant.
    pub fn bitwise_eq(&self, other: &Self) -> bool {
        if let (Self::Shared(a), Self::Shared(b)) = (self, other) {
            if Arc::ptr_eq(a, b) { return true; }
        }
        match (self.resolved(), other.resolved()) {
            (Self::Null, Self::Null) => true,
            (Self::ReplayFrames(a), Self::ReplayFrames(b)) => a.bitwise_eq(b),
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a.to_bits() == b.to_bits(),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::EncodedReplayFrames(a), Self::EncodedReplayFrames(b)) => a.json == b.json,
            (Self::RaceFrame(a), Self::RaceFrame(b)) => a.t.to_bits() == b.t.to_bits()
                && a.horses.len() == b.horses.len()
                && a.horses.iter().flatten().zip(b.horses.iter().flatten())
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
            (Self::Array(a), Self::Array(b)) => a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| a.bitwise_eq(b)),
            (Self::Object(a), Self::Object(b)) => a.len() == b.len()
                && a.iter().zip(b).all(|((ka, a), (kb, b))| ka == kb && a.bitwise_eq(b)),
            _ => false,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self.resolved() {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }
    pub fn as_str(&self) -> Option<&str> {
        match self.resolved() {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self.resolved() {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self.resolved() {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self.resolved() {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|o| o.get(key))
    }
    pub fn is_null(&self) -> bool {
        matches!(self.resolved(), Value::Null)
    }

    pub fn get_f64(&self, key: &str) -> Result<f64, String> {
        self.get(key).and_then(Value::as_f64).ok_or_else(|| format!("missing/invalid numeric field {key:?}"))
    }
    pub fn get_i64(&self, key: &str) -> Result<i64, String> {
        self.get(key).and_then(Value::as_i64).ok_or_else(|| format!("missing/invalid integer field {key:?}"))
    }
    pub fn get_str<'a>(&'a self, key: &str) -> Result<&'a str, String> {
        self.get(key).and_then(Value::as_str).ok_or_else(|| format!("missing/invalid string field {key:?}"))
    }
    pub fn get_bool(&self, key: &str) -> Result<bool, String> {
        self.get(key).and_then(Value::as_bool).ok_or_else(|| format!("missing/invalid bool field {key:?}"))
    }
    pub fn get_array<'a>(&'a self, key: &str) -> Result<&'a Vec<Value>, String> {
        self.get(key).and_then(Value::as_array).ok_or_else(|| format!("missing/invalid array field {key:?}"))
    }
    pub fn get_object<'a>(&'a self, key: &str) -> Result<&'a BTreeMap<String, Value>, String> {
        self.get(key).and_then(Value::as_object).ok_or_else(|| format!("missing/invalid object field {key:?}"))
    }
}

// ---------------------------------------------------------------------------
// Parsing

// ASCII requests can borrow token text directly. The character-backed path
// retains the existing Unicode whitespace, escape and diagnostic behavior.
trait Input {
    fn len(&self) -> usize;
    fn at(&self, index: usize) -> char;
    fn text(&self, start: usize, end: usize) -> std::borrow::Cow<'_, str>;
    fn matching_end(&self, start: usize, text: &str) -> Option<usize> {
        let mut end = start;
        for c in text.chars() {
            if end == self.len() || self.at(end) != c { return None; }
            end += 1;
        }
        Some(end)
    }
}

impl Input for str {
    fn len(&self) -> usize { str::len(self) }
    fn at(&self, index: usize) -> char { self.as_bytes()[index] as char }
    fn text(&self, start: usize, end: usize) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self[start..end])
    }
    fn matching_end(&self, start: usize, text: &str) -> Option<usize> {
        self[start..].starts_with(text).then_some(start + text.len())
    }
}

impl Input for [char] {
    fn len(&self) -> usize { <[char]>::len(self) }
    fn at(&self, index: usize) -> char { self[index] }
    fn text(&self, start: usize, end: usize) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(self[start..end].iter().collect())
    }
}

pub fn parse(s: &str) -> Result<Value, String> {
    if s.is_ascii() {
        parse_input(s)
    } else {
        parse_input(s.chars().collect::<Vec<_>>().as_slice())
    }
}

struct CachedValue {
    raw: String,
    value: Arc<Value>,
}

/// Reuse one root-level skillData subtree only after exact byte comparison.
/// Cache misses use the existing token parser and diagnostics on both input paths.
#[derive(Default)]
pub struct RequestParser {
    skill_data: Option<CachedValue>,
}

impl RequestParser {
    pub fn parse(&mut self, input: &str) -> Result<Value, String> {
        if input.is_ascii() {
            self.parse_input(input)
        } else {
            self.parse_input(input.chars().collect::<Vec<_>>().as_slice())
        }
    }

    fn parse_input<I: Input + ?Sized>(&mut self, input: &I) -> Result<Value, String> {
        let mut pos = 0;
        skip_ws(input, &mut pos);
        if pos == input.len() || input.at(pos) != '{' { return parse_input(input); }
        let mut replacement = None;
        let value = parse_object_with(input, &mut pos, |key, input, pos| {
            if key != "skillData" { return parse_value(input, pos); }
            skip_ws(input, pos);
            if let Some(cached) = &self.skill_data {
                if let Some(end) = input.matching_end(*pos, &cached.raw) {
                    // A cached number must not consume only a prefix of a new one.
                    if end == input.len() || input.at(end).is_whitespace()
                        || matches!(input.at(end), ',' | '}' | ']') {
                        *pos = end;
                        replacement = None; // The last duplicate key wins.
                        return Ok(Value::Shared(Arc::clone(&cached.value)));
                    }
                }
            }
            let start = *pos;
            let value = Arc::new(parse_value(input, pos)?);
            replacement = Some(CachedValue {
                raw: input.text(start, *pos).into_owned(), value: Arc::clone(&value),
            });
            Ok(Value::Shared(value))
        })?;
        skip_ws(input, &mut pos);
        if pos != input.len() { return Err(format!("trailing data at byte {pos}")); }
        // Malformed requests cannot displace the last successfully parsed table.
        if let Some(cached) = replacement { self.skill_data = Some(cached); }
        Ok(value)
    }
}

fn parse_input<I: Input + ?Sized>(chars: &I) -> Result<Value, String> {
    let mut pos = 0usize;
    let v = parse_value(chars, &mut pos)?;
    skip_ws(chars, &mut pos);
    if pos != chars.len() {
        return Err(format!("trailing data at byte {pos}"));
    }
    Ok(v)
}

fn skip_ws<I: Input + ?Sized>(chars: &I, pos: &mut usize) {
    while *pos < chars.len() && chars.at(*pos).is_whitespace() {
        *pos += 1;
    }
}

fn parse_value<I: Input + ?Sized>(chars: &I, pos: &mut usize) -> Result<Value, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("unexpected end of input".to_string());
    }
    match chars.at(*pos) {
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        '"' => Ok(Value::String(parse_string(chars, pos)?)),
        't' => {
            expect_literal(chars, pos, "true")?;
            Ok(Value::Bool(true))
        }
        'f' => {
            expect_literal(chars, pos, "false")?;
            Ok(Value::Bool(false))
        }
        'n' => {
            expect_literal(chars, pos, "null")?;
            Ok(Value::Null)
        }
        c if c == '-' || c.is_ascii_digit() => parse_number(chars, pos),
        c => Err(format!("unexpected character {c:?} at byte {pos}")),
    }
}

fn expect_literal<I: Input + ?Sized>(chars: &I, pos: &mut usize, lit: &str) -> Result<(), String> {
    for c in lit.chars() {
        if *pos >= chars.len() || chars.at(*pos) != c {
            return Err(format!("expected literal {lit:?} at byte {pos}"));
        }
        *pos += 1;
    }
    Ok(())
}

fn parse_object<I: Input + ?Sized>(chars: &I, pos: &mut usize) -> Result<Value, String> {
    parse_object_with(chars, pos, |_, chars, pos| parse_value(chars, pos))
}

fn parse_object_with<I: Input + ?Sized>(chars: &I, pos: &mut usize,
    mut read_value: impl FnMut(&str, &I, &mut usize) -> Result<Value, String>) -> Result<Value, String> {
    *pos += 1; // '{'
    let mut map = BTreeMap::new();
    skip_ws(chars, pos);
    if *pos < chars.len() && chars.at(*pos) == '}' {
        *pos += 1;
        return Ok(Value::Object(map));
    }
    loop {
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars.at(*pos) != '"' {
            return Err(format!("expected object key at byte {pos}"));
        }
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars.at(*pos) != ':' {
            return Err(format!("expected ':' at byte {pos}"));
        }
        *pos += 1;
        let value = read_value(&key, chars, pos)?;
        map.insert(key, value);
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("unexpected end of input in object".to_string());
        }
        match chars.at(*pos) {
            ',' => {
                *pos += 1;
            }
            '}' => {
                *pos += 1;
                break;
            }
            c => return Err(format!("expected ',' or '}}' at byte {pos}, found {c:?}")),
        }
    }
    Ok(Value::Object(map))
}

fn parse_array<I: Input + ?Sized>(chars: &I, pos: &mut usize) -> Result<Value, String> {
    *pos += 1; // '['
    let mut arr = Vec::new();
    skip_ws(chars, pos);
    if *pos < chars.len() && chars.at(*pos) == ']' {
        *pos += 1;
        return Ok(Value::Array(arr));
    }
    loop {
        let value = parse_value(chars, pos)?;
        arr.push(value);
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("unexpected end of input in array".to_string());
        }
        match chars.at(*pos) {
            ',' => {
                *pos += 1;
            }
            ']' => {
                *pos += 1;
                break;
            }
            c => return Err(format!("expected ',' or ']' at byte {pos}, found {c:?}")),
        }
    }
    Ok(Value::Array(arr))
}

fn parse_string<I: Input + ?Sized>(chars: &I, pos: &mut usize) -> Result<String, String> {
    *pos += 1; // opening quote
    let start = *pos;
    while *pos < chars.len() {
        match chars.at(*pos) {
            '"' => {
                let text = chars.text(start, *pos).into_owned();
                *pos += 1;
                return Ok(text);
            }
            '\\' => break,
            _ => *pos += 1,
        }
    }
    let mut out = chars.text(start, *pos).into_owned();
    loop {
        if *pos >= chars.len() {
            return Err("unterminated string".to_string());
        }
        let c = chars.at(*pos);
        *pos += 1;
        match c {
            '"' => break,
            '\\' => {
                if *pos >= chars.len() {
                    return Err("unterminated escape".to_string());
                }
                let esc = chars.at(*pos);
                *pos += 1;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        if *pos + 4 > chars.len() {
                            return Err("truncated \\u escape".to_string());
                        }
                        let hex = chars.text(*pos, *pos + 4);
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| "invalid \\u escape".to_string())?;
                        *pos += 4;
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                    other => return Err(format!("invalid escape \\{other}")),
                }
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

fn parse_number<I: Input + ?Sized>(chars: &I, pos: &mut usize) -> Result<Value, String> {
    let start = *pos;
    if chars.at(*pos) == '-' {
        *pos += 1;
    }
    while *pos < chars.len() && chars.at(*pos).is_ascii_digit() {
        *pos += 1;
    }
    if *pos < chars.len() && chars.at(*pos) == '.' {
        *pos += 1;
        while *pos < chars.len() && chars.at(*pos).is_ascii_digit() {
            *pos += 1;
        }
    }
    if *pos < chars.len() && (chars.at(*pos) == 'e' || chars.at(*pos) == 'E') {
        *pos += 1;
        if *pos < chars.len() && (chars.at(*pos) == '+' || chars.at(*pos) == '-') {
            *pos += 1;
        }
        while *pos < chars.len() && chars.at(*pos).is_ascii_digit() {
            *pos += 1;
        }
    }
    let text = chars.text(start, *pos);
    text.parse::<f64>().map(Value::Number).map_err(|_| format!("invalid number {text:?}"))
}

// ---------------------------------------------------------------------------
// Serializing

pub fn to_string(v: &Value) -> String {
    let mut out = String::new();
    to_string_into(v, &mut out);
    out
}

/// Replace the buffer contents while retaining capacity for the next response.
pub fn to_string_into(v: &Value, out: &mut String) {
    out.clear();
    write_value(out, v);
}

fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Shared(value) => write_value(out, value),
        Value::ReplayFrames(frames) => {
            let mut cache = Vec::new();
            out.push('[');
            for i in 0..frames.len() {
                if i != 0 { out.push(','); }
                let (t, horses) = frames.frame(i);
                write_cached_frame_parts(out, t, horses, &mut cache);
            }
            out.push(']');
        }
        Value::EncodedReplayFrames(frames) => out.push_str(&frames.json),
        Value::RaceFrame(frame) => {
            out.push_str("{\"horses\":[");
            for (i, horse) in frame.horses.iter().enumerate() {
                if i > 0 { out.push(','); }
                write_horse_frame(out, horse);
            }
            out.push_str("],\"t\":"); write_number(out, frame.t);
            out.push('}');
        }
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => write_number(out, *n),
        Value::String(s) => write_string(out, s),
        Value::Array(arr) => {
            if matches!(arr.first(), Some(Value::RaceFrame(_))) {
                write_replay_frames(out, arr);
                return;
            }
            out.push('[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (i, (k, val)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write_value(out, val);
            }
            out.push('}');
        }
    }
}

// Replay fields often retain the same float bits across adjacent ticks. Keep
// their exact formatted text only for the duration of this frame array.
#[derive(Default)]
struct CachedNumber {
    bits: u64,
    text: String,
}

impl CachedNumber {
    fn write(&mut self, out: &mut String, n: f64) {
        let bits = n.to_bits();
        if self.text.is_empty() || self.bits != bits {
            self.bits = bits;
            self.text.clear();
            write_number(&mut self.text, n);
        }
        out.push_str(&self.text);
    }
}

/// Incrementally encode frame batches while keeping numeric text caches local
/// to one race. A reused buffer is cleared before the first frame is written.
pub struct ReplayFrameEncoder {
    out: String,
    cache: Vec<[CachedNumber; 2]>,
    has_frames: bool,
}

impl ReplayFrameEncoder {
    pub fn new(mut buffer: String) -> Self {
        buffer.clear();
        buffer.push('[');
        Self { out: buffer, cache: Vec::new(), has_frames: false }
    }

    pub fn push(&mut self, frame: &ReplayFrame) {
        if self.has_frames { self.out.push(','); }
        write_cached_replay_frame(&mut self.out, frame, &mut self.cache);
        self.has_frames = true;
    }

    pub fn finish(mut self) -> EncodedReplayFrames {
        self.out.push(']');
        EncodedReplayFrames { json: self.out }
    }
}

fn write_replay_frames(out: &mut String, frames: &[Value]) {
    let mut cache: Vec<[CachedNumber; 2]> = Vec::new();
    out.push('[');
    for (index, value) in frames.iter().enumerate() {
        if index != 0 { out.push(','); }
        let Value::RaceFrame(frame) = value else {
            write_value(out, value);
            continue;
        };
        write_cached_replay_frame(out, frame, &mut cache);
    }
    out.push(']');
}

#[inline]
fn write_cached_replay_frame(out: &mut String, frame: &ReplayFrame,
    cache: &mut Vec<[CachedNumber; 2]>) {
    write_cached_frame_parts(out, frame.t, &frame.horses, cache);
}

#[inline]
fn write_cached_frame_parts(out: &mut String, t: f64, horses: &[[f64; 6]],
    cache: &mut Vec<[CachedNumber; 2]>) {
    cache.resize_with(horses.len(), || std::array::from_fn(|_| CachedNumber::default()));
    out.push_str("{\"horses\":[");
    for (index, (horse, fields)) in horses.iter().zip(cache.iter_mut()).enumerate() {
        if index != 0 { out.push(','); }
        if horse[5] == -1.0 {
            out.push_str("{\"blockFrontHorseIndex\":-1,\"hp\":");
        } else {
            out.push_str("{\"blockFrontHorseIndex\":"); write_small_integer(out, horse[5]);
            out.push_str(",\"hp\":");
        }
        write_number(out, horse[2]);
        out.push_str(",\"lane\":"); fields[0].write(out, horse[3]);
        out.push_str(",\"pos\":"); write_number(out, horse[0]);
        out.push_str(",\"speed\":"); fields[1].write(out, horse[1]);
        if horse[4] == 0.0 {
            out.push_str(",\"temptationMode\":0}");
        } else {
            out.push_str(",\"temptationMode\":"); write_small_integer(out, horse[4]);
            out.push('}');
        }
    }
    out.push_str("],\"t\":"); write_number(out, t);
    out.push('}');
}

fn write_small_integer(out: &mut String, n: f64) {
    const TEXT: [&str; 32] = ["-1", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
        "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22",
        "23", "24", "25", "26", "27", "28", "29", "30"];
    let integer = n as i32;
    if n == integer as f64 && (-1..=30).contains(&integer) {
        out.push_str(TEXT[(integer + 1) as usize]);
    } else {
        write_number(out, n);
    }
}

fn write_horse_frame(out: &mut String, h: &[f64; 6]) {
    // Match the ordinary object's BTreeMap key order and number format.
    out.push_str("{\"blockFrontHorseIndex\":"); write_number(out, h[5]);
    out.push_str(",\"hp\":"); write_number(out, h[2]);
    out.push_str(",\"lane\":"); write_number(out, h[3]);
    out.push_str(",\"pos\":"); write_number(out, h[0]);
    out.push_str(",\"speed\":"); write_number(out, h[1]);
    out.push_str(",\"temptationMode\":"); write_number(out, h[4]);
    out.push('}');
}

fn write_number(out: &mut String, n: f64) {
    if n.is_nan() || n.is_infinite() {
        // Not valid JSON; every producer of these values in this codebase
        // guards against them reaching serialization, but fall back to
        // `null` rather than emit invalid JSON if one ever slips through.
        out.push_str("null");
    } else if n == n.trunc() && n.abs() < 1e15 {
        let _ = write!(out, "{}", n as i64);
    } else if !write_widened_f32(out, n) {
        let _ = write!(out, "{n}");
    }
}

// Format widened f32 values with integer arithmetic, retaining f64 Display's
// shortest decimal text. With magnitude >= 16, the fractional decimal
// coefficient fits in u64. Other values keep the standard formatter.
//
// Before cancelling powers of two, an f32 has `shift` fractional binary digits
// and its widened f64 has half-ULP 2^(-shift-30). After cancelling `trailing`
// bits, multiply the rounding interval by 10^digits: the allowed integer
// distance from the exact coefficient is floor(5^digits / 2^(30+trailing)).
// Round to progressively coarser decimal units only within that interval.
fn write_widened_f32(out: &mut String, n: f64) -> bool {
    // Widening a normal f32 leaves its lower 29 f64 mantissa bits zero.
    // The bit interval is exactly the finite magnitude range [16, 2^23).
    let bits = n.to_bits() & 0x7fff_ffff_ffff_ffff;
    if !(0x4030_0000_0000_0000..0x4160_0000_0000_0000).contains(&bits)
        || bits & 0x1fff_ffff != 0 { return false; }
    let shift = 1046 - (bits >> 52) as u32;
    let mantissa = ((bits >> 29) as u32 & 0x7f_ffff) | 0x80_0000;
    let fraction = mantissa & ((1 << shift) - 1);
    debug_assert!(fraction != 0);
    let trailing = fraction.trailing_zeros();
    let mut digits = shift - trailing;
    const POW5: [u64; 20] = [1, 5, 25, 125, 625, 3125, 15625, 78125,
        390625, 1953125, 9765625, 48828125, 244140625, 1220703125, 6103515625, 30517578125, 152587890625, 762939453125, 3814697265625, 19073486328125];
    let mut decimal = (fraction >> trailing) as u64 * POW5[digits as usize];
    let limit = POW5[digits as usize] >> (30 + trailing);
    // The reduced exact coefficient ends in 5; a distance below 5 cannot
    // remove even one digit. Most large position/HP values take this path.
    if limit >= 5 {
        let exact = decimal;
        let mut unit = 1;
        while unit*10 <= limit*2 { unit *= 10; digits -= 1; }
        // Match Display's choice of the larger magnitude on a decimal tie.
        decimal = (exact + unit / 2) / unit;
        while digits > 0 {
            let next_unit = unit * 10;
            let next = (exact + next_unit / 2) / next_unit;
            if (next * next_unit).abs_diff(exact) > limit { break; }
            decimal = next;
            unit = next_unit;
            digits -= 1;
        }
    }
    const PAIRS: &[u8; 200] = b"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899";
    let mut buffer = [b'0'; 32];
    let mut start = buffer.len() - digits as usize;
    let mut fractional = buffer[start..].rchunks_exact_mut(2);
    for chunk in &mut fractional {
        let pair = (decimal % 100) as usize * 2;
        chunk[0] = PAIRS[pair];
        chunk[1] = PAIRS[pair + 1];
        decimal /= 100;
    }
    if let [digit] = fractional.into_remainder() {
        *digit = b'0' + (decimal % 10) as u8;
    }
    start -= 1;
    buffer[start] = b'.';
    let mut integer = mantissa >> shift;
    while integer >= 100 {
        let pair = (integer % 100) as usize * 2;
        start -= 2;
        buffer[start] = PAIRS[pair];
        buffer[start + 1] = PAIRS[pair + 1];
        integer /= 100;
    }
    if integer >= 10 {
        let pair = integer as usize * 2;
        start -= 2;
        buffer[start] = PAIRS[pair];
        buffer[start + 1] = PAIRS[pair + 1];
    } else {
        start -= 1;
        buffer[start] = b'0' + integer as u8;
    }
    if n.is_sign_negative() {
        start -= 1;
        buffer[start] = b'-';
    }
    // Every byte above is an ASCII digit, sign or decimal point.
    out.push_str(std::str::from_utf8(&buffer[start..]).unwrap());
    true
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    if !s.bytes().any(|b| b == b'"' || b == b'\\' || b < 0x20) {
        out.push_str(s);
        out.push('"');
        return;
    }
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// -- small builder helpers used by main.rs's response assembly -------------

pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn num(n: f64) -> Value {
    Value::Number(n)
}

pub fn arr(items: Vec<Value>) -> Value {
    Value::Array(items)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn check_request(parser: &mut RequestParser, input: &str) {
        match (parser.parse(input), parse(input)) {
            (Ok(actual), Ok(expected)) => {
                assert!(actual.bitwise_eq(&expected), "{input}");
                assert_eq!(to_string(&actual), to_string(&expected), "{input}");
            }
            (Err(actual), Err(expected)) => assert_eq!(actual, expected, "{input}"),
            (actual, expected) => panic!("request parser mismatch for {input:?}: {actual:?} / {expected:?}"),
        }
    }

    #[test]
    fn request_cache_shares_exact_text_and_survives_errors_and_omission() {
        let mut parser = RequestParser::default();
        let input = r#"{"skillData":{"x":[1,-0.0]},"dataSignature":"a"}"#;
        let first = parser.parse(input).unwrap();
        let Value::Shared(first) = first.get("skillData").unwrap() else { panic!("not shared") };
        for request in [input, r#"{"dataSignature":"b","skillData":{"x":[1,-0.0]}}"#] {
            let next = parser.parse(request).unwrap();
            let Value::Shared(next) = next.get("skillData").unwrap() else { panic!("not shared") };
            assert!(Arc::ptr_eq(first, next));
        }
        check_request(&mut parser, r#"{"skillData":{"different":true}} trailing"#);
        check_request(&mut parser, r#"{"nested":{"skillData":[1]}}"#);
        check_request(&mut parser, "{}");
        let restored = parser.parse(input).unwrap();
        let Value::Shared(restored) = restored.get("skillData").unwrap() else { panic!("not shared") };
        assert!(Arc::ptr_eq(first, restored));
        let changed = parser.parse(r#"{"skillData":{"x":[1,0.0]},"dataSignature":"a"}"#).unwrap();
        assert!(!changed.get("skillData").unwrap().bitwise_eq(first));
    }

    #[test]
    fn request_cache_preserves_duplicates_numbers_unicode_and_errors() {
        let mut parser = RequestParser::default();
        for value in ["1", "12", "1.25", "1e20", "-0.0", "0", "null", "true", "false",
                      r#""braces } ] and escaped \" quote""#, "[]", r#"{"a":[1,-0.0,true]}"#] {
            let input = format!(r#"{{"skillData":{value},"seed":2}}"#);
            check_request(&mut parser, &input);
            check_request(&mut parser, &input);
            check_request(&mut parser, &format!(r#"{{"skillData":[],"skillData":{value}}}"#));
            check_request(&mut parser, &format!(r#"{{"sk\u0069llData":{value}}}"#));
            for suffix in ["0", ".1", "e+2", "x", "", " ", ",", "]", "}"] {
                check_request(&mut parser, &format!(r#"{{"skillData":{value}{suffix}}}"#));
            }
        }
        for input in [r#"{"skillData":{},"skillData":{"a":2},"skillData":{}}"#,
                      r#"{"skillData":{"a":1,"a":2}}"#, r#"{"skillData":{"a":2,"a":1}}"#,
                      "\u{2003}{\"skillData\":{\"名\":\"雪\"}}", r#"{"skillData":{"\u540d":"\u96ea"}}"#,
                      "[]", "true", "null", "", "  ", "{"] {
            check_request(&mut parser, input);
            check_request(&mut parser, input);
        }
        // Exercise every truncation and varied malformed tokens around a warm hit.
        let input = r#"{"skillData":{"a":[1,-0.0,"x\\y",null]},"seed":123}"#;
        for end in 0..=input.len() {
            check_request(&mut parser, input);
            check_request(&mut parser, &input[..end]);
        }
        for index in 0..input.len() {
            for byte in [b'0', b'}', b',', b'\\', b'"', b' '] {
                check_request(&mut parser, input);
                let mut mutated = input.as_bytes().to_vec();
                mutated[index] = byte;
                check_request(&mut parser, std::str::from_utf8(&mutated).unwrap());
            }
        }
    }

    #[test]
    fn compact_replay_frames_preserve_object_bytes_and_float_formatting() {
        for values in [
            [1234.5678901234567, 20.25, 1500., 0.3333333432674408, 1., -1.],
            [-0., f64::MIN_POSITIVE, 5e-324, 1e15, 0., 17.],
            [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1e20, 2., 0.],
        ] {
            let object = obj(vec![
                ("pos", num(values[0])), ("speed", num(values[1])),
                ("hp", num(values[2])), ("lane", num(values[3])),
                ("temptationMode", num(values[4])), ("blockFrontHorseIndex", num(values[5])),
            ]);
            let expected = obj(vec![("t", num(values[0])), ("horses", arr(vec![object.clone(), object]))]);
            let actual = Value::RaceFrame(Box::new(ReplayFrame { t: values[0], horses: vec![values, values] }));
            assert_eq!(to_string(&actual), to_string(&expected));
        }
    }

    #[test]
    fn data_comparison_checks_nested_content_and_float_bits() {
        let data = parse(r#"{"a":[1,{"modifier":0}],"b":true}"#).unwrap();
        assert!(data.bitwise_eq(&parse(r#"{"b":true,"a":[1,{"modifier":0.0}]}"#).unwrap()));
        for changed in [
            r#"{"a":[1,{"modifier":-0.0}],"b":true}"#,
            r#"{"a":[1,{"modifier":0.0000000000000001}],"b":true}"#,
            r#"{"a":[1,{"modifier":0}],"b":false}"#,
            r#"{"a":[{"modifier":0},1],"b":true}"#,
            r#"{"a":[1,{"modifier":0}],"b":true,"c":null}"#,
        ] {
            assert!(!data.bitwise_eq(&parse(changed).unwrap()), "{changed}");
        }
    }

    #[test]
    fn ascii_and_character_paths_agree_on_tokens_and_errors() {
        for input in [
            r#"{"name":"horse","values":[1,-0,1.25,2e-3,true,false,null]}"#,
            r#"{"escaped":"\"\\\/\n\t\r\b\f\u65e5"}"#,
            r#""\ud83d\udc0e""#, r#""\uZZZZ""#, r#""\u12""#,
            r#""\q""#, r#""unterminated"#, r#"{"x":1,}"#,
            "", "[1,]", "null extra", "1e+", "-", "[", "true\u{b}",
        ] {
            let chars: Vec<char> = input.chars().collect();
            assert_eq!(format!("{:?}", parse(input)),
                format!("{:?}", parse_input(chars.as_slice())), "{input:?}");
        }
        assert_eq!(parse("[1,]").unwrap_err(), "unexpected character ']' at byte 3");
    }

    #[test]
    fn numeric_tokens_keep_float_bits() {
        for input in ["-0", "-0.0", "0", "9007199254740993", "18446744073709551615",
            "1.2345678901234567", "5e-324", "1e-9999", "1e9999", "-2.2250738585072014e-308"] {
            assert_eq!(parse(input).unwrap().as_f64().unwrap().to_bits(),
                input.parse::<f64>().unwrap().to_bits(), "{input}");
        }
    }

    #[test]
    fn unicode_input_retains_whitespace_and_character_offsets() {
        let value = parse("\u{2003}{\"名前\":\"馬🏇\"}\u{a0}").unwrap();
        assert_eq!(value.get_str("名前").unwrap(), "馬🏇");
        assert_eq!(parse("[\"馬\",]").unwrap_err(), "unexpected character ']' at byte 5");
    }

    #[test]
    fn string_serialization_preserves_escapes_and_unicode() {
        for input in ["plain", "名前🏇", "quote\"slash\\", "\n\r\t\u{8}\u{c}\0"] {
            let encoded = to_string(&Value::String(input.into()));
            assert_eq!(parse(&encoded).unwrap().as_str(), Some(input));
        }
        assert_eq!(to_string(&Value::String("a\0\"\\\n".into())), r#""a\u0000\"\\\n""#);
    }
    #[test]
    fn specialized_numbers_match_the_original_standard_formatter() {
        fn check(n: f64) {
            let expected = if !n.is_finite() {
                "null".to_owned()
            } else if n == n.trunc() && n.abs() < 1e15 {
                format!("{}", n as i64)
            } else {
                format!("{n}")
            };
            let mut actual = String::new();
            write_number(&mut actual, n);
            assert_eq!(actual, expected, "float bits {:016x}", n.to_bits());
        }
        for n in [0., -0., 5e-324, f64::MIN_POSITIVE, f64::MAX, f64::MIN,
            f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e15, -1e15,
            1.09943389892578125, 16., 1024., 8388608.] {
            for bits in [n.to_bits().wrapping_sub(1), n.to_bits(), n.to_bits().wrapping_add(1)] {
                check(f64::from_bits(bits));
            }
        }
        let mut state = 0x2718_2818_3141_5926u64;
        for index in 0..30_000u32 {
            state ^= state << 13; state ^= state >> 7; state ^= state << 17;
            let narrowed = f32::from_bits(((131 + index % 19) << 23)
                | (state as u32 & 0x7f_ffff)) as f64;
            for n in [narrowed, -narrowed, f64::from_bits(narrowed.to_bits() + 1),
                f32::from_bits(state as u32) as f64, f64::from_bits(state)] {
                check(n);
            }
        }
    }

    #[test]
    fn replay_number_cache_preserves_transitions_and_array_shapes() {
        let values = [
            [1234.5678901234567, 20.25, 1500., 0.3333333432674408, 1., -1.],
            [16.000001907348633, 20.25, 512.0000610351562, 0.3333333432674408, 0., 30.],
            [-0., -0., f64::MIN_POSITIVE, -0., -2., 31.],
            [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e20, 0.5, f64::NAN],
        ];
        let mut frames = Vec::new();
        let mut objects = Vec::new();
        for (i, count) in [0, 1, 4, 4, 2, 0, 4].into_iter().enumerate() {
            let horses: Vec<_> = (0..count).map(|h| values[(h + i / 2) % values.len()]).collect();
            objects.push(obj(vec![("t", num(i as f64 / 15.)), ("horses", arr(horses.iter().map(|h| {
                obj(vec![("pos", num(h[0])), ("speed", num(h[1])), ("hp", num(h[2])),
                    ("lane", num(h[3])), ("temptationMode", num(h[4])), ("blockFrontHorseIndex", num(h[5]))])
            }).collect()))]));
            frames.push(Value::RaceFrame(Box::new(ReplayFrame { t: i as f64 / 15., horses })));
            if i == 3 { frames.push(Value::Null); objects.push(Value::Null); }
        }
        assert_eq!(to_string(&arr(frames)), to_string(&arr(objects)));
    }

    #[test]
    fn response_buffer_reuses_capacity_and_replaces_previous_contents() {
        let mut out = String::with_capacity(8192);
        for value in [Value::String("long response ".repeat(300)), Value::Null,
            obj(vec![("error", Value::String("invalid request".into()))]),
            arr(vec![num(-0.), num(16.000001907348633), Value::String("\u{99ac}".into())])] {
            to_string_into(&value, &mut out);
            assert_eq!(out, to_string(&value));
            assert_eq!(out.capacity(), 8192);
            out.push('\n'); // The worker appends its line delimiter before reuse.
        }
    }

    #[test]
    fn replay_batch_encoder_matches_arrays_and_reuses_finished_storage() {
        let samples = [
            [1234.5678901234567, 20.25, 1500., 0.3333333432674408, 0., -1.],
            [16.000001907348633, 20.25, 512.0000610351562, 0.3333333432674408, 1., 30.],
            [-0., -0., f64::MIN_POSITIVE, -0., -2., 31.],
            [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e20, 0.5, f64::NAN],
        ];
        let frames: Vec<_> = [0, 1, 4, 4, 2, 0, 4].into_iter().enumerate().map(|(i, count)| {
            ReplayFrame { t: i as f64 / 15., horses: (0..count)
                .map(|h| samples[(h + i / 2) % samples.len()]).collect() }
        }).collect();
        let mut buffer = String::with_capacity(8192);
        for count in [0, frames.len(), 1, 0, frames.len()] {
            for batch_size in [1, 2, 3, 8] {
                let mut encoder = ReplayFrameEncoder::new(buffer);
                for batch in frames[..count].chunks(batch_size) {
                    for frame in batch { encoder.push(frame); }
                }
                let encoded = encoder.finish();
                let expected = to_string(&arr(frames[..count].iter().cloned()
                    .map(|frame| Value::RaceFrame(Box::new(frame))).collect()));
                let value = Value::EncodedReplayFrames(encoded);
                let actual = to_string(&value);
                assert_eq!(actual, expected);
                assert_eq!(parse(&actual).unwrap().as_array().unwrap().len(), count);
                let Value::EncodedReplayFrames(encoded) = value else { unreachable!() };
                buffer = encoded.into_buffer();
                assert_eq!(buffer.capacity(), 8192);
            }
        }
    }

}
