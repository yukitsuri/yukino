use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-sync-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_sync_std_execution() {
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("sync_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sync_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.sync: *
load std.io: println

struct TestError {
    code: int32
    message: string
}

fun fail(code: int32, msg: string) {
    panic(TestError { code: code, message: msg })
}

fun main() {
    // 1. Mutex
    set m = mutex()
    m.lock()
    if m.try_lock() {
        fail(1, "mutex already locked should not succeed try_lock")
    }
    m.unlock()
    if !m.try_lock() {
        fail(2, "try_lock should succeed on unlocked mutex")
    }
    m.unlock()
    m.destroy()

    // 2. RwLock
    set rw = rwlock()
    rw.read()
    if !rw.try_read() {
        fail(3, "multiple readers should be allowed")
    }
    rw.unlock_read()
    if rw.try_write() {
        fail(4, "writer should not enter while reader active")
    }
    rw.unlock_read()
    if !rw.try_write() {
        fail(5, "writer should succeed when no readers")
    }
    rw.unlock_write()
    rw.destroy()

    // 3. AtomicInt64
    set a_i64 = atomic_i64(10)
    if a_i64.get() != 10 {
        fail(6, "initial atomic value mismatch")
    }
    set prev = a_i64.fetch_add(5)
    if prev != 10 || a_i64.get() != 15 {
        fail(7, "fetch_add mismatch")
    }
    set prev_sub = a_i64.fetch_sub(3)
    if prev_sub != 15 || a_i64.get() != 12 {
        fail(8, "fetch_sub mismatch")
    }
    set old_swap = a_i64.swap(100)
    if old_swap != 12 || a_i64.get() != 100 {
        fail(9, "swap mismatch")
    }
    if !a_i64.compare_exchange(100, 200) || a_i64.get() != 200 {
        fail(10, "compare_exchange success mismatch")
    }
    if a_i64.compare_exchange(999, 300) {
        fail(11, "compare_exchange should fail with wrong expected")
    }
    a_i64.destroy()

    // 4. AtomicBool
    set a_bool = atomic_bool(false)
    if a_bool.get() {
        fail(12, "initial bool should be false")
    }
    a_bool.set(true)
    if !a_bool.get() {
        fail(13, "bool set true failed")
    }
    set old_b = a_bool.swap(false)
    if !old_b || a_bool.get() {
        fail(14, "bool swap mismatch")
    }
    if !a_bool.compare_exchange(false, true) || !a_bool.get() {
        fail(15, "bool cas mismatch")
    }
    a_bool.destroy()

    // 5. Channel
    set ch = channel(5)
    if ch.len() != 0 || ch.is_closed() {
        fail(16, "channel initial state mismatch")
    }
    if !ch.send_i64(42) || !ch.send_i64(108) {
        fail(17, "channel send failed")
    }
    if ch.len() != 2 {
        fail(18, "channel len should be 2")
    }
    set item1 = ch.try_recv_i64()
    if item1 == null || (item1 ?? 0) != 42 {
        fail(19, "channel first item mismatch")
    }
    set item2 = ch.recv_i64()
    if item2 == null || (item2 ?? 0) != 108 {
        fail(20, "channel second item mismatch")
    }
    set empty_opt = ch.try_recv_i64()
    if empty_opt != null {
        fail(21, "channel should be empty")
    }
    ch.close()
    if !ch.is_closed() {
        fail(22, "channel is_closed mismatch")
    }
    ch.destroy()

    // 6. Semaphore
    set sem = semaphore(2)
    if !sem.acquire() {
        fail(23, "first permit should be acquired")
    }
    if !sem.try_acquire() {
        fail(24, "second permit should be acquired")
    }
    if sem.try_acquire() {
        fail(25, "third permit should fail")
    }
    set _ = sem.release()
    if !sem.try_acquire() {
        fail(26, "permit should be available after release")
    }
    set _ = sem.release()
    set _ = sem.release()
    sem.destroy()

    // 7. Barrier
    set bar = barrier(1u32)
    set is_lead = bar.wait()
    if !is_lead {
        fail(27, "single party barrier should be leader")
    }
    bar.destroy()

    // 8. Once
    set on = once()
    if on.is_completed() {
        fail(28, "once initially not completed")
    }
    on.call_once(null, null)
    if !on.is_completed() {
        fail(29, "once should be completed after call")
    }
    on.destroy()

    // 9. CondVar
    set cv = condvar()
    set cv_m = mutex()
    cv_m.lock()
    set timed_out = !cv.wait_timeout(ref cv_m, 10u64)
    cv_m.unlock()
    if !timed_out {
        fail(30, "condvar wait_timeout should time out when not notified")
    }
    cv.notify_one()
    cv.notify_all()
    cv.destroy()
    cv_m.destroy()

    println("Sync tests passed successfully!")
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for sync app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let run_res = std::process::Command::new(&app_out)
        .output()
        .expect("run sync app");
    assert!(
        run_res.status.success(),
        "app execution failed: status={:?}, stdout={}, stderr={}",
        run_res.status,
        String::from_utf8_lossy(&run_res.stdout),
        String::from_utf8_lossy(&run_res.stderr)
    );

    let stdout = String::from_utf8_lossy(&run_res.stdout);
    assert!(
        stdout.contains("Sync tests passed successfully!"),
        "unexpected output: {stdout}"
    );

    let _ = fs::remove_dir_all(&dir);
}
