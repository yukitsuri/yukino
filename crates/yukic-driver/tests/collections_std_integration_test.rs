use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-collections-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_collections_std_modules_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"coll_std_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.collections: *

fun double_num(x: int64) -> int64 {
    return x * 2
}

fun is_positive(x: int64) -> bool {
    return x > 0
}

fun add_nums(a: int64, b: int64) -> int64 {
    return a + b
}

fun cmp_asc(a: int64, b: int64) -> int32 {
    return cast(a - b, "int32")
}

fun test_search_and_transform() {
    set arr = [10, 20, 30]
    set has_20 = acontains(ref arr, 20)
    set found = afind(ref arr, is_positive)

    set doubled = amap(ref arr, double_num)
    set pos = afilter(ref arr, is_positive)
    set sum = areduce(ref arr, 0, add_nums)
}

fun test_mut() {
    set arr = [3, 1, 2]
    asort(ref ~arr)
    areverse(ref ~arr)
    afill(ref ~arr, 42)
}

fun test_strings() {
    set words = ["hello", "world"]
    set joined = ajoin(ref words, ", ")
}

fun test_containers() {
    set list = init List<int64>()
    list.push(100)
    list.push(200)
    set l_len = list.len()
    set val = list.get(0)
    list.set_at(0, 150)
    set popped = list.pop()

    set map = init HashMap<string, string>()
    map.set("key1", "val1")
    set v1 = map.get("key1")
    set has_k = map.contains_key("key1")
    set del_ok = map.remove("key1")

    set hset = init HashSet<string>()
    hset.add("item1")
    set has_item = hset.contains("item1")
    hset.remove("item1")

    set deque = init Deque<int64>()
    deque.push_back(1)
    deque.push_front(2)
    set d_pop = deque.pop_front()

    set queue = init Queue<int64>()
    queue.push(10)
    set q_peek = queue.peek()
    set q_pop = queue.pop()
}

fun main() {
    test_search_and_transform()
    test_mut()
    test_strings()
    test_containers()
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
    assert!(ir.contains("_Y_HashMap_init"), "ir must contain _Y_HashMap_init");
    assert!(ir.contains("_Y_HashSet_init"), "ir must contain _Y_HashSet_init");
    assert!(ir.contains("_Y_Deque_init"), "ir must contain _Y_Deque_init");
    assert!(ir.contains("_Y_Queue_init"), "ir must contain _Y_Queue_init");
    assert!(ir.contains("ajoin"), "ir must contain ajoin");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_collections_std_binary_execution() {
    let dir = std::env::temp_dir().join(format!("yukic-collections-std-exec-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create test dir");
    let input = dir.join("main.ykn");
    let app_out = dir.join("coll_exec_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"coll_exec_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.collections: *

fun double_num(x: int64) -> int64 {
    return x * 2
}

fun is_positive(x: int64) -> bool {
    return x > 0
}

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

fun add_nums(a: int64, b: int64) -> int64 {
    return a + b
}

fun main() {
    set arr = [10, 20, 30]
    if !acontains(ref arr, 20) { fail(1, "acontains 20") }
    if acontains(ref arr, 99) { fail(2, "acontains 99") }
    if aindex_of(ref arr, 30) != 2 { fail(3, "aindex_of 30") }

    set doubled = amap(ref arr, double_num)
    if doubled.len() != 3 || doubled[0] != 20 || doubled[2] != 60 { fail(4, "amap") }
    set pos = afilter(ref arr, is_positive)
    if pos.len() != 3 { fail(21, "afilter") }
    set f_elem = afind(ref arr, is_positive)
    if f_elem != 10 { fail(22, "afind") }
    set sum = areduce(ref arr, 0, add_nums)
    if sum != 60 { fail(5, "areduce") }

    set m_arr: int64[] = [3, 1, 2]
    asort(ref ~m_arr)
    if m_arr[0] != 1 || m_arr[1] != 2 || m_arr[2] != 3 { fail(6, "asort") }
    areverse(ref ~m_arr)
    if m_arr[0] != 3 || m_arr[2] != 1 { fail(7, "areverse") }
    afill(ref ~m_arr, 42)
    if m_arr[0] != 42 || m_arr[1] != 42 || m_arr[2] != 42 { fail(23, "afill") }

    set s_arr = ["banana", "apple", "cherry"]
    asort_str(ref ~s_arr)
    if s_arr[0] != "apple" || s_arr[1] != "banana" || s_arr[2] != "cherry" { fail(24, "asort_str") }
    areverse_str(ref ~s_arr)
    if s_arr[0] != "cherry" || s_arr[2] != "apple" { fail(25, "areverse_str") }
    if !acontains_str(ref s_arr, "banana") { fail(26, "acontains_str") }
    if aindex_of_str(ref s_arr, "cherry") != 0 { fail(27, "aindex_of_str") }
    afill_str(ref ~s_arr, "x")
    if s_arr[0] != "x" || s_arr[2] != "x" { fail(28, "afill_str") }

    set words = ["hello", "world"]
    set joined = ajoin(ref words, ", ")
    if joined != "hello, world" { fail(8, "ajoin") }

    set list = init List<int64>()
    list.push(100)
    list.push(200)
    if list.len() != 2 || list.get(0) != 100 { fail(9, "list push/get") }
    set popped = list.pop()
    if popped != 200 || list.len() != 1 { fail(10, "list pop") }

    set map = init HashMap<string, string>()
    map.set("key1", "val1")
    if !map.contains_key("key1") { fail(11, "map contains_key") }
    set v1 = map.get("key1")
    if v1 != "val1" { fail(12, "map get") }
    if !map.remove("key1") { fail(13, "map remove") }
    if map.contains_key("key1") { fail(14, "map contains_key after remove") }

    set hset = init HashSet<string>()
    hset.add("item1")
    if !hset.contains("item1") { fail(15, "hset add") }
    hset.remove("item1")
    if hset.contains("item1") { fail(16, "hset remove") }

    set deque = init Deque<int64>()
    deque.push_back(10)
    deque.push_front(20)
    set front = deque.pop_front()
    if front != 20 { fail(17, "deque pop_front") }
    set back = deque.pop_back()
    if back != 10 { fail(18, "deque pop_back") }

    set queue = init Queue<int64>()
    queue.push(50)
    queue.push(60)
    if queue.peek() != 50 { fail(19, "queue peek") }
    set q1 = queue.pop()
    if q1 != 50 { fail(20, "queue pop") }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("build executable");
    assert!(app_out.exists());

    let status = std::process::Command::new(&app_out)
        .status()
        .expect("run executable");
    assert!(status.success(), "binary execution must succeed");

    let _ = fs::remove_dir_all(dir);
}
