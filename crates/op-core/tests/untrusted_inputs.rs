//! Deterministic generated cases supplement targeted import regressions.
use op_core::{document::Document, mining::EventLog};

fn next(seed: &mut u64) -> usize {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 32) as usize
}

#[test]
fn quoted_csv_labels_roundtrip_generated_unicode_and_control_characters() {
    let alphabet: Vec<_> = "abc éñ;,#\"\n\r\t<>".chars().collect();
    let mut seed = 7;
    for delimiter in [',', ';', '\t'] {
        for _ in 0..200 {
            let mut label = "Activity ".to_string();
            for _ in 0..32 {
                label.push(alphabet[next(&mut seed) % alphabet.len()]);
            }
            let encoded = label.replace('"', "\"\"");
            let source = format!("case{delimiter}activity\r\n1{delimiter}\"{encoded}\"\r\n");
            assert_eq!(EventLog::parse(&source).unwrap().traces, vec![vec![label]]);
        }
    }
}

#[test]
fn bounded_generated_imports_return_errors_instead_of_panicking() {
    let alphabet: Vec<_> = "<>/!?&;='\"{}[]:\n abc012é".chars().collect();
    let mut seed = 31;
    for index in 0..1000 {
        let mut source = String::new();
        for _ in 0..(index % 200) {
            source.push(alphabet[next(&mut seed) % alphabet.len()]);
        }
        assert!(std::panic::catch_unwind(|| op_core::xml::parse(&source)).is_ok());
        assert!(std::panic::catch_unwind(|| Document::read(source.as_bytes())).is_ok());
        assert!(std::panic::catch_unwind(|| EventLog::parse(&source)).is_ok());
    }
}
