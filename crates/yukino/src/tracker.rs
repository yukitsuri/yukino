#![deny(unsafe_code)]

use owo_colors::OwoColorize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadStatus {
    Idle,
    Compiling {
        target: String,
        current: usize,
        total: usize,
        speed: Option<String>,
    },
    Downloading {
        dep: String,
        current: usize,
        total: usize,
        speed: Option<String>,
    },
    Unpacking {
        dep: String,
        current: usize,
        total: usize,
    },
    Codegen {
        cgu_name: String,
        item: String,
        current: usize,
        total: usize,
    },
    Optimizing {
        cgu_name: String,
        current: usize,
        total: usize,
    },
    Emitting {
        cgu_name: String,
        current: usize,
        total: usize,
    },
}

pub fn format_thread_status(thread_id: usize, status: &ThreadStatus) -> String {
    match status {
        ThreadStatus::Idle => format!("thread :: {thread_id} [waiting]: idle"),
        ThreadStatus::Compiling {
            target,
            current,
            total,
            speed,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            if let Some(sp) = speed {
                format!("thread :: {thread_id} [{current}/{total} | {pct}% | {sp}]: compiling {target}")
            } else {
                format!("thread :: {thread_id} [{current}/{total} | {pct}%]: compiling {target}")
            }
        }
        ThreadStatus::Downloading {
            dep,
            current,
            total,
            speed,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            if let Some(sp) = speed {
                format!("thread :: {thread_id} [{current}/{total} | {pct}% | {sp}]: downloading {dep}")
            } else {
                format!("thread :: {thread_id} [{current}/{total} | {pct}%]: downloading {dep}")
            }
        }
        ThreadStatus::Unpacking {
            dep,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: unpacking {dep}")
        }
        ThreadStatus::Codegen {
            cgu_name,
            item,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            if item.is_empty() {
                format!("thread :: {thread_id} [{current}/{total} | {pct}%]: codegen {cgu_name}")
            } else {
                format!(
                    "thread :: {thread_id} [{current}/{total} | {pct}%]: codegen {cgu_name} ({item})"
                )
            }
        }
        ThreadStatus::Optimizing {
            cgu_name,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: optimizing {cgu_name}")
        }
        ThreadStatus::Emitting {
            cgu_name,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: emitting {cgu_name}")
        }
    }
}

pub fn format_thread_linear(thread_id: usize, event: &str) -> String {
    format!("[thread {thread_id}] {event}")
}

pub fn format_thread_status_linear(thread_id: usize, status: &ThreadStatus) -> Option<String> {
    match status {
        ThreadStatus::Idle => None,
        ThreadStatus::Compiling { target, .. } => {
            Some(format_thread_linear(thread_id, &format!("compiling {target}")))
        }
        ThreadStatus::Downloading { dep, .. } => {
            Some(format_thread_linear(thread_id, &format!("downloading {dep}")))
        }
        ThreadStatus::Unpacking { dep, .. } => {
            Some(format_thread_linear(thread_id, &format!("unpacking {dep}")))
        }
        ThreadStatus::Codegen { cgu_name, .. } => {
            Some(format_thread_linear(thread_id, &format!("codegen {cgu_name}")))
        }
        ThreadStatus::Optimizing { cgu_name, .. } => {
            Some(format_thread_linear(thread_id, &format!("optimizing {cgu_name}")))
        }
        ThreadStatus::Emitting { cgu_name, .. } => {
            Some(format_thread_linear(thread_id, &format!("emitting {cgu_name}")))
        }
    }
}

fn parse_progress_prefix(trimmed: &str) -> Option<(usize, usize, Option<String>, &str)> {
    let (progress_part, action_part) = trimmed.split_once(": ")?;
    let progress_inner = progress_part.strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = progress_inner.split(" | ");
    let (cur_str, total_str) = parts.next()?.split_once('/')?;
    let current: usize = cur_str.parse().ok()?;
    let total: usize = total_str.parse().ok()?;
    let _pct = parts.next();
    let speed = parts.next().map(ToString::to_string);
    Some((current, total, speed, action_part))
}

fn parse_codegen_action(target: &str, current: usize, total: usize) -> ThreadStatus {
    let (cgu_name, item) = if let Some((cgu, item_part)) = target.split_once(" (") {
        let item = item_part.strip_suffix(')').unwrap_or(item_part);
        (cgu.to_string(), item.to_string())
    } else {
        (target.to_string(), String::new())
    };
    ThreadStatus::Codegen {
        cgu_name,
        item,
        current,
        total,
    }
}

pub fn parse_thread_status_line(line: &str) -> Option<(usize, ThreadStatus)> {
    let rest = line.strip_prefix("thread :: ")?;
    let (id_str, after_id) = rest.split_once(' ')?;
    let thread_id: usize = id_str.parse().ok()?;
    if thread_id == 0 {
        return None;
    }

    let trimmed = after_id.trim();
    if trimmed == "[waiting]: idle" {
        return Some((thread_id, ThreadStatus::Idle));
    }

    let (current, total, speed, action_part) = parse_progress_prefix(trimmed)?;
    let (action, target) = action_part.split_once(' ')?;
    let status = match action {
        "compiling" => ThreadStatus::Compiling {
            target: target.to_string(),
            current,
            total,
            speed,
        },
        "downloading" => ThreadStatus::Downloading {
            dep: target.to_string(),
            current,
            total,
            speed,
        },
        "unpacking" => ThreadStatus::Unpacking {
            dep: target.to_string(),
            current,
            total,
        },
        "codegen" => parse_codegen_action(target, current, total),
        "optimizing" => ThreadStatus::Optimizing {
            cgu_name: target.to_string(),
            current,
            total,
        },
        "emitting" => ThreadStatus::Emitting {
            cgu_name: target.to_string(),
            current,
            total,
        },
        "using" if target.starts_with("cached ") => {
            let cgu_name = target.strip_prefix("cached ").unwrap_or(target);
            ThreadStatus::Emitting {
                cgu_name: cgu_name.to_string(),
                current,
                total,
            }
        }
        _ => return None,
    };

    Some((thread_id, status))
}

#[derive(Debug, Clone)]
pub struct ThreadTracker {
    statuses: Vec<ThreadStatus>,
}

impl ThreadTracker {
    pub fn new(num_threads: usize) -> Self {
        let count = num_threads.max(1);
        Self {
            statuses: vec![ThreadStatus::Idle; count],
        }
    }

    pub fn len(&self) -> usize {
        self.statuses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.statuses.is_empty()
    }

    pub fn get_status(&self, thread_idx: usize) -> Option<&ThreadStatus> {
        self.statuses.get(thread_idx)
    }

    pub fn set_status(&mut self, thread_idx: usize, status: ThreadStatus) {
        if thread_idx < self.statuses.len() {
            self.statuses[thread_idx] = status;
        }
    }

    pub fn set_idle(&mut self, thread_idx: usize) {
        self.set_status(thread_idx, ThreadStatus::Idle);
    }

    pub fn set_all_idle(&mut self) {
        for s in &mut self.statuses {
            *s = ThreadStatus::Idle;
        }
    }

    pub fn all_idle(&self) -> bool {
        self.statuses.iter().all(|s| *s == ThreadStatus::Idle)
    }

    fn render_minibar_plain(&self, active_tasks: &[(usize, &ThreadStatus)]) -> String {
        let tasks_str = if active_tasks.is_empty() {
            "all cores idle".to_string()
        } else {
            active_tasks
                .iter()
                .map(|(id, st)| {
                    let act = short_action_name(st);
                    let tgt = short_target_name(st);
                    if tgt.is_empty() {
                        format!("[{id:02}] {act}")
                    } else {
                        format!("[{id:02}] {act} {tgt}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" · ")
        };

        format!(
            "Processing {}/{} busy: {tasks_str}",
            active_tasks.len(),
            self.statuses.len()
        )
    }

    fn render_minibar_colored(&self, active_tasks: &[(usize, &ThreadStatus)]) -> String {
        let proc = "Processing".cyan().bold().to_string();
        let busy_colored = format!(
            "{} {}",
            format!("{}/{}", active_tasks.len(), self.statuses.len()).white(),
            "busy:".dimmed()
        );

        let tasks_colored = if active_tasks.is_empty() {
            format!("{} {}", "all cores".dimmed(), "idle".cyan())
        } else {
            active_tasks
                .iter()
                .map(|(id, st)| {
                    let tag = format!("{}{}{}", "[".dimmed(), format!("{id:02}").white(), "]".dimmed());
                    let act = short_action_name(st);
                    let act_col = match st {
                        ThreadStatus::Codegen { .. } => act.cyan().to_string(),
                        ThreadStatus::Optimizing { .. } => act.blue().to_string(),
                        ThreadStatus::Emitting { .. } | ThreadStatus::Compiling { .. } => act.bright_cyan().to_string(),
                        ThreadStatus::Downloading { .. } => act.blue().to_string(),
                        ThreadStatus::Unpacking { .. } => act.cyan().to_string(),
                        ThreadStatus::Idle => act.dimmed().to_string(),
                    };
                    let tgt = short_target_name(st);
                    if tgt.is_empty() {
                        format!("{tag} {act_col}")
                    } else {
                        format!("{tag} {act_col} {}", tgt.white())
                    }
                })
                .collect::<Vec<_>>()
                .join(&" · ".dimmed().to_string())
        };

        format!("{proc} {busy_colored} {tasks_colored}")
    }

    pub fn render_minibar(&self, colored: bool) -> String {
        let active_tasks: Vec<(usize, &ThreadStatus)> = self
            .statuses
            .iter()
            .enumerate()
            .filter(|(_, s)| **s != ThreadStatus::Idle)
            .map(|(i, s)| (i + 1, s))
            .collect();

        if colored {
            self.render_minibar_colored(&active_tasks)
        } else {
            self.render_minibar_plain(&active_tasks)
        }
    }

    pub fn render_display_lines(&self, colored: bool, minibar: bool) -> Vec<String> {
        if minibar {
            return vec![self.render_minibar(colored)];
        }

        let idle_count = self
            .statuses
            .iter()
            .filter(|s| **s == ThreadStatus::Idle)
            .count();

        let max_target_width = self
            .statuses
            .iter()
            .map(|s| target_display_name(s).chars().count())
            .max()
            .unwrap_or(0);

        if idle_count > 1 {
            let mut lines: Vec<String> = self
                .statuses
                .iter()
                .enumerate()
                .filter(|(_, status)| **status != ThreadStatus::Idle)
                .map(|(i, status)| format_normal_thread(i + 1, status, max_target_width, colored))
                .collect();

            let active_count = self.statuses.len().saturating_sub(idle_count);
            let idle_line = if colored {
                let circle = "○".cyan().to_string();
                let tag = format!("{}{}{}", "[".dimmed(), "##".white(), "]".dimmed());
                let text = if active_count > 0 {
                    format!(
                        "{} {} {}",
                        format!("{idle_count}").white(),
                        "more cores are currently".white(),
                        "idle".cyan()
                    )
                } else {
                    format!(
                        "{} {} {}",
                        format!("{idle_count}").white(),
                        "cores are currently".white(),
                        "idle".cyan()
                    )
                };
                format!("{circle} {tag} {text}")
            } else if active_count > 0 {
                format!("○ [##] {idle_count} more cores are currently idle")
            } else {
                format!("○ [##] {idle_count} cores are currently idle")
            };
            lines.push(idle_line);
            lines
        } else {
            self.statuses
                .iter()
                .enumerate()
                .map(|(i, status)| format_normal_thread(i + 1, status, max_target_width, colored))
                .collect()
        }
    }

    pub fn render_lines(&self) -> Vec<String> {
        self.render_display_lines(false, false)
    }
}

fn action_display_name(status: &ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Idle => "idle",
        ThreadStatus::Compiling { .. } => "compiling",
        ThreadStatus::Downloading { .. } => "downloading",
        ThreadStatus::Unpacking { .. } => "unpacking",
        ThreadStatus::Codegen { .. } => "codegen",
        ThreadStatus::Optimizing { .. } => "optimizing",
        ThreadStatus::Emitting { .. } => "emitting",
    }
}

fn short_action_name(status: &ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Idle => "idle",
        ThreadStatus::Compiling { .. } => "compile",
        ThreadStatus::Downloading { .. } => "dl",
        ThreadStatus::Unpacking { .. } => "unpack",
        ThreadStatus::Codegen { .. } => "codegen",
        ThreadStatus::Optimizing { .. } => "opt",
        ThreadStatus::Emitting { .. } => "emit",
    }
}

fn target_display_name(status: &ThreadStatus) -> String {
    match status {
        ThreadStatus::Idle => String::new(),
        ThreadStatus::Compiling { target, .. } => target.clone(),
        ThreadStatus::Downloading { dep, .. } => dep.clone(),
        ThreadStatus::Unpacking { dep, .. } => dep.clone(),
        ThreadStatus::Codegen { cgu_name, item, .. } => {
            if item.is_empty() {
                cgu_name.clone()
            } else {
                format!("{cgu_name} ({item})")
            }
        }
        ThreadStatus::Optimizing { cgu_name, .. } => cgu_name.clone(),
        ThreadStatus::Emitting { cgu_name, .. } => cgu_name.clone(),
    }
}

fn short_target_name(status: &ThreadStatus) -> String {
    match status {
        ThreadStatus::Idle => String::new(),
        ThreadStatus::Compiling { target, .. } => target.clone(),
        ThreadStatus::Downloading { dep, .. } => dep.clone(),
        ThreadStatus::Unpacking { dep, .. } => dep.clone(),
        ThreadStatus::Codegen { cgu_name, .. } => cgu_name.clone(),
        ThreadStatus::Optimizing { cgu_name, .. } => cgu_name.clone(),
        ThreadStatus::Emitting { cgu_name, .. } => cgu_name.clone(),
    }
}

fn status_progress_parts(status: &ThreadStatus) -> (usize, usize, Option<&str>) {
    match status {
        ThreadStatus::Compiling { current, total, speed, .. }
        | ThreadStatus::Downloading { current, total, speed, .. } => (*current, *total, speed.as_deref()),
        ThreadStatus::Unpacking { current, total, .. }
        | ThreadStatus::Codegen { current, total, .. }
        | ThreadStatus::Optimizing { current, total, .. }
        | ThreadStatus::Emitting { current, total, .. } => (*current, *total, None),
        ThreadStatus::Idle => (0, 0, None),
    }
}

fn format_normal_plain(
    thread_id: usize,
    status: &ThreadStatus,
    max_target_width: usize,
) -> String {
    if *status == ThreadStatus::Idle {
        return format!("○ [{thread_id:02}] idle");
    }

    let act_name = action_display_name(status);
    let target = target_display_name(status);
    let (cur, tot, speed) = status_progress_parts(status);
    let pct = (cur * 100).checked_div(tot).unwrap_or(0);

    let circle = "●";
    let tag = format!("[{thread_id:02}]");
    let act_padded = format!("{act_name:<10}");
    let tgt_padded = format!("{target:<width$}", width = max_target_width);
    let prog = if let Some(sp) = speed {
        format!("[{cur:>2}/{tot:<2} | {pct:>3}% | {sp}]")
    } else {
        format!("[{cur:>2}/{tot:<2} | {pct:>3}%]")
    };
    format!("{circle} {tag} {act_padded} {tgt_padded}  {prog}")
}

fn format_normal_colored(
    thread_id: usize,
    status: &ThreadStatus,
    max_target_width: usize,
) -> String {
    if *status == ThreadStatus::Idle {
        return format!(
            "{} {}{}{} {}",
            "○".cyan(),
            "[".dimmed(),
            format!("{thread_id:02}").white(),
            "]".dimmed(),
            "idle".cyan()
        );
    }

    let act_name = action_display_name(status);
    let target = target_display_name(status);
    let (cur, tot, speed) = status_progress_parts(status);
    let pct = (cur * 100).checked_div(tot).unwrap_or(0);

    let circle = match status {
        ThreadStatus::Codegen { .. } => "●".cyan().to_string(),
        ThreadStatus::Optimizing { .. } => "●".blue().to_string(),
        ThreadStatus::Emitting { .. } | ThreadStatus::Compiling { .. } => "●".bright_cyan().to_string(),
        ThreadStatus::Downloading { .. } => "●".blue().to_string(),
        ThreadStatus::Unpacking { .. } => "●".cyan().to_string(),
        ThreadStatus::Idle => "○".dimmed().to_string(),
    };
    let tag = format!("{}{}{}", "[".dimmed(), format!("{thread_id:02}").white(), "]".dimmed());
    let act_padded = format!("{act_name:<10}");
    let act_col = match status {
        ThreadStatus::Codegen { .. } => act_padded.cyan().to_string(),
        ThreadStatus::Optimizing { .. } => act_padded.blue().to_string(),
        ThreadStatus::Emitting { .. } | ThreadStatus::Compiling { .. } => act_padded.bright_cyan().to_string(),
        ThreadStatus::Downloading { .. } => act_padded.blue().to_string(),
        ThreadStatus::Unpacking { .. } => act_padded.cyan().to_string(),
        ThreadStatus::Idle => act_padded.dimmed().to_string(),
    };
    let tgt_padded = format!("{target:<width$}", width = max_target_width);
    let tgt_col = tgt_padded.white().to_string();

    let prog = if let Some(sp) = speed {
        format!(
            "{}{}{}{}{}{}{}",
            "[".dimmed(),
            format!("{cur:>2}/{tot:<2}").white(),
            " | ".dimmed(),
            format!("{pct:>3}%").cyan(),
            " | ".dimmed(),
            sp.dimmed(),
            "]".dimmed()
        )
    } else {
        format!(
            "{}{}{}{}{}",
            "[".dimmed(),
            format!("{cur:>2}/{tot:<2}").white(),
            " | ".dimmed(),
            format!("{pct:>3}%").cyan(),
            "]".dimmed()
        )
    };

    format!("{circle} {tag} {act_col} {tgt_col}  {prog}")
}

fn format_normal_thread(
    thread_id: usize,
    status: &ThreadStatus,
    max_target_width: usize,
    colored: bool,
) -> String {
    if colored {
        format_normal_colored(thread_id, status, max_target_width)
    } else {
        format_normal_plain(thread_id, status, max_target_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_lines_multiple_idle_collapsed() {
        let mut tracker = ThreadTracker::new(4);
        tracker.set_status(
            1,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_test".to_string(),
                current: 1,
                total: 2,
            },
        );

        let lines = tracker.render_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "● [02] codegen    cgu_0 (fn_test)  [ 1/2  |  50%]");
        assert_eq!(lines[1], "○ [##] 3 more cores are currently idle");
    }

    #[test]
    fn test_render_lines_single_idle_preserved() {
        let mut tracker = ThreadTracker::new(2);
        tracker.set_status(
            0,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_test".to_string(),
                current: 1,
                total: 2,
            },
        );

        let lines = tracker.render_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "● [01] codegen    cgu_0 (fn_test)  [ 1/2  |  50%]");
        assert_eq!(lines[1], "○ [02] idle");
    }

    #[test]
    fn test_render_lines_all_active() {
        let mut tracker = ThreadTracker::new(2);
        tracker.set_status(
            0,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_test".to_string(),
                current: 1,
                total: 2,
            },
        );
        tracker.set_status(
            1,
            ThreadStatus::Optimizing {
                cgu_name: "cgu_1".to_string(),
                current: 2,
                total: 2,
            },
        );

        let lines = tracker.render_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "● [01] codegen    cgu_0 (fn_test)  [ 1/2  |  50%]");
        assert_eq!(lines[1], "● [02] optimizing cgu_1            [ 2/2  | 100%]");
    }

    #[test]
    fn test_render_lines_all_idle() {
        let tracker = ThreadTracker::new(3);
        let lines = tracker.render_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "○ [##] 3 cores are currently idle");
    }

    #[test]
    fn test_render_minibar_plain() {
        let mut tracker = ThreadTracker::new(8);
        tracker.set_status(
            0,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_main".to_string(),
                current: 3,
                total: 10,
            },
        );
        tracker.set_status(
            1,
            ThreadStatus::Optimizing {
                cgu_name: "cgu_1".to_string(),
                current: 2,
                total: 2,
            },
        );

        let bar = tracker.render_minibar(false);
        assert_eq!(
            bar,
            "Processing 2/8 busy: [01] codegen cgu_0 · [02] opt cgu_1"
        );
    }

    #[test]
    fn test_render_display_lines_colored() {
        let mut tracker = ThreadTracker::new(2);
        tracker.set_status(
            0,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_main".to_string(),
                current: 3,
                total: 10,
            },
        );

        let lines = tracker.render_display_lines(true, false);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\x1b["));
        assert!(lines[1].contains("\x1b["));

        let mini = tracker.render_display_lines(true, true);
        assert_eq!(mini.len(), 1);
        assert!(mini[0].contains("\x1b["));
    }

    #[test]
    fn test_render_minibar_idle_message() {
        let mut tracker = ThreadTracker::new(2);
        tracker.set_status(
            0,
            ThreadStatus::Emitting {
                cgu_name: "cgu_0".to_string(),
                current: 2,
                total: 2,
            },
        );
        let bar_active = tracker.render_minibar(false);
        assert_eq!(bar_active, "Processing 1/2 busy: [01] emit cgu_0");

        tracker.set_idle(0);
        let bar_idle = tracker.render_minibar(false);
        assert_eq!(bar_idle, "Processing 0/2 busy: all cores idle");
    }

    #[test]
    fn test_format_thread_status_idle() {
        let status = ThreadStatus::Idle;
        let line = format_thread_status(2, &status);
        assert_eq!(line, "thread :: 2 [waiting]: idle");
    }

    #[test]
    fn test_format_thread_status_active() {
        let status = ThreadStatus::Codegen {
            cgu_name: "cgu_0".to_string(),
            item: "fn_main".to_string(),
            current: 3,
            total: 10,
        };
        let line = format_thread_status(1, &status);
        assert_eq!(line, "thread :: 1 [3/10 | 30%]: codegen cgu_0 (fn_main)");
    }

    #[test]
    fn test_format_thread_status_compiling_and_unpacking() {
        let comp = ThreadStatus::Compiling {
            target: "std.io".to_string(),
            current: 3,
            total: 10,
            speed: Some("2.5 MB/s".to_string()),
        };
        assert_eq!(
            format_thread_status(1, &comp),
            "thread :: 1 [3/10 | 30% | 2.5 MB/s]: compiling std.io"
        );

        let unp = ThreadStatus::Unpacking {
            dep: "yuki-http".to_string(),
            current: 1,
            total: 4,
        };
        assert_eq!(
            format_thread_status(3, &unp),
            "thread :: 3 [1/4 | 25%]: unpacking yuki-http"
        );
    }

    #[test]
    fn test_parse_thread_status_roundtrip() {
        let idle_str = "thread :: 2 [waiting]: idle";
        assert_eq!(
            parse_thread_status_line(idle_str),
            Some((2, ThreadStatus::Idle))
        );

        let comp_str = "thread :: 1 [3/10 | 30% | 2.5 MB/s]: compiling std.io";
        assert_eq!(
            parse_thread_status_line(comp_str),
            Some((
                1,
                ThreadStatus::Compiling {
                    target: "std.io".to_string(),
                    current: 3,
                    total: 10,
                    speed: Some("2.5 MB/s".to_string())
                }
            ))
        );

        let unp_str = "thread :: 3 [1/4 | 25%]: unpacking yuki-http";
        assert_eq!(
            parse_thread_status_line(unp_str),
            Some((
                3,
                ThreadStatus::Unpacking {
                    dep: "yuki-http".to_string(),
                    current: 1,
                    total: 4
                }
            ))
        );

        let code_str = "thread :: 1 [3/10 | 30%]: codegen cgu_0 (fn_main)";
        assert_eq!(
            parse_thread_status_line(code_str),
            Some((
                1,
                ThreadStatus::Codegen {
                    cgu_name: "cgu_0".to_string(),
                    item: "fn_main".to_string(),
                    current: 3,
                    total: 10
                }
            ))
        );

        let cached_str = "thread :: 1 [8/8 | 100%]: using cached cgu_misc_0";
        assert_eq!(
            parse_thread_status_line(cached_str),
            Some((
                1,
                ThreadStatus::Emitting {
                    cgu_name: "cgu_misc_0".to_string(),
                    current: 8,
                    total: 8,
                }
            ))
        );
    }

    #[test]
    fn test_thread_tracker_lifecycle() {
        let mut tracker = ThreadTracker::new(2);
        assert_eq!(tracker.len(), 2);
        assert!(tracker.all_idle());

        tracker.set_status(
            0,
            ThreadStatus::Codegen {
                cgu_name: "cgu_0".to_string(),
                item: "fn_test".to_string(),
                current: 1,
                total: 2,
            },
        );
        assert!(!tracker.all_idle());

        tracker.set_idle(0);
        assert!(tracker.all_idle());
    }
}
