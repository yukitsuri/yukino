#![deny(unsafe_code)]

use std::io::{BufRead, BufReader, IsTerminal, Write, stdout};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

use crate::project::ProjectError;
use crate::should_color;
use crate::tracker::{
    ThreadStatus, ThreadTracker, format_thread_status_linear, parse_thread_status_line,
};

pub fn is_interactive_terminal() -> bool {
    stdout().is_terminal()
        && std::env::var("TERM").map_or(true, |t| t != "dumb")
        && std::env::var("CI").is_err()
}

struct DashboardInner {
    tracker: ThreadTracker,
    interactive: bool,
    lines_drawn: usize,
    terminal_width: usize,
    minibar: bool,
    extra_info: bool,
    no_warns: bool,
    noanim: bool,
    no_out: bool,
}

impl DashboardInner {
    fn new(num_threads: usize, interactive: bool, minibar: bool, noanim: bool) -> Self {
        let width = Self::detect_terminal_width();
        Self {
            tracker: ThreadTracker::new(num_threads),
            interactive,
            lines_drawn: 0,
            terminal_width: width,
            minibar,
            extra_info: false,
            no_warns: false,
            noanim,
            no_out: false,
        }
    }

    fn detect_terminal_width() -> usize {
        crossterm::terminal::size()
            .map_or(80, |(w, _)| w as usize)
            .max(20)
    }

    fn refresh_width(&mut self) {
        self.terminal_width = Self::detect_terminal_width();
    }

    fn clear_lines(&mut self) {
        if self.interactive && self.lines_drawn > 0 {
            let mut out = stdout().lock();
            let _ = write!(out, "\x1b[{}A\r\x1b[J", self.lines_drawn);
            let _ = out.flush();
            self.lines_drawn = 0;
        }
    }

    fn redraw(&mut self) {
        if self.no_out || !self.interactive {
            return;
        }
        self.clear_lines();
        self.refresh_width();

        let mut out = stdout().lock();
        let _ = writeln!(out);

        let colored = crate::should_color();
        let status_lines = self.tracker.render_display_lines(colored, self.minibar);
        for line in &status_lines {
            let vis_len = strip_ansi_codes(line).chars().count();
            if vis_len >= self.terminal_width {
                let max_len = self.terminal_width.saturating_sub(1);
                let plain = strip_ansi_codes(line);
                let truncated: String = plain.chars().take(max_len).collect();
                let _ = writeln!(out, "{truncated}");
            } else {
                let _ = writeln!(out, "{line}");
            }
        }
        let _ = writeln!(out);
        let _ = out.flush();
        self.lines_drawn = 2 + status_lines.len();
    }

    fn log(&mut self, message: &str) {
        if self.no_out {
            return;
        }
        if self.no_warns {
            let plain = strip_ansi_codes(message);
            let trimmed = plain.trim_start();
            if trimmed.starts_with("warning:") || trimmed.starts_with("warning[") {
                return;
            }
        }
        if !self.interactive {
            println!("{message}");
            return;
        }

        self.clear_lines();
        {
            let mut out = stdout().lock();
            for line in message.lines() {
                let _ = writeln!(out, "{line}");
            }
            let _ = out.flush();
        }
        self.redraw();
    }

    fn update_status(&mut self, thread_idx: usize, status: ThreadStatus) {
        if self.no_out {
            return;
        }
        if self.interactive {
            self.tracker.set_status(thread_idx, status);
            self.redraw();
        } else {
            if let Some(linear) = format_thread_status_linear(thread_idx + 1, &status) {
                println!("{linear}");
            }
            self.tracker.set_status(thread_idx, status);
        }
    }

    fn set_idle(&mut self, thread_idx: usize) {
        if self.no_out {
            return;
        }
        self.tracker.set_idle(thread_idx);
        if self.interactive {
            self.redraw();
        }
    }

    fn set_all_idle(&mut self) {
        if self.no_out {
            return;
        }
        self.tracker.set_all_idle();
        if self.interactive {
            self.redraw();
        }
    }

    fn mask_text_with_particles(
        text: &str,
        chance: u64,
        particles: &[char],
        rng: &mut u64,
    ) -> String {
        text.chars()
            .map(|ch| {
                if ch == ' ' {
                    return ' ';
                }
                *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                if ((*rng >> 32) % 100) < chance {
                    let idx = ((*rng >> 40) % particles.len() as u64) as usize;
                    particles[idx]
                } else {
                    ch
                }
            })
            .collect()
    }

    fn animate_minibar_evaporate(&mut self) {
        let colored = crate::should_color();
        let raw_text = self.tracker.render_minibar(false);
        let particles = ['.', ':', '*', ' '];
        let mut rng: u64 = 0x1234_5678_9abc_def0;

        for frame in 0..3 {
            self.clear_lines();
            let mask_chance = 35 + (frame as u64) * 25;
            let (r, g, b) = match frame {
                0 => (100u8, 160u8, 200u8),
                1 => (65u8, 110u8, 145u8),
                _ => (30u8, 60u8, 90u8),
            };
            let text = Self::mask_text_with_particles(&raw_text, mask_chance, &particles, &mut rng);
            let mut out = stdout().lock();
            let _ = writeln!(out);
            if colored {
                let _ = writeln!(out, "\x1b[38;2;{r};{g};{b}m{text}\x1b[0m");
            } else {
                let _ = writeln!(out, "{text}");
            }
            let _ = writeln!(out);
            let _ = out.flush();
            self.lines_drawn = 3;
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    fn animate_fullbar_dust(&mut self) {
        let colored = crate::should_color();
        let display_lines = self.tracker.render_display_lines(false, false);
        let particles = ['.', ':', '*', ' '];
        let mut rng: u64 = 0x9876_5432_10fe_dcba;

        for frame in 0..3 {
            self.clear_lines();
            let mask_chance = 35 + (frame as u64) * 25;
            let (r, g, b) = match frame {
                0 => (140u8, 160u8, 180u8),
                1 => (90u8, 110u8, 130u8),
                _ => (45u8, 60u8, 75u8),
            };
            let mut out = stdout().lock();
            let _ = writeln!(out);
            for line in &display_lines {
                let text = Self::mask_text_with_particles(line, mask_chance, &particles, &mut rng);
                if colored {
                    let _ = writeln!(out, "\x1b[38;2;{r};{g};{b}m{text}\x1b[0m");
                } else {
                    let _ = writeln!(out, "{text}");
                }
            }
            let _ = writeln!(out);
            let _ = out.flush();
            self.lines_drawn = 2 + display_lines.len();
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    fn animate_materialize_summary(&mut self, summary_lines: &[String]) {
        let colored = crate::should_color();
        let plain_lines: Vec<String> = summary_lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let particles = ['.', ':', '*'];
        let mut rng: u64 = 0xa5a5_5a5a_3c3c_c3c3;

        for frame in 0..3 {
            self.clear_lines();
            let mask_chance = match frame {
                0 => 75,
                1 => 45,
                _ => 15,
            };
            let (r, g, b) = match frame {
                0 => (45u8, 70u8, 95u8),
                1 => (110u8, 155u8, 190u8),
                _ => (185u8, 215u8, 235u8),
            };

            let mut out = stdout().lock();
            for line in &plain_lines {
                let text = Self::mask_text_with_particles(line, mask_chance, &particles, &mut rng);
                if colored {
                    let _ = writeln!(out, "\x1b[38;2;{r};{g};{b}m{text}\x1b[0m");
                } else {
                    let _ = writeln!(out, "{text}");
                }
            }
            let _ = out.flush();
            self.lines_drawn = plain_lines.len();
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        self.clear_lines();
        let mut out = stdout().lock();
        for line in summary_lines {
            let _ = writeln!(out, "{line}");
        }
        let _ = out.flush();
        self.lines_drawn = 0;
    }

    fn finish(&mut self) {
        if self.no_out || !self.interactive || self.lines_drawn == 0 {
            self.lines_drawn = 0;
            return;
        }

        if !self.noanim {
            if self.minibar {
                self.animate_minibar_evaporate();
            } else {
                self.animate_fullbar_dust();
            }
        }

        self.clear_lines();
    }

    fn finish_with_summary(&mut self, summary_lines: &[String]) {
        if self.no_out {
            self.clear_lines();
            self.lines_drawn = 0;
            return;
        }

        if !self.interactive || self.lines_drawn == 0 {
            self.lines_drawn = 0;
            for line in summary_lines {
                println!("{line}");
            }
            return;
        }

        if self.noanim {
            self.clear_lines();
            for line in summary_lines {
                println!("{line}");
            }
            self.lines_drawn = 0;
            return;
        }

        if self.minibar {
            self.animate_minibar_evaporate();
        } else {
            self.animate_fullbar_dust();
        }
        self.clear_lines();

        self.animate_materialize_summary(summary_lines);
    }
}

impl Drop for DashboardInner {
    fn drop(&mut self) {
        self.clear_lines();
    }
}

fn strip_ansi_codes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_escape = false;
    for c in input.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Clone)]
pub struct BuildDashboard {
    inner: Arc<Mutex<DashboardInner>>,
}

impl BuildDashboard {
    pub fn new(num_threads: usize) -> Self {
        Self::with_interactive(num_threads, is_interactive_terminal())
    }

    pub fn with_interactive(num_threads: usize, interactive: bool) -> Self {
        let minibar = crate::config::load_global_minibar();
        let noanim = crate::config::load_global_noanim();
        Self::with_options(num_threads, interactive, minibar, noanim)
    }

    pub fn with_options(num_threads: usize, interactive: bool, minibar: bool, noanim: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DashboardInner::new(num_threads, interactive, minibar, noanim))),
        }
    }

    pub fn set_minibar(&self, minibar: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.minibar = minibar;
        }
    }

    pub fn set_extra_info(&self, extra_info: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.extra_info = extra_info;
        }
    }

    pub fn set_no_warns(&self, no_warns: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.no_warns = no_warns;
        }
    }

    pub fn set_noanim(&self, noanim: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.noanim = noanim;
        }
    }

    pub fn set_no_out(&self, no_out: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.no_out = no_out;
        }
    }

    pub fn is_extra_info(&self) -> bool {
        self.inner.lock().map_or(false, |inner| inner.extra_info)
    }

    pub fn is_no_warns(&self) -> bool {
        self.inner.lock().map_or(false, |inner| inner.no_warns)
    }

    pub fn is_noanim(&self) -> bool {
        self.inner.lock().map_or(false, |inner| inner.noanim)
    }

    pub fn is_no_out(&self) -> bool {
        self.inner.lock().map_or(false, |inner| inner.no_out)
    }

    pub fn is_interactive(&self) -> bool {
        self.inner.lock().is_ok_and(|d| d.interactive)
    }

    pub fn start(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.redraw();
        }
    }

    pub fn log(&self, message: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.log(message);
        } else {
            println!("{message}");
        }
    }

    pub fn update_status(&self, thread_idx: usize, status: ThreadStatus) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.update_status(thread_idx, status);
        }
    }

    pub fn set_idle(&self, thread_idx: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.set_idle(thread_idx);
        }
    }

    pub fn set_all_idle(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.set_all_idle();
        }
    }

    pub fn finish(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.finish();
        }
    }

    pub fn finish_with_summary(&self, summary_lines: &[String]) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.finish_with_summary(summary_lines);
        } else {
            for line in summary_lines {
                println!("{line}");
            }
        }
    }

    pub fn handle_compiler_line(&self, line: &str) {
        self.handle_compiler_line_internal(line, false, true);
    }

    pub fn handle_compiler_line_internal(&self, line: &str, is_lib: bool, full_lib_log: bool) {
        if self.is_no_out() {
            return;
        }
        let clean = strip_ansi_codes(line);
        if let Some((thread_id, status)) = parse_thread_status_line(&clean) {
            self.update_status(thread_id.saturating_sub(1), status);
            return;
        }
        if clean.trim_start().starts_with("thread :: ") {
            return;
        }

        let trimmed = clean.trim_start();
        if !self.is_extra_info() && trimmed == "Compiler initialized" {
            return;
        }
        if self.is_no_warns() && (trimmed.starts_with("warning:") || trimmed.starts_with("warning[")) {
            return;
        }

        if let Some(rest) = trimmed.strip_prefix("Compiled ") {
            let target_str = rest.split(" -> ").last().unwrap_or(rest).trim();
            let cgu_token = target_str.split_whitespace().next().unwrap_or(target_str);
            let path = Path::new(cgu_token);
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem.starts_with("cgu_")
            {
                if let Ok(mut inner) = self.inner.lock() {
                    inner.tracker.set_status(
                        0,
                        ThreadStatus::Emitting {
                            cgu_name: stem.to_string(),
                            current: 1,
                            total: 1,
                        },
                    );
                }
            }
        }

        if trimmed.starts_with("Artifact:") {
            return;
        }

        if is_lib && !full_lib_log {
            return;
        }

        self.log(line);
    }

    pub fn run_compiler_command(&self, cmd: &mut Command) -> Result<ExitStatus, ProjectError> {
        self.run_compiler_command_filtered(cmd, false)
    }

    pub fn run_lib_compiler_command(&self, cmd: &mut Command) -> Result<ExitStatus, ProjectError> {
        self.run_compiler_command_filtered(cmd, true)
    }

    pub fn run_compiler_command_filtered(
        &self,
        cmd: &mut Command,
        is_lib: bool,
    ) -> Result<ExitStatus, ProjectError> {
        if should_color() {
            cmd.env("CLICOLOR_FORCE", "1");
        }

        if self.is_no_out() {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            let mut child = cmd
                .spawn()
                .map_err(|e| ProjectError::Io(format!("failed to run command: {e}")))?;
            let status = child
                .wait()
                .map_err(|e| ProjectError::Io(format!("failed to wait for command: {e}")))?;
            return Ok(status);
        }

        cmd.stdout(Stdio::piped());
        if self.is_interactive() {
            cmd.stderr(Stdio::piped());
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| ProjectError::Io(format!("failed to run command: {e}")))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let dash_out = self.clone();
        let full_lib_log = crate::config::load_global_full_lib_log();
        let out_handle = std::thread::spawn(move || {
            if let Some(pipe) = stdout {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(Result::ok) {
                    dash_out.handle_compiler_line_internal(&line, is_lib, full_lib_log);
                }
            }
        });

        let dash_err = self.clone();
        let err_handle = std::thread::spawn(move || {
            if let Some(pipe) = stderr {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(Result::ok) {
                    dash_err.log(&line);
                }
            }
        });

        let status = child
            .wait()
            .map_err(|e| ProjectError::Io(format!("failed to wait for command: {e}")))?;

        let _ = out_handle.join();
        let _ = err_handle.join();

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_interactive_dashboard_flow() {
        let dashboard = BuildDashboard::with_interactive(2, false);
        assert!(!dashboard.is_interactive());

        dashboard.start();
        dashboard.log("compiling test module");
        dashboard.update_status(
            0,
            ThreadStatus::Compiling {
                target: "demo".to_string(),
                current: 1,
                total: 2,
                speed: None,
            },
        );
        dashboard.set_idle(0);
        dashboard.finish();
    }

    #[test]
    fn test_compiler_line_handling() {
        let dashboard = BuildDashboard::with_interactive(2, false);
        dashboard.handle_compiler_line("Compiled /path/to/cgu_0.o");

        {
            let inner = dashboard.inner.lock().unwrap();
            assert_eq!(
                inner.tracker.get_status(0),
                Some(&ThreadStatus::Emitting {
                    cgu_name: "cgu_0".to_string(),
                    current: 1,
                    total: 1,
                })
            );
        }

        dashboard.handle_compiler_line("Compiled src/main.ykn -> cgu_1.o");
        {
            let inner = dashboard.inner.lock().unwrap();
            assert_eq!(
                inner.tracker.get_status(0),
                Some(&ThreadStatus::Emitting {
                    cgu_name: "cgu_1".to_string(),
                    current: 1,
                    total: 1,
                })
            );
        }

        dashboard.handle_compiler_line("\x1b[1;32m    Compiled\x1b[0m src/main.ykn \x1b[2m->\x1b[0m cgu_2.o");
        {
            let inner = dashboard.inner.lock().unwrap();
            assert_eq!(
                inner.tracker.get_status(0),
                Some(&ThreadStatus::Emitting {
                    cgu_name: "cgu_2".to_string(),
                    current: 1,
                    total: 1,
                })
            );
        }

        dashboard.handle_compiler_line("thread :: 1 [8/8 | 100%]: using cached cgu_misc_0");
        {
            let inner = dashboard.inner.lock().unwrap();
            assert_eq!(
                inner.tracker.get_status(0),
                Some(&ThreadStatus::Emitting {
                    cgu_name: "cgu_misc_0".to_string(),
                    current: 8,
                    total: 8,
                })
            );
        }
    }

    #[test]
    fn test_extra_info_and_no_warns_flags() {
        let dashboard = BuildDashboard::with_interactive(2, false);
        assert!(!dashboard.is_extra_info());
        assert!(!dashboard.is_no_warns());
        assert!(!dashboard.is_noanim());
        assert!(!dashboard.is_no_out());

        dashboard.set_extra_info(true);
        assert!(dashboard.is_extra_info());

        dashboard.set_no_warns(true);
        assert!(dashboard.is_no_warns());

        dashboard.set_noanim(true);
        assert!(dashboard.is_noanim());

        dashboard.set_no_out(true);
        assert!(dashboard.is_no_out());
    }

    #[test]
    fn test_finish_with_summary_flow() {
        let dashboard = BuildDashboard::with_interactive(2, false);
        let summary = vec![
            "    Finished: release target(s) in 0.12s".to_string(),
            "    Artifact: target/release/demo (128 KB)".to_string(),
        ];
        dashboard.finish_with_summary(&summary);

        let mut dash_no_out = DashboardInner::new(2, false, false, false);
        dash_no_out.no_out = true;
        dash_no_out.finish_with_summary(&summary);
        assert_eq!(dash_no_out.lines_drawn, 0);

        let mut dash_noanim = DashboardInner::new(2, false, false, true);
        dash_noanim.finish_with_summary(&summary);
        assert_eq!(dash_noanim.lines_drawn, 0);
    }

    #[test]
    fn test_lib_compiler_line_filtered() {
        let dashboard = BuildDashboard::with_interactive(2, false);
        // When is_lib = true and full_lib_log = false, thread statuses still update
        dashboard.handle_compiler_line_internal("thread :: 1 [3/10 | 30%]: compiling dep/lib.ykn", true, false);
        {
            let inner = dashboard.inner.lock().unwrap();
            assert_eq!(
                inner.tracker.get_status(0),
                Some(&ThreadStatus::Compiling {
                    target: "dep/lib.ykn".to_string(),
                    current: 3,
                    total: 10,
                    speed: None,
                })
            );
        }
    }
}
