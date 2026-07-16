//! Focused tests for macOS key translation result handling.
//!
//! These exercise the pure selection logic that mirrors
//! `KeyNameFromTranslatedString` in `objc/keycode.m` so we can cover empty,
//! single-character, surrogate-pair, and multi-character UCKeyTranslate results
//! without needing a live keyboard layout.

use super::key_name_from_utf16_translation;

#[test]
fn empty_translation_falls_back_to_control_key_name() {
    assert_eq!(
        key_name_from_utf16_translation(&[], Some("f1")),
        Some("f1".to_string())
    );
    assert_eq!(key_name_from_utf16_translation(&[], None), None);
}

#[test]
fn single_control_character_falls_back_to_control_key_name() {
    // Tab is a C0 control character; function/arrow keys surface this way when
    // UCKeyTranslate cannot produce a printable character.
    assert_eq!(
        key_name_from_utf16_translation(&[0x0009], Some("tab")),
        Some("tab".to_string())
    );
    assert_eq!(
        key_name_from_utf16_translation(&[0x001b], Some("escape")),
        Some("escape".to_string())
    );
    // No control-key mapping available for this keycode.
    assert_eq!(key_name_from_utf16_translation(&[0x0009], None), None);
}

#[test]
fn single_printable_character_is_preserved() {
    assert_eq!(
        key_name_from_utf16_translation(&[b'a' as u16], None),
        Some("a".to_string())
    );
    assert_eq!(
        key_name_from_utf16_translation(&[b'!' as u16], Some("f1")),
        Some("!".to_string())
    );
}

#[test]
fn surrogate_pair_is_preserved() {
    // U+1F600 GRINNING FACE encodes as the UTF-16 surrogate pair D83D DE00.
    let grin = "😀";
    let utf16: Vec<u16> = grin.encode_utf16().collect();
    assert_eq!(utf16.len(), 2);
    assert_eq!(
        key_name_from_utf16_translation(&utf16, Some("f1")),
        Some(grin.to_string())
    );
}

#[test]
fn multi_character_sequence_is_preserved() {
    // Some dead-key / multi-unit layouts can emit more than one code unit that
    // is not a single scalar value's surrogate pair (e.g. two BMP characters).
    let utf16 = [b'a' as u16, b'b' as u16];
    assert_eq!(
        key_name_from_utf16_translation(&utf16, Some("f1")),
        Some("ab".to_string())
    );
}

#[test]
fn invalid_utf16_returns_none() {
    // Lone high surrogate is not valid UTF-16.
    assert_eq!(key_name_from_utf16_translation(&[0xD800], None), None);
}