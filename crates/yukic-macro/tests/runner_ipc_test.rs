use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use yukic_ast::*;
use yukic_lexer::Lexer;
use yukic_macro::runner::MacroRunnerClient;
use yukic_macro::{BuiltinMacroConfig, MacroExpander};
use yukic_parser::Parser;
use yukic_span::{BytePos, FileId, Span};
use yukino_macroproto::{MacroKind, MacroRequest};

fn create_mock_runner(filename: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("macro_mock_{}_{}", std::process::id(), count));
    let _ = fs::create_dir_all(&dir);
    let script_path = dir.join(filename);

    let python_code = r#"#!/usr/bin/env python3
import sys, struct, json

raw_len = sys.stdin.buffer.read(4)
if not raw_len or len(raw_len) < 4:
    sys.exit(0)
length = struct.unpack('<I', raw_len)[0]
payload = sys.stdin.buffer.read(length)
req = json.loads(payload.decode('utf-8'))

macro_name = req.get("macro_name", "")

if macro_name == "crash":
    sys.stderr.write("Fatal error in procedural macro runner")
    sys.exit(101)

if macro_name == "hang":
    import time
    time.sleep(10)
    sys.exit(0)

if macro_name == "stmt_macro":
    resp = {
        "success": True,
        "output_tokens": "rewrite x = 100",
        "tracked_files": [],
        "diagnostics": [],
        "error_message": None
    }
elif macro_name == "logged":
    resp = {
        "success": True,
        "output_tokens": "public fun traced_fn() -> int32 { return 777 }",
        "tracked_files": [],
        "diagnostics": [],
        "error_message": None
    }
elif macro_name == "ToString":
    resp = {
        "success": True,
        "output_tokens": "public fun user_to_string(u: User) -> string { return \"user\" }",
        "tracked_files": ["user_model.tracked"],
        "diagnostics": [],
        "error_message": None
    }
else:
    resp = {
        "success": True,
        "output_tokens": "42 * 2",
        "tracked_files": ["tracked_extra.ykn"],
        "diagnostics": [],
        "error_message": None
    }

resp_bytes = json.dumps(resp).encode('utf-8')
sys.stdout.buffer.write(struct.pack('<I', len(resp_bytes)))
sys.stdout.buffer.write(resp_bytes)
sys.stdout.buffer.flush()
"#;

    {
        use std::io::Write;
        let mut f = fs::File::create(&script_path).expect("create file");
        f.write_all(python_code.as_bytes()).expect("write mock runner");
        f.sync_all().expect("sync file");
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    script_path
}

fn parse_code(source: &str, file_id: FileId) -> Module {
    let lexer = Lexer::new(file_id, source);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, _) = parser.parse_module();
    module
}

fn dummy_span() -> Span {
    Span::new(FileId(0), BytePos(0), BytePos(10))
}

#[test]
fn test_macro_runner_ipc_expression_expansion() {
    let runner_path = create_mock_runner("runner_expr.py");
    let mut macro_runners = HashMap::new();
    macro_runners.insert("my_proc_macro".to_string(), runner_path.clone());

    let code = r#"
fun test_fn() {
    set res = my_proc_macro!(5)
}
"#;
    let mut module = parse_code(code, FileId(0));
    let config = BuiltinMacroConfig {
        macro_runners,
        ..Default::default()
    };
    let expander = MacroExpander::with_config(config);

    let (result, tracked) = expander.expand_module_tracked(&mut module, None);
    assert!(result.is_ok(), "expand_module_tracked failed: {:?}", result.err());
    assert!(tracked.contains(&"tracked_extra.ykn".to_string()));

    if let ItemKind::Fun(f) = &module.items[0].kind {
        if let StmtKind::Set { init, .. } = &f.body.stmts[0].kind {
            if let ExprKind::Binary(BinaryOp::Mul, lhs, rhs) = &init.kind {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(42)));
                assert_eq!(rhs.kind, ExprKind::Literal(LiteralValue::Int(2)));
            } else {
                panic!("expected binary multiplication expression");
            }
        } else {
            panic!("expected set statement");
        }
    } else {
        panic!("expected fun item");
    }

    let _ = fs::remove_file(&runner_path);
}

#[test]
fn test_macro_runner_ipc_statement_expansion() {
    let runner_path = create_mock_runner("runner_stmt.py");
    let mut macro_runners = HashMap::new();
    macro_runners.insert("stmt_macro".to_string(), runner_path.clone());

    let code = r#"
fun test_fn() {
    stmt_macro!(x)
}
"#;
    let mut module = parse_code(code, FileId(0));
    let config = BuiltinMacroConfig {
        macro_runners,
        ..Default::default()
    };
    let expander = MacroExpander::with_config(config);

    let (result, _) = expander.expand_module_tracked(&mut module, None);
    assert!(result.is_ok(), "expand_module_tracked failed: {:?}", result.err());

    if let ItemKind::Fun(f) = &module.items[0].kind {
        assert_eq!(f.body.stmts.len(), 1);
        if let StmtKind::Rewrite { value, .. } = &f.body.stmts[0].kind {
            assert_eq!(value.kind, ExprKind::Literal(LiteralValue::Int(100)));
        } else {
            panic!("expected rewrite statement from macro expansion");
        }
    } else {
        panic!("expected fun item");
    }

    let _ = fs::remove_file(&runner_path);
}

#[test]
fn test_macro_runner_client_timeout() {
    let runner_path = create_mock_runner("runner_hang.py");
    let client = MacroRunnerClient::with_timeout(Duration::from_millis(50));

    let req = MacroRequest::new(
        "hang",
        MacroKind::FunctionLike,
        "()",
        None,
        "x86_64-unknown-linux-gnu",
        "debug",
        None,
    );

    let result = client.execute(&runner_path, &req, dummy_span());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("timed out"));

    let _ = fs::remove_file(&runner_path);
}

#[test]
fn test_macro_runner_client_crash_captures_stderr() {
    let runner_path = create_mock_runner("runner_crash.py");
    let client = MacroRunnerClient::new();

    let req = MacroRequest::new(
        "crash",
        MacroKind::FunctionLike,
        "()",
        None,
        "x86_64-unknown-linux-gnu",
        "debug",
        None,
    );

    let result = client.execute(&runner_path, &req, dummy_span());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Fatal error in procedural macro runner"),
        "error message did not contain stderr: {}",
        err.message
    );

    let _ = fs::remove_file(&runner_path);
}

#[test]
fn test_macro_runner_ipc_attribute_expansion() {
    let runner_path = create_mock_runner("runner_attr.py");
    let mut macro_runners = HashMap::new();
    macro_runners.insert("logged".to_string(), runner_path.clone());

    let code = r#"
#[logged]
fun original_fn() {
    println("old")
}
"#;
    let mut module = parse_code(code, FileId(0));
    let config = BuiltinMacroConfig {
        macro_runners,
        ..Default::default()
    };
    let expander = MacroExpander::with_config(config);

    let (result, _) = expander.expand_module_tracked(&mut module, None);
    assert!(result.is_ok(), "expand_module_tracked failed: {:?}", result.err());

    assert_eq!(module.items.len(), 1);
    if let ItemKind::Fun(f) = &module.items[0].kind {
        assert_eq!(f.name.name, "traced_fn");
        assert_eq!(f.body.stmts.len(), 1);
        if let StmtKind::Return(Some(expr)) = &f.body.stmts[0].kind {
            assert_eq!(expr.kind, ExprKind::Literal(LiteralValue::Int(777)));
        } else {
            panic!("expected return statement in expanded function");
        }
    } else {
        panic!("expected fun item");
    }

    let _ = fs::remove_file(&runner_path);
}

#[test]
fn test_macro_runner_ipc_mutation_expansion() {
    let runner_path = create_mock_runner("runner_mut.py");
    let mut macro_runners = HashMap::new();
    macro_runners.insert("ToString".to_string(), runner_path.clone());

    let code = r#"
#[mutation(ToString)]
public struct User {
    name: string
}
"#;
    let mut module = parse_code(code, FileId(0));
    let config = BuiltinMacroConfig {
        macro_runners,
        ..Default::default()
    };
    let expander = MacroExpander::with_config(config);

    let (result, tracked) = expander.expand_module_tracked(&mut module, None);
    assert!(result.is_ok(), "expand_module_tracked failed: {:?}", result.err());
    assert!(tracked.contains(&"user_model.tracked".to_string()));

    assert_eq!(module.items.len(), 2);
    if let ItemKind::Struct(s) = &module.items[0].kind {
        assert_eq!(s.name.name, "User");
        assert!(module.items[0].attrs.iter().all(|a| !a.is("mutation")));
    } else {
        panic!("expected struct item first");
    }

    if let ItemKind::Fun(f) = &module.items[1].kind {
        assert_eq!(f.name.name, "user_to_string");
    } else {
        panic!("expected generated companion function second");
    }

    let _ = fs::remove_file(&runner_path);
}
