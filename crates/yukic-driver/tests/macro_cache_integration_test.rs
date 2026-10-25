#![deny(unsafe_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(100);

fn unique_test_dir(name: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("yukic_macro_cache_{name}_{}_{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create test dir");
    dir
}

fn create_schema_macro_script(dir: &PathBuf) -> PathBuf {
    let script_path = dir.join("mock_schema_runner.py");
    let script_content = r#"#!/usr/bin/env python3
import sys
import struct
import json
import os

header = sys.stdin.buffer.read(4)
if not header or len(header) < 4:
    sys.exit(0)

msg_len = struct.unpack('<I', header)[0]
payload = sys.stdin.buffer.read(msg_len).decode('utf-8')
req = json.loads(payload)

script_dir = os.path.dirname(os.path.abspath(__file__))
schema_path = os.path.join(script_dir, "schema.txt")
val = "default_val"
if os.path.exists(schema_path):
    with open(schema_path, "r", encoding="utf-8") as f:
        val = f.read().strip()

resp = {
    "success": True,
    "output_tokens": f'public fun get_schema_data() -> string {{ return "{val}" }}',
    "tracked_files": [schema_path],
    "diagnostics": [],
    "error_message": None
}

resp_bytes = json.dumps(resp).encode('utf-8')
sys.stdout.buffer.write(struct.pack('<I', len(resp_bytes)))
sys.stdout.buffer.write(resp_bytes)
sys.stdout.buffer.flush()
"#;

    fs::write(&script_path, script_content).expect("write mock runner");
    let mut perms = fs::metadata(&script_path).expect("get perms").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set exec perms");

    // flush file handle write lease
    let f = fs::File::open(&script_path).expect("open to sync");
    let _ = f.sync_all();
    std::thread::sleep(std::time::Duration::from_millis(20));

    script_path
}

#[test]
fn test_macro_tracked_file_invalidates_cgu_cache() {
    let test_dir = unique_test_dir("e2e_tracked");
    let runner_path = create_schema_macro_script(&test_dir);
    let cache_dir = test_dir.join("cgu_cache");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");
    let schema_file = test_dir.join("schema.txt");

    fs::write(&schema_file, "version_1").expect("write schema v1");

    let main_src = r#"
#[macro::schema_macro]
fun placeholder() -> int32 {
    return 1
}

fun main() -> int32 {
    set s = get_schema_data()
    return 0
}
"#;
    fs::write(&main_file, main_src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());
    config.cache_dir = Some(cache_dir.clone());
    config.profile_name = Some("debug".to_string());
    config.macro_runners.insert("schema_macro".to_string(), runner_path.clone());

    // 1st compile: fresh compilation
    let res1 = run_compiler_driver(&config);
    assert!(res1.is_ok(), "first compilation should succeed: {res1:?}");
    assert!(out_file.is_file(), "output object should exist");

    let manifest_path = cache_dir.join("cgu_manifest.json");
    assert!(manifest_path.is_file(), "cgu manifest must exist");
    let manifest1_content = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest1: serde_json::Value = serde_json::from_str(&manifest1_content).expect("parse manifest");
    let cgu_entries = manifest1.get("entries").and_then(|e| e.as_object()).expect("cgu entries");
    assert!(!cgu_entries.is_empty(), "expected at least one cgu entry in manifest");
    let first_cgu_key = cgu_entries.keys().next().unwrap().clone();
    let initial_hash = cgu_entries.get(&first_cgu_key).unwrap().get("hash").unwrap().as_str().unwrap().to_string();

    // 2nd compile without changes: must retain the exact same hash
    let res2 = run_compiler_driver(&config);
    assert!(res2.is_ok(), "second compilation should succeed: {res2:?}");
    let manifest2_content = fs::read_to_string(&manifest_path).expect("read manifest 2");
    let manifest2: serde_json::Value = serde_json::from_str(&manifest2_content).expect("parse manifest 2");
    let hash_after_recompile = manifest2["entries"][&first_cgu_key]["hash"].as_str().unwrap();
    assert_eq!(initial_hash, hash_after_recompile, "cache hash must be identical when inputs did not change");

    // 3rd compile: modify the external tracked file
    fs::write(&schema_file, "version_2_changed_data").expect("write schema v2");
    let res3 = run_compiler_driver(&config);
    assert!(res3.is_ok(), "third compilation should succeed after file change: {res3:?}");
    let manifest3_content = fs::read_to_string(&manifest_path).expect("read manifest 3");
    let manifest3: serde_json::Value = serde_json::from_str(&manifest3_content).expect("parse manifest 3");
    let hash_after_file_change = manifest3["entries"][&first_cgu_key]["hash"].as_str().unwrap();
    assert_ne!(
        initial_hash, hash_after_file_change,
        "modifying tracked file must invalidate CGU cache and change the entry hash"
    );

    // 4th compile: modify macro runner binary content
    let mut updated_script = fs::read_to_string(&runner_path).expect("read runner");
    updated_script.push_str("\n# runner version 2 comment\n");
    fs::write(&runner_path, updated_script).expect("write updated runner");
    let f = fs::File::open(&runner_path).expect("open to sync");
    let _ = f.sync_all();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let res4 = run_compiler_driver(&config);
    assert!(res4.is_ok(), "fourth compilation should succeed: {res4:?}");
    let manifest4_content = fs::read_to_string(&manifest_path).expect("read manifest 4");
    let manifest4: serde_json::Value = serde_json::from_str(&manifest4_content).expect("parse manifest 4");
    let hash_after_runner_change = manifest4["entries"][&first_cgu_key]["hash"].as_str().unwrap();
    assert_ne!(
        hash_after_file_change, hash_after_runner_change,
        "modifying macro runner binary must invalidate CGU cache and change the entry hash"
    );

    let _ = fs::remove_dir_all(&test_dir);
}
