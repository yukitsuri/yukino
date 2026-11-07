use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-crypto-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_crypto_std_modules_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"crypto_std_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.crypto: *

struct CryptoTestError {
    message: string
}

fun main() {
    set k: bytes = cast("key", "bytes")
    set d: bytes = cast("data", "bytes")
    set h256 = hmac_sha256(ref k, ref d)
    set h512 = hmac_sha512(ref k, ref d)
    if h256.len() != 32 || h512.len() != 64 {
        panic(CryptoTestError { message: "unexpected len" })
    }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    if let Err(e) = run_compiler_driver(&config) {
        panic!("driver compilation failed: {:?}", e);
    }
    assert!(output.exists(), "llvm ir file must be emitted");

    let ir = fs::read_to_string(&output).expect("read llvm ir");
    assert!(ir.contains("hmac_sha256"), "ir must contain hmac_sha256");
    assert!(ir.contains("hmac_sha512"), "ir must contain hmac_sha512");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_crypto_std_binary_execution() {
    let dir = std::env::temp_dir().join(format!("yukic-crypto-std-exec-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create test dir");
    let input = dir.join("main.ykn");
    let app_out = dir.join("crypto_exec_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"crypto_exec_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.crypto: *
load std.hash: hex_to_bytes, bytes_to_hex

struct TestError {
    message: string
    code: int32
}

fun fail(code: int32, msg: string) {
    panic(TestError {
        message: msg,
        code: code,
    })
}

fun test_hmac() {
    set key: bytes = cast("secret-key", "bytes")
    set msg: bytes = cast("The quick brown fox jumps over the lazy dog", "bytes")

    set h256: string = hmac_sha256_hex(ref key, ref msg)
    if h256 != "affee3b4888c714d8369e419b5e51d1ff7c024b64a94d76b8dd53c8fb5d0a2dc" {
        fail(1, "hmac_sha256 mismatch")
    }
    set tag256: bytes = hex_to_bytes(h256)
    if !hmac_sha256_verify(ref key, ref msg, ref tag256) {
        fail(2, "hmac_sha256_verify failed")
    }

    set h512: string = hmac_sha512_hex(ref key, ref msg)
    if h512 != "246be23a6a10781830011e57db63ec7fcd18f284bf002d6d85a0b79358aaf61084c151977e7e99781672271c5d57abcf43c747acac6efda65bc099ad295ff217" {
        fail(3, "hmac_sha512 mismatch")
    }
    set tag512: bytes = hex_to_bytes(h512)
    if !hmac_sha512_verify(ref key, ref msg, ref tag512) {
        fail(4, "hmac_sha512_verify failed")
    }
}

fun test_chacha20() {
    set k_arr: uint8[] = []
    for i in 0..32 { k_arr.push(1u8) }
    set key: bytes = from_raw_parts(k_arr.as_ptr(), 32, 32)

    set n_arr: uint8[] = []
    for i in 0..12 { n_arr.push(2u8) }
    set nonce: bytes = from_raw_parts(n_arr.as_ptr(), 12, 12)

    set msg: bytes = cast("Hello, ChaCha20 world in Yukino!", "bytes")
    set ct: bytes = chacha20_crypt(ref key, ref nonce, 0u32, ref msg)
    set ct_hex: string = bytes_to_hex(ref ct)
    if ct_hex != "4e8494ba05e9e712e992a685fe935a897ece893bc089d97f2cdd89b7641f67a1" {
        fail(11, "chacha20 encryption mismatch")
    }

    set decrypted: bytes = chacha20_crypt(ref key, ref nonce, 0u32, ref ct)
    if cast(decrypted, "string") != "Hello, ChaCha20 world in Yukino!" {
        fail(12, "chacha20 decryption mismatch")
    }
}

fun test_poly1305() {
    set key_hex: string = "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"
    set key: bytes = hex_to_bytes(key_hex)
    set msg: bytes = cast("Cryptographic Forum Research Group", "bytes")
    set tag: bytes = poly1305_mac(ref key, ref msg)
    set tag_hex: string = bytes_to_hex(ref tag)
    if tag_hex != "a8061dc1305136c6c22b8baf0c0127a9" {
        fail(21, "poly1305 rfc vector mismatch: got " + tag_hex)
    }
    if !poly1305_verify(ref key, ref msg, ref tag) {
        fail(22, "poly1305 verify failed")
    }
}

fun test_aead() {
    set k_arr: uint8[] = []
    for i in 0..32 { k_arr.push(4u8) }
    set key: bytes = from_raw_parts(k_arr.as_ptr(), 32, 32)

    set n_arr: uint8[] = []
    for i in 0..12 { n_arr.push(5u8) }
    set nonce: bytes = from_raw_parts(n_arr.as_ptr(), 12, 12)

    set aad: bytes = cast("additional-authenticated-data", "bytes")
    set plaintext: bytes = cast("Sensitive payload needing authenticated encryption", "bytes")

    set ct_tag: bytes = chacha20_poly1305_encrypt(ref key, ref nonce, ref aad, ref plaintext)
    set ct_hex: string = bytes_to_hex(ref ct_tag)
    if ct_hex != "c3f389a0c7691adc636cfac340b8c319713e340913afae387ecb5318cdfd121a2c05070e047fac9ac525ecc09e44b53ef6476d2fad3d7682d5dee18f1d7ebe0ccc92" {
        fail(31, "aead encrypt mismatch")
    }

    set decrypted: bytes = chacha20_poly1305_decrypt(ref key, ref nonce, ref aad, ref ct_tag)
    if cast(decrypted, "string") != "Sensitive payload needing authenticated encryption" {
        fail(32, "aead decrypt mismatch")
    }
}

fun test_x25519() {
    set a_arr: uint8[] = []
    for i in 0..32 { a_arr.push(10u8) }
    set alice_sec: bytes = from_raw_parts(a_arr.as_ptr(), 32, 32)
    set alice_pub: bytes = x25519_public_key(ref alice_sec)
    if bytes_to_hex(ref alice_pub) != "f77ff4b10788bfdca62ca0bb160d427cf5762d85f2b5cad6807ec9c3febbde09" {
        fail(41, "alice x25519 public key mismatch")
    }

    set b_arr: uint8[] = []
    for i in 0..32 { b_arr.push(20u8) }
    set bob_sec: bytes = from_raw_parts(b_arr.as_ptr(), 32, 32)
    set bob_pub: bytes = x25519_public_key(ref bob_sec)
    if bytes_to_hex(ref bob_pub) != "18a6f8c1a7fddf22bd410138f79f7298cd38d1d0a542d4266d556be8609d8862" {
        fail(42, "bob x25519 public key mismatch")
    }

    set shared_a: bytes = x25519_diffie_hellman(ref alice_sec, ref bob_pub)
    set shared_b: bytes = x25519_diffie_hellman(ref bob_sec, ref alice_pub)
    if bytes_to_hex(ref shared_a) != "6a6abbd361cecacbda9053a8c94b400bc0292398b040a6ec1f45004383fe432e" {
        fail(43, "x25519 shared secret mismatch")
    }
    if !constant_time_eq(ref shared_a, ref shared_b) {
        fail(44, "x25519 diffie hellman symmetry mismatch")
    }
}

fun test_ed25519() {
    set seed_arr: uint8[] = []
    for i in 0..32 { seed_arr.push(42u8) }
    set seed: bytes = from_raw_parts(seed_arr.as_ptr(), 32, 32)
    set pubkey: bytes = ed25519_public_key(ref seed)
    if bytes_to_hex(ref pubkey) != "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61" {
        fail(51, "ed25519 public key mismatch")
    }

    set msg: bytes = cast("Ed25519 signed message", "bytes")
    set sig: bytes = ed25519_sign(ref seed, ref msg)
    if bytes_to_hex(ref sig) != "f95197721d1366d5a68ad52d42307778ebf1f0db6fe4ec8ba95c614176565153ee28ccc0008bcc47fa3da96d310ca78b5c0bb1af11687c2a57ca30be6e0a2f04" {
        fail(52, "ed25519 signature mismatch")
    }

    if !ed25519_verify(ref pubkey, ref msg, ref sig) {
        fail(53, "ed25519 verify failed for valid signature")
    }

    set bad_msg: bytes = cast("Ed25519 signed message!", "bytes")
    if ed25519_verify(ref pubkey, ref bad_msg, ref sig) {
        fail(54, "ed25519 verify must fail on tampered message")
    }
}

fun test_hkdf() {
    set salt: bytes = cast("salt", "bytes")
    set ikm: bytes = cast("input-key-material", "bytes")
    set info: bytes = cast("info-context", "bytes")
    set okm: bytes = hkdf(ref salt, ref ikm, ref info, 42)
    set okm_hex: string = bytes_to_hex(ref okm)
    if okm_hex != "95b868a1768662fc53cd52e49e829f01afddd9f7c0b2b30888c24ba239bbcf258f4f097688e79ed2eda6" {
        fail(61, "hkdf mismatch")
    }
}

fun test_pbkdf2() {
    set pass: bytes = cast("password", "bytes")
    set salt: bytes = cast("salt", "bytes")
    set dk: bytes = pbkdf2_sha256(ref pass, ref salt, 1000, 32)
    set dk_hex: string = bytes_to_hex(ref dk)
    if dk_hex != "632c2812e46d4604102ba7618e9d6d7d2f8128f6266b4a03264d2a0460b7dcb3" {
        fail(71, "pbkdf2 mismatch")
    }
}

fun test_random_bytes() {
    set r1: bytes = random_bytes(16)
    set r2: bytes = random_bytes(16)
    if r1.len() != 16 || r2.len() != 16 {
        fail(81, "random_bytes len mismatch")
    }
    if constant_time_eq(ref r1, ref r2) {
        fail(82, "random_bytes collision")
    }
}

fun main() {
    test_hmac()
    test_chacha20()
    test_poly1305()
    test_aead()
    test_x25519()
    test_ed25519()
    test_hkdf()
    test_pbkdf2()
    test_random_bytes()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("build executable");
    assert!(app_out.exists());

    let output = std::process::Command::new(&app_out)
        .output()
        .expect("run executable");
    if !output.status.success() {
        eprintln!("EXEC FAILED status: {:?}", output.status);
        eprintln!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "binary execution must succeed");

    let _ = fs::remove_dir_all(dir);
}
