//! Process stdout/stderr capture.

use gag::BufferRedirect;
use std::io::{self, Read, Write};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureOptions {
    pub stdout: bool,
    pub stderr: bool,
}

impl CaptureOptions {
    pub const ALL: Self = Self {
        stdout: true,
        stderr: true,
    };
}

pub struct ConsoleCapture {
    stdout: Option<BufferRedirect>,
    stderr: Option<BufferRedirect>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapturedOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CapturedOutput {
    #[must_use]
    pub fn stdout_lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[must_use]
    pub fn stderr_lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.stderr)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[must_use]
    pub fn contains_stdout(&self, value: &str) -> bool {
        String::from_utf8_lossy(&self.stdout).contains(value)
    }

    #[must_use]
    pub fn contains_stderr(&self, value: &str) -> bool {
        String::from_utf8_lossy(&self.stderr).contains(value)
    }
}

impl ConsoleCapture {
    pub fn begin(options: CaptureOptions) -> io::Result<Self> {
        Ok(Self {
            stdout: options.stdout.then(BufferRedirect::stdout).transpose()?,
            stderr: options.stderr.then(BufferRedirect::stderr).transpose()?,
        })
    }

    pub fn finish(mut self) -> io::Result<CapturedOutput> {
        io::stdout().flush()?;
        io::stderr().flush()?;
        let mut output = CapturedOutput::default();
        if let Some(stdout) = self.stdout.as_mut() {
            stdout.read_to_end(&mut output.stdout)?;
        }
        if let Some(stderr) = self.stderr.as_mut() {
            stderr.read_to_end(&mut output.stderr)?;
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureOptions, ConsoleCapture};
    use std::io::Write;

    #[test]
    fn captures_stdout_and_stderr() {
        let capture = ConsoleCapture::begin(CaptureOptions::ALL).unwrap();
        writeln!(std::io::stdout(), "captured stdout").unwrap();
        writeln!(std::io::stderr(), "captured stderr").unwrap();
        let output = capture.finish().unwrap();
        assert!(output.contains_stdout("captured stdout"));
        assert!(output.contains_stderr("captured stderr"));
    }
}
