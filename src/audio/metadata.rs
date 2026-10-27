//! Metadata encoding recovery.
/// Recover legacy Japanese tags stored as Latin-1 despite containing CP932 bytes.
/// C1 controls distinguish this case from valid Western or Unicode metadata.
pub fn repair_legacy_tag(text: &str) -> String {
    if !text.chars().any(|c| ('\u{80}'..='\u{9f}').contains(&c)) {
        return text.into();
    }
    let Some(bytes) = text
        .chars()
        .map(|c| u8::try_from(c as u32).ok())
        .collect::<Option<Vec<_>>>()
    else {
        return text.into();
    };
    let Some(decoded) =
        encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(&bytes)
    else {
        return text.into();
    };
    if decoded.chars().any(char::is_control)
        || !decoded.chars().any(|c| {
            ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)
        })
    {
        return text.into();
    }
    decoded.into_owned()
}

#[cfg(test)]
mod tag_tests {
    use super::repair_legacy_tag;

    #[test]
    fn repairs_legacy_japanese_metadata_without_changing_unicode_or_western_tags() {
        let broken = "\u{93}\u{fa}\u{96}{\u{8c}\u{ea}\u{82}\u{cc}\u{8b}\u{c8}\u{81}\u{99}";
        assert_eq!(repair_legacy_tag(broken), "日本語の曲☆");
        for text in [
            "日本語の曲☆",
            "Björk",
            "Sigur Rós",
            "中文",
            "한글",
            "\u{81}",
            "title\u{fffd}",
        ] {
            assert_eq!(repair_legacy_tag(text), text);
        }
    }
}
