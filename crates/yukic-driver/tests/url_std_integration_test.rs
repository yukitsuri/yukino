use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-url-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_url_std_execution() {
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("url_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"url_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    let yukino_code = r#"
load std: *

fun fail(code: int32, msg: string) {
    println("FAILED: " + msg)
    exit(code)
}

fun keep_lang(k: string, v: string) -> bool {
    return k == "lang"
}

fun main() {
    // 1. Encoding and Decoding
    set raw = "hello world & foo=bar/baz"
    set enc = url_encode_component(raw.cp())
    if enc != "hello%20world%20%26%20foo%3dbar%2fbaz" && enc != "hello%20world%20%26%20foo%3Dbar%2Fbaz" {
        fail(1, "url_encode_component")
    }
    set dec = url_decode_component(enc)
    if dec != raw { fail(2, "url_decode_component") }

    // 2. URL parsing RFC 3986
    set raw_url = "http://alice:secret@example.com:8080/api/v1/users?page=2&limit=50#section-3"
    set u = url_parse(raw_url)
    if u.scheme != "http" { fail(3, "scheme") }
    if u.username != "alice" { fail(4, "username") }
    if u.password != "secret" { fail(5, "password") }
    if u.host != "example.com" { fail(6, "host") }
    if u.effective_port() != 8080 { fail(7, "port") }
    if u.path != "/api/v1/users" { fail(8, "path") }
    if u.fragment != "section-3" { fail(9, "fragment") }
    if u.origin() != "http://example.com:8080" { fail(10, "origin") }

    // 3. Query params inspection
    if u.get_query_param("page") != "2" { fail(11, "query param page") }
    if u.get_query_param("limit") != "50" { fail(12, "query param limit") }
    if u.get_query_param("unknown") != null { fail(13, "unknown query param is null") }

    // 4. Query params mutation
    u.set_query_param("page", "3")
    if u.get_query_param("page") != "3" { fail(14, "query param mutated") }
    u.remove_query_param("limit")
    if u.get_query_param("limit") != null { fail(15, "query param removed") }
    u.add_query_param("lang", "yukino")
    if u.get_query_param("lang") != "yukino" { fail(16, "query param added") }

    // 5. Query filtering with predicate
    u.filter_query(keep_lang)
    if u.get_query_param("page") != null { fail(17, "page filtered out") }
    if u.get_query_param("lang") != "yukino" { fail(18, "lang retained") }

    // 6. Path joining
    set base_url = url_parse("https://example.org/docs/intro.html")
    set joined = base_url.join("quickstart.html")
    if joined.path != "/docs/quickstart.html" { fail(19, "relative join") }
    set abs_joined = base_url.join("/index.html")
    if abs_joined.path != "/index.html" { fail(20, "absolute join") }

    // 7. UrlBuilder
    set builder = url_builder()
    builder.scheme("https")
    builder.host("yukino-lang.org")
    builder.path("/reference")
    builder.set_query_param("tab", "api")
    builder.fragment("top")
    set built = builder.build()
    if built.scheme != "https" { fail(21, "builder scheme") }
    if built.host != "yukino-lang.org" { fail(22, "builder host") }
    if built.path != "/reference" { fail(23, "builder path") }
    if built.get_query_param("tab") != "api" { fail(24, "builder query") }
    if built.fragment != "top" { fail(25, "builder fragment") }
    if built.to_string() != "https://yukino-lang.org/reference?tab=api#top" { fail(26, "builder to_string") }

    println("URL TESTS PASSED")
}
"#;

    fs::write(&input, yukino_code).expect("write main.ykn");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    let compile_res = run_compiler_driver(&config);
    if let Err(e) = &compile_res {
        eprintln!("DRIVER ERROR: {e:?}");
    }
    compile_res.expect("driver compilation for url app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let exec_res = std::process::Command::new(&app_out)
        .current_dir(&dir)
        .output()
        .expect("execute compiled url test app");

    if !exec_res.status.success() {
        let stdout = String::from_utf8_lossy(&exec_res.stdout);
        let stderr = String::from_utf8_lossy(&exec_res.stderr);
        panic!("Execution failed with status: {:?}\nstdout: {stdout}\nstderr: {stderr}", exec_res.status);
    }
}
