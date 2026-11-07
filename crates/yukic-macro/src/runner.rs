use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;
use yukino_macroproto::{MacroRequest, MacroResponse, read_response, write_request};

pub const DEFAULT_MACRO_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone)]
pub struct MacroRunnerClient {
    pub timeout: Duration,
}

impl MacroRunnerClient {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_MACRO_TIMEOUT_SECS),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn execute(
        &self,
        runner_path: &Path,
        req: &MacroRequest,
        span: Span,
    ) -> Result<MacroResponse, Diagnostic> {
        let runner_name = runner_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let mut child = Command::new(runner_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Diagnostic::error(
                    ErrorCode::E0306,
                    format!("failed to spawn macro runner '{runner_name}': {e}"),
                    span,
                )
            })?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            Diagnostic::error(
                ErrorCode::E0306,
                format!("failed to open stdin for macro runner '{runner_name}'"),
                span,
            )
        })?;

        write_request(&mut stdin, req).map_err(|e| {
            Diagnostic::error(
                ErrorCode::E0306,
                format!("failed to send request to macro runner '{runner_name}': {e}"),
                span,
            )
        })?;
        drop(stdin);

        let mut stdout = child.stdout.take().ok_or_else(|| {
            Diagnostic::error(
                ErrorCode::E0306,
                format!("failed to capture stdout from macro runner '{runner_name}'"),
                span,
            )
        })?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let res = read_response(&mut stdout);
            let _ = tx.send(res);
        });

        match rx.recv_timeout(self.timeout) {
            Ok(Ok(response)) => {
                let _ = child.wait();
                Ok(response)
            }
            Ok(Err(e)) => {
                let stderr_text = Self::read_child_stderr(&mut child);
                let msg = if stderr_text.is_empty() {
                    format!("macro runner '{runner_name}' failed to deliver response: {e}")
                } else {
                    format!("macro runner '{runner_name}' failed: {stderr_text}")
                };
                Err(Diagnostic::error(ErrorCode::E0306, msg, span))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                let _ = child.wait();
                let secs = self.timeout.as_secs();
                Err(Diagnostic::error(
                    ErrorCode::E0306,
                    format!("macro runner '{runner_name}' timed out after {secs}s"),
                    span,
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let stderr_text = Self::read_child_stderr(&mut child);
                let msg = if stderr_text.is_empty() {
                    format!("macro runner '{runner_name}' disconnected unexpectedly")
                } else {
                    format!("macro runner '{runner_name}' crashed: {stderr_text}")
                };
                Err(Diagnostic::error(ErrorCode::E0306, msg, span))
            }
        }
    }

    fn read_child_stderr(child: &mut std::process::Child) -> String {
        let mut err = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut err);
        }
        let _ = child.wait();
        err.trim().to_string()
    }
}

impl Default for MacroRunnerClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId};
    use yukino_macroproto::MacroKind;

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    #[test]
    fn test_macro_runner_client_init() {
        let client = MacroRunnerClient::new();
        assert_eq!(client.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_execute_missing_binary() {
        let client = MacroRunnerClient::new();
        let req = MacroRequest::new(
            "test_macro",
            MacroKind::FunctionLike,
            "1 + 2",
            None,
            "x86_64-unknown-linux-gnu",
            "debug",
            None,
        );
        let res = client.execute(Path::new("/non/existent/path/to/runner"), &req, dummy_span());
        assert!(res.is_err());
        assert!(res.unwrap_err().message.contains("failed to spawn macro runner"));
    }
}
