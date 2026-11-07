use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-encore-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_encore_std_execution() {
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("encore_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"encore_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.encore: *
load std.io: println

struct TestError {
    code: int32
    message: string
}

fun fail(code: int32, msg: string) {
    panic(TestError { code: code, message: msg })
}

fun main() {
    // 1. UTF-8 Validation
    set valid_bytes: bytes = cast("Hello", "bytes")
    if !is_valid_utf8(ref valid_bytes) {
        fail(1, "ascii bytes must be valid utf-8")
    }

    set validated = validate_utf8(ref valid_bytes)
    if validated != "Hello" {
        fail(2, "validated string mismatch")
    }

    set invalid_raw: uint8[] = []
    invalid_raw.push(0xFFu8)
    invalid_raw.push(0xFEu8)
    set invalid_bytes: bytes = from_raw_parts(invalid_raw.as_ptr(), 2, 2)
    if is_valid_utf8(ref invalid_bytes) {
        fail(3, "0xFF 0xFE is not valid utf-8")
    }

    set lossy_res = from_utf8_lossy(ref invalid_bytes)
    if lossy_res.len() == 0 {
        fail(30, "lossy conversion should not be empty")
    }

    // 2. Codepoints
    set sample = "Yukino ❄"
    set cp_count = count_codepoints(sample)
    if cp_count != 8 {
        fail(4, "expected 8 codepoints for Yukino ❄")
    }

    set cps = decode_codepoints(sample)
    if cps.len() != 8 {
        fail(5, "cps len must be 8")
    }
    if cps[0] != 89u32 { // 'Y'
        fail(6, "first codepoint must be 'Y'")
    }

    set re_encoded = encode_codepoints(ref cps)
    if re_encoded != sample {
        fail(7, "roundtrip codepoint encode failed")
    }

    set single_cp = encode_codepoint(65u32) // 'A'
    if single_cp != "A" {
        fail(8, "encode_codepoint 65 should be 'A'")
    }

    set entries = codepoints(sample)
    if entries.len() != 8 {
        fail(9, "codepoints entries count mismatch")
    }

    // 3. Grapheme Clusters
    set combined = "e\u{0301}" // e with combining acute accent
    set g_count = count_graphemes(combined)
    if g_count != 1 {
        fail(10, "e + combining acute should be 1 grapheme cluster")
    }

    set g_list = graphemes(combined)
    if g_list.len() != 1 {
        fail(11, "graphemes list len mismatch")
    }

    set first_g = grapheme_at(sample, 0)
    if first_g == null || (first_g ?? "") != "Y" {
        fail(12, "grapheme_at index 0 should be 'Y'")
    }

    set out_of_bounds_g = grapheme_at(sample, 999)
    if out_of_bounds_g != null {
        fail(13, "grapheme_at out of bounds should be null")
    }

    // 4. UTF-16 Win32 / JVM Interop
    set text = "Привет, Win32!"
    set u16_units = to_utf16(text)
    if u16_units.len() == 0 {
        fail(14, "to_utf16 result empty")
    }

    set restored = from_utf16(ref u16_units)
    if restored != text {
        fail(15, "from_utf16 roundtrip failed")
    }

    set u16_null = to_utf16_null_terminated(text)
    if u16_null.len() != u16_units.len() + 1 {
        fail(16, "null terminated len should be +1")
    }
    if u16_null[u16_null.len() - 1] != 0u16 {
        fail(17, "last element must be 0u16")
    }

    set restored_null = from_utf16_null_terminated(u16_null.as_ptr())
    if restored_null != text {
        fail(18, "from_utf16_null_terminated roundtrip failed")
    }

    // 5. Surrogate helpers
    set high: uint16 = 0xD83Du16
    set low: uint16 = 0xDE00u16
    if !is_high_surrogate(high) {
        fail(19, "0xD83D is high surrogate")
    }
    if !is_low_surrogate(low) {
        fail(20, "0xDE00 is low surrogate")
    }
    if !is_surrogate(high) || !is_surrogate(low) {
        fail(21, "both are surrogates")
    }
    if is_surrogate(65u16) {
        fail(22, "'A' is not surrogate")
    }

    println("Encore tests passed successfully!")
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for encore app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let run_res = std::process::Command::new(&app_out)
        .output()
        .expect("run encore app");
    assert!(
        run_res.status.success(),
        "app execution failed: status={:?}, stdout={}, stderr={}",
        run_res.status,
        String::from_utf8_lossy(&run_res.stdout),
        String::from_utf8_lossy(&run_res.stderr)
    );

    let stdout = String::from_utf8_lossy(&run_res.stdout);
    assert!(
        stdout.contains("Encore tests passed successfully!"),
        "unexpected output: {stdout}"
    );

    let _ = fs::remove_dir_all(&dir);
}
