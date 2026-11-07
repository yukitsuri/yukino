#![allow(
    unsafe_code,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions
)]

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[repr(C)]
pub struct YukinoTestDesc {
    pub name_ptr: *const u8,
    pub name_len: usize,
    pub test_fn: unsafe extern "C" fn(),
    pub should_panic: bool,
    pub expected_panic_ptr: *const u8,
    pub expected_panic_len: usize,
    pub ignore: bool,
    pub ignore_reason_ptr: *const u8,
    pub ignore_reason_len: usize,
    pub timeout_ms: u64,
}

unsafe impl Sync for YukinoTestDesc {}
unsafe impl Send for YukinoTestDesc {}

impl YukinoTestDesc {
    pub fn name(&self) -> &str {
        if self.name_ptr.is_null() || self.name_len == 0 {
            return "<unknown>";
        }
        unsafe {
            let slice = std::slice::from_raw_parts(self.name_ptr, self.name_len);
            std::str::from_utf8(slice).unwrap_or("<invalid_utf8>")
        }
    }

    pub fn expected_panic(&self) -> Option<&str> {
        if self.expected_panic_ptr.is_null() || self.expected_panic_len == 0 {
            return None;
        }
        unsafe {
            let slice = std::slice::from_raw_parts(self.expected_panic_ptr, self.expected_panic_len);
            std::str::from_utf8(slice).ok()
        }
    }

    pub fn ignore_reason(&self) -> Option<&str> {
        if self.ignore_reason_ptr.is_null() || self.ignore_reason_len == 0 {
            return None;
        }
        unsafe {
            let slice = std::slice::from_raw_parts(self.ignore_reason_ptr, self.ignore_reason_len);
            std::str::from_utf8(slice).ok()
        }
    }

    pub fn suite_name(&self) -> &str {
        let name = self.name();
        if let Some(pos) = name.find("::") {
            &name[..pos]
        } else {
            "main"
        }
    }
}

#[derive(Default, Debug)]
struct TestCliArgs {
    filter: Option<String>,
    exact: bool,
    nocapture: bool,
    threads: Option<usize>,
    timeout_ms: Option<u64>,
    ignored: bool,
    include_ignored: bool,
    fail_fast: bool,
    list: bool,
    quiet: bool,
    internal_run_test: Option<String>,
}

fn parse_test_args(argc: i32, argv: *const *const u8) -> TestCliArgs {
    let mut args = TestCliArgs::default();
    if argc <= 1 || argv.is_null() {
        return args;
    }

    let mut i = 1;
    while i < argc as usize {
        let arg_ptr = unsafe { *argv.add(i) };
        if arg_ptr.is_null() {
            i += 1;
            continue;
        }
        let c_str = unsafe { std::ffi::CStr::from_ptr(arg_ptr.cast()) };
        let arg = c_str.to_string_lossy();

        if arg == "--exact" {
            args.exact = true;
        } else if arg == "--nocapture" {
            args.nocapture = true;
        } else if arg == "--ignored" {
            args.ignored = true;
        } else if arg == "--include-ignored" {
            args.include_ignored = true;
        } else if arg == "--fail-fast" {
            args.fail_fast = true;
        } else if arg == "--list" {
            args.list = true;
        } else if arg == "--quiet" || arg == "-q" {
            args.quiet = true;
        } else if arg == "--internal-run-test" && i + 1 < argc as usize {
            i += 1;
            let next_ptr = unsafe { *argv.add(i) };
            if !next_ptr.is_null() {
                let next_str = unsafe { std::ffi::CStr::from_ptr(next_ptr.cast()) };
                args.internal_run_test = Some(next_str.to_string_lossy().to_string());
            }
        } else if (arg == "--threads" || arg == "-j") && i + 1 < argc as usize {
            i += 1;
            let next_ptr = unsafe { *argv.add(i) };
            if !next_ptr.is_null() {
                let next_str = unsafe { std::ffi::CStr::from_ptr(next_ptr.cast()) };
                if let Ok(th) = next_str.to_string_lossy().parse::<usize>() {
                    args.threads = Some(th);
                }
            }
        } else if arg.starts_with("--threads=") || arg.starts_with("-j=") {
            if let Some(val) = arg.split('=').nth(1) {
                if let Ok(th) = val.parse::<usize>() {
                    args.threads = Some(th);
                }
            }
        } else if arg == "--timeout" && i + 1 < argc as usize {
            i += 1;
            let next_ptr = unsafe { *argv.add(i) };
            if !next_ptr.is_null() {
                let next_str = unsafe { std::ffi::CStr::from_ptr(next_ptr.cast()) };
                if let Ok(to) = next_str.to_string_lossy().parse::<u64>() {
                    args.timeout_ms = Some(to);
                }
            }
        } else if arg.starts_with("--timeout=") {
            if let Some(val) = arg.split('=').nth(1) {
                if let Ok(to) = val.parse::<u64>() {
                    args.timeout_ms = Some(to);
                }
            }
        } else if !arg.starts_with('-') && args.filter.is_none() {
            args.filter = Some(arg.to_string());
        }
        i += 1;
    }

    args
}

#[derive(Clone, Debug)]
enum TestOutcome {
    Passed,
    Failed { details: String },
    Ignored {
        #[allow(dead_code)]
        reason: Option<String>,
    },
}

#[derive(Clone, Debug)]
struct TestExecutionResult {
    name: String,
    suite: String,
    outcome: TestOutcome,
    duration: Duration,
}

struct TestPalette {
    use_colors: bool,
}

impl TestPalette {
    fn new() -> Self {
        let is_term = std::io::stdout().is_terminal();
        let no_color = std::env::var_os("NO_COLOR").is_some();
        Self {
            use_colors: is_term && !no_color,
        }
    }

    fn reset(&self) -> &'static str {
        if self.use_colors { "\x1b[0m" } else { "" }
    }

    fn bold(&self) -> &'static str {
        if self.use_colors { "\x1b[1m" } else { "" }
    }

    fn cyan_scope(&self) -> &'static str {
        if self.use_colors { "\x1b[36m" } else { "" }
    }

    fn cyan(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;120;220;255m" } else { "" }
    }

    fn white(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;240;240;240m" } else { "" }
    }

    fn gray(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;140;150;160m" } else { "" }
    }

    fn dark_gray(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;90;105;120m" } else { "" }
    }

    fn green(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;80;220;120m" } else { "" }
    }

    fn red(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;255;95;110m" } else { "" }
    }

    fn yellow(&self) -> &'static str {
        if self.use_colors { "\x1b[38;2;255;210;90m" } else { "" }
    }

    fn format_scoped_name(&self, name: &str) -> String {
        let parts: Vec<&str> = name.split("::").collect();
        let sep = format!("{}::{}", self.cyan_scope(), self.white());
        format!("{}{}{}", self.white(), parts.join(&sep), self.reset())
    }
}

fn execute_single_worker(test: &YukinoTestDesc) -> ! {
    unsafe {
        (test.test_fn)();
    }
    std::process::exit(0);
}

fn run_worker_process(
    self_exe: &std::path::Path,
    test: &YukinoTestDesc,
    default_timeout_ms: u64,
    nocapture: bool,
) -> TestOutcome {
    let mut cmd = Command::new(self_exe);
    cmd.arg("--internal-run-test").arg(test.name());

    if nocapture {
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return TestOutcome::Failed {
                details: format!("failed to spawn test worker: {e}"),
            };
        }
    };

    let timeout_ms = if test.timeout_ms > 0 {
        test.timeout_ms
    } else if default_timeout_ms > 0 {
        default_timeout_ms
    } else {
        60_000
    };

    let timeout = Duration::from_millis(timeout_ms);
    let start_time = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = if nocapture {
                    None
                } else {
                    child.wait_with_output().ok()
                };

                let stderr_str = output.as_ref().map_or_else(String::new, |o| {
                    String::from_utf8_lossy(&o.stderr).to_string()
                });
                let stdout_str = output.as_ref().map_or_else(String::new, |o| {
                    String::from_utf8_lossy(&o.stdout).to_string()
                });

                if test.should_panic {
                    if status.success() {
                        return TestOutcome::Failed {
                            details: "test did not panic, expected panic".to_string(),
                        };
                    }
                    if let Some(expected) = test.expected_panic() {
                        if !stderr_str.contains(expected) {
                            return TestOutcome::Failed {
                                details: format!(
                                    "panic did not contain expected substring '{expected}'\n  captured stderr: {stderr_str}"
                                ),
                            };
                        }
                    }
                    return TestOutcome::Passed;
                }

                if status.success() {
                    return TestOutcome::Passed;
                }

                let mut details = String::new();
                if !stdout_str.trim().is_empty() {
                    details.push_str("captured stdout:\n");
                    details.push_str(&stdout_str);
                    if !details.ends_with('\n') {
                        details.push('\n');
                    }
                }
                if !stderr_str.trim().is_empty() {
                    details.push_str("captured stderr:\n");
                    details.push_str(&stderr_str);
                } else {
                    details.push_str(&format!("test process exited with {status}"));
                }
                return TestOutcome::Failed { details };
            }
            Ok(None) => {
                if start_time.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return TestOutcome::Failed {
                        details: format!("test timed out after {timeout_ms}ms"),
                    };
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => {
                return TestOutcome::Failed {
                    details: format!("error waiting for test worker: {e}"),
                };
            }
        }
    }
}

fn format_live_line(
    pal: &TestPalette,
    spinner: char,
    name: &str,
    cur: usize,
    total: usize,
    elapsed: Duration,
) -> String {
    let bar_width = 10;
    let filled = if total > 0 {
        (bar_width * cur) / total
    } else {
        0
    };
    let empty = bar_width.saturating_sub(filled);

    let hashes = "#".repeat(filled);
    let spaces = " ".repeat(empty);

    let bar_part = format!(
        "{}[{}{}{}{}]",
        pal.dark_gray(),
        pal.cyan(),
        hashes,
        spaces,
        pal.dark_gray()
    );

    let count_part = format!(
        "{}[{}{}/{}{}]",
        pal.dark_gray(),
        pal.white(),
        cur,
        total,
        pal.dark_gray()
    );

    let elapsed_sec = elapsed.as_secs_f64();
    let time_part = format!("{}{:>4.1}s{}", pal.gray(), elapsed_sec, pal.reset());
    let spin_part = format!("{}{}{}", pal.cyan(), spinner, pal.reset());

    let base_col_width = 26;
    let raw_len = name.len();
    let gap = if raw_len >= base_col_width {
        2
    } else {
        base_col_width - raw_len + 2
    };
    let spacing = " ".repeat(gap);

    let styled_name = pal.format_scoped_name(name);
    let right_part = format!("{bar_part} {count_part} {time_part}");

    format!(
        "\r\x1b[2K {} {}{}{}",
        spin_part,
        styled_name,
        spacing,
        right_part
    )
}

#[derive(Default)]
struct SuiteSummary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    time_ms: u64,
}

fn render_final_table(
    pal: &TestPalette,
    results: &[TestExecutionResult],
    total_elapsed: Duration,
) {
    let mut suites: BTreeMap<String, SuiteSummary> = BTreeMap::new();
    let mut failures: Vec<(&str, &str)> = Vec::new();

    for r in results {
        let entry = suites.entry(r.suite.clone()).or_default();
        entry.total += 1;
        entry.time_ms += r.duration.as_millis() as u64;

        match &r.outcome {
            TestOutcome::Passed => entry.passed += 1,
            TestOutcome::Failed { details } => {
                entry.failed += 1;
                failures.push((r.name.as_str(), details.as_str()));
            }
            TestOutcome::Ignored { .. } => entry.skipped += 1,
        }
    }

    println!(
        "\n{}{:<14} {:<9} {:<10} {:<8}{}",
        pal.dark_gray(),
        "SUITE",
        "TESTS",
        "STATUS",
        "TIME",
        pal.reset()
    );

    let mut all_passed_count = 0;
    let mut all_failed_count = 0;
    let mut all_skipped_count = 0;

    for (suite_name, s) in &suites {
        all_passed_count += s.passed;
        all_failed_count += s.failed;
        all_skipped_count += s.skipped;

        let (status_str, status_col) = if s.failed > 0 {
            ("FAIL", pal.red())
        } else if s.skipped == s.total {
            ("SKIP", pal.yellow())
        } else {
            ("OKIE", pal.green())
        };

        let count_str = format!("{}/{}", s.passed, s.total);
        let elapsed_str = format!("{}ms", s.time_ms);
        let badge = format!(
            "{}{}[{:<4}]{}",
            status_col,
            pal.bold(),
            status_str,
            pal.reset()
        );

        println!(
            "{}{:<14}{} {}{:<9}{} {}     {}{:<8}{}",
            pal.white(),
            suite_name,
            pal.reset(),
            pal.dark_gray(),
            count_str,
            pal.reset(),
            badge,
            pal.gray(),
            elapsed_str,
            pal.reset()
        );
    }

    if !failures.is_empty() {
        for (name, details) in &failures {
            let styled_name = pal.format_scoped_name(name);
            println!(
                "\n{}{}FAIL{} {}",
                pal.red(),
                pal.bold(),
                pal.reset(),
                styled_name
            );
            for line in details.lines() {
                println!("  {}{line}{}", pal.red(), pal.reset());
            }
        }
    }

    let res_badge = if all_failed_count > 0 {
        format!("{}{}Result: FAIL{}", pal.red(), pal.bold(), pal.reset())
    } else {
        format!("{}{}Result: OKIE{}", pal.green(), pal.bold(), pal.reset())
    };

    println!(
        "\n{} {}|{} {}{} passed{}, {}{} failed{}, {}{} skipped{} {}in {:.2}s{}",
        res_badge,
        pal.dark_gray(),
        pal.reset(),
        pal.green(),
        all_passed_count,
        pal.reset(),
        pal.red(),
        all_failed_count,
        pal.reset(),
        pal.yellow(),
        all_skipped_count,
        pal.reset(),
        pal.dark_gray(),
        total_elapsed.as_secs_f64(),
        pal.reset()
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_run_tests(
    count: usize,
    tests: *const YukinoTestDesc,
    argc: i32,
    argv: *const *const u8,
    default_timeout_ms: u64,
) -> i32 {
    let test_slice = if count == 0 || tests.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(tests, count) }
    };

    let args = parse_test_args(argc, argv);
    let default_timeout_ms = args.timeout_ms.unwrap_or(default_timeout_ms);

    if let Some(target_name) = &args.internal_run_test {
        for t in test_slice {
            if t.name() == target_name {
                execute_single_worker(t);
            }
        }
        eprintln!("error: internal test '{target_name}' not found");
        std::process::exit(1);
    }

    let mut filtered_tests: Vec<&YukinoTestDesc> = Vec::new();
    for t in test_slice {
        if let Some(filter) = &args.filter {
            let matched = if args.exact {
                t.name() == filter
            } else {
                t.name().contains(filter)
            };
            if !matched {
                continue;
            }
        }

        if t.ignore {
            if args.ignored || args.include_ignored {
                filtered_tests.push(t);
            }
        } else if !args.ignored {
            filtered_tests.push(t);
        }
    }

    if args.list {
        for t in &filtered_tests {
            if t.ignore {
                println!("{}: test (ignored)", t.name());
            } else {
                println!("{}: test", t.name());
            }
        }
        return 0;
    }

    let pal = TestPalette::new();
    let self_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("fatal: cannot determine current test executable path: {e}");
            return 1;
        }
    };

    let total = filtered_tests.len();
    if total == 0 {
        println!(
            "\n{}{:<14} {:<9} {:<10} {:<8}{}",
            pal.dark_gray(),
            "SUITE",
            "TESTS",
            "STATUS",
            "TIME",
            pal.reset()
        );
        println!(
            "\n{}{}Result: OKIE{} {}|{} {}0 passed{} {}in 0.00s{}",
            pal.green(),
            pal.bold(),
            pal.reset(),
            pal.dark_gray(),
            pal.reset(),
            pal.white(),
            pal.reset(),
            pal.dark_gray(),
            pal.reset()
        );
        return 0;
    }

    let thread_count = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()))
        .max(1);

    let start_instant = Instant::now();
    let results: Arc<Mutex<Vec<TestExecutionResult>>> = Arc::new(Mutex::new(Vec::new()));
    let test_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let has_failure = Arc::new(AtomicBool::new(false));

    let spinner_chars = ['-', '\\', '|', '/'];
    let spinner_idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    std::thread::scope(|s| {
        for _ in 0..thread_count {
            let results_ref = Arc::clone(&results);
            let index_ref = Arc::clone(&test_index);
            let fail_ref = Arc::clone(&has_failure);
            let spin_ref = Arc::clone(&spinner_idx);
            let self_exe_ref = &self_exe;
            let pal_ref = &pal;
            let tests_ref = &filtered_tests;

            s.spawn(move || {
                loop {
                    if args.fail_fast && fail_ref.load(Ordering::Relaxed) {
                        break;
                    }

                    let idx = index_ref.fetch_add(1, Ordering::SeqCst);
                    if idx >= total {
                        break;
                    }

                    let t = tests_ref[idx];
                    let test_name = t.name().to_string();
                    let suite_name = t.suite_name().to_string();

                    if !args.quiet && std::io::stdout().is_terminal() {
                        let sp_i = spin_ref.fetch_add(1, Ordering::Relaxed);
                        let sp = spinner_chars[sp_i % spinner_chars.len()];
                        let line = format_live_line(
                            pal_ref,
                            sp,
                            &test_name,
                            idx + 1,
                            total,
                            start_instant.elapsed(),
                        );
                        let mut out = std::io::stdout().lock();
                        let _ = write!(out, "{line}");
                        let _ = out.flush();
                    }

                    let test_start = Instant::now();
                    let outcome = if t.ignore && !args.ignored && !args.include_ignored {
                        TestOutcome::Ignored {
                            reason: t.ignore_reason().map(ToString::to_string),
                        }
                    } else {
                        run_worker_process(
                            self_exe_ref,
                            t,
                            default_timeout_ms,
                            args.nocapture,
                        )
                    };
                    let test_duration = test_start.elapsed();

                    if matches!(outcome, TestOutcome::Failed { .. }) {
                        fail_ref.store(true, Ordering::Relaxed);
                    }

                    let mut guard = results_ref.lock().unwrap();
                    guard.push(TestExecutionResult {
                        name: test_name,
                        suite: suite_name,
                        outcome,
                        duration: test_duration,
                    });
                }
            });
        }
    });

    if !args.quiet && std::io::stdout().is_terminal() {
        let mut out = std::io::stdout().lock();
        let _ = write!(out, "\r\x1b[2K");
        let _ = out.flush();
    }

    let final_results = results.lock().unwrap().clone();
    let total_elapsed = start_instant.elapsed();

    render_final_table(&pal, &final_results, total_elapsed);

    let any_failed = final_results
        .iter()
        .any(|r| matches!(r.outcome, TestOutcome::Failed { .. }));

    if any_failed { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn dummy_test() {}

    #[test]
    fn test_desc_accessors() {
        let name = "math::test_add";
        let expected = "panic expected";
        let reason = "ignore reason";
        let desc = YukinoTestDesc {
            name_ptr: name.as_ptr(),
            name_len: name.len(),
            test_fn: dummy_test,
            should_panic: true,
            expected_panic_ptr: expected.as_ptr(),
            expected_panic_len: expected.len(),
            ignore: true,
            ignore_reason_ptr: reason.as_ptr(),
            ignore_reason_len: reason.len(),
            timeout_ms: 500,
        };

        assert_eq!(desc.name(), "math::test_add");
        assert_eq!(desc.suite_name(), "math");
        assert_eq!(desc.expected_panic(), Some("panic expected"));
        assert_eq!(desc.ignore_reason(), Some("ignore reason"));
        assert_eq!(desc.timeout_ms, 500);
        assert!(desc.should_panic);
        assert!(desc.ignore);
    }

    #[test]
    fn test_parse_cli_arguments() {
        let args = vec![
            std::ffi::CString::new("prog").unwrap(),
            std::ffi::CString::new("math").unwrap(),
            std::ffi::CString::new("--exact").unwrap(),
            std::ffi::CString::new("--nocapture").unwrap(),
            std::ffi::CString::new("-j").unwrap(),
            std::ffi::CString::new("8").unwrap(),
            std::ffi::CString::new("--fail-fast").unwrap(),
            std::ffi::CString::new("--timeout").unwrap(),
            std::ffi::CString::new("2500").unwrap(),
        ];

        let ptrs: Vec<*const u8> = args.iter().map(|s| s.as_ptr().cast::<u8>()).collect();
        let parsed = parse_test_args(ptrs.len() as i32, ptrs.as_ptr());

        assert_eq!(parsed.filter.as_deref(), Some("math"));
        assert!(parsed.exact);
        assert!(parsed.nocapture);
        assert_eq!(parsed.threads, Some(8));
        assert!(parsed.fail_fast);
        assert_eq!(parsed.timeout_ms, Some(2500));
    }

    #[test]
    fn test_format_live_line_layout() {
        let pal = TestPalette { use_colors: false };
        let line = format_live_line(&pal, '*', "math::test_add", 1, 10, Duration::from_millis(50));
        assert!(line.contains("math::test_add"));
        assert!(line.contains("[#         ]"));
        assert!(line.contains("[1/10]"));
    }
}
