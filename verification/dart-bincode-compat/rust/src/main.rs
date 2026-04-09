// English: Replicates Crux's BincodeFfiFormat config verbatim:
//   bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
// 中文:完全复制 Crux 的 BincodeFfiFormat 配置。
//
// Serializes a ViewModel identical to crux-template/shared/src/app.rs and
// prints the hex bytes. The Dart side reads the same hex and asserts each
// field round-trips.
// 序列化一个和 crux-template 的 ViewModel 结构一致的对象,打印十六进制字节。
// Dart 侧读同一串十六进制并断言每个字段 round-trip 一致。

use bincode::Options;
use serde::{Deserialize, Serialize};

// ─── Test fixtures ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
struct ViewModel {
    text: String,
    confirmed: bool,
    platform: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct WithOptions {
    maybe_name: Option<String>,
    maybe_count: Option<u32>,
    definitely_none: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WithVec {
    items: Vec<String>,
    numbers: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(C)]
enum Event {
    None,
    Increment,
    SetName(String),
    Move { x: i32, y: i32 },
}

// ─── Helper ──────────────────────────────────────────────────────

fn emit<T: Serialize>(label: &str, value: &T) {
    let options = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    let mut buffer = Vec::new();
    {
        let mut serializer = bincode::Serializer::new(&mut buffer, options);
        value.serialize(&mut serializer).expect("serialize failed");
    }

    print!("{label}: [");
    for (i, b) in buffer.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{b}");
    }
    println!("]  // {} bytes", buffer.len());
}

fn main() {
    // ─── Case 1: Simple struct (ViewModel) ───────────────────────────────
    emit(
        "viewmodel",
        &ViewModel {
            text: "hello".to_string(),
            confirmed: true,
            platform: "iOS".to_string(),
        },
    );

    // ─── Case 2: Option — Some("alice"), Some(42), None ─────────────────
    emit(
        "options_some_some_none",
        &WithOptions {
            maybe_name: Some("alice".to_string()),
            maybe_count: Some(42),
            definitely_none: None,
        },
    );

    // ─── Case 3: Option — all None ──────────────────────────────────────
    emit(
        "options_all_none",
        &WithOptions {
            maybe_name: None,
            maybe_count: None,
            definitely_none: None,
        },
    );

    // ─── Case 4: Vec<String> and Vec<i32> ───────────────────────────────
    emit(
        "vec",
        &WithVec {
            items: vec!["a".to_string(), "bc".to_string(), "def".to_string()],
            numbers: vec![1, -2, 3],
        },
    );

    // ─── Case 5: Enum unit variant (Event::None) ────────────────────────
    emit("event_none", &Event::None);

    // ─── Case 6: Enum unit variant (Event::Increment) ───────────────────
    emit("event_increment", &Event::Increment);

    // ─── Case 7: Enum newtype variant (Event::SetName) ──────────────────
    emit("event_set_name", &Event::SetName("bob".to_string()));

    // ─── Case 8: Enum struct variant (Event::Move) ──────────────────────
    emit("event_move", &Event::Move { x: 10, y: -20 });
}
