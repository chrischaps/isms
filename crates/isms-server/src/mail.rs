//! Magic-link delivery behind a trait (TDD T16). Phase 1 dev and tests use
//! the stdout, file, and memory senders; the SMTP sender arrives with the
//! deploy card (S1.14) once Chris picks a provider.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail sender {0:?} is not configured (S1.14)")]
    NotConfigured(String),
    #[error("mail file {path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub trait MailSender: Send + Sync + std::fmt::Debug {
    fn send_magic_link(&self, to: &str, url: &str) -> Result<(), MailError>;
}

/// Prints the link; the dev server's "inbox".
#[derive(Debug, Default)]
pub struct StdoutSender;

impl MailSender for StdoutSender {
    fn send_magic_link(&self, to: &str, url: &str) -> Result<(), MailError> {
        tracing::info!(to, url, "magic link");
        println!("magic link for {to}: {url}");
        Ok(())
    }
}

/// Appends `to<TAB>url` to a file (and prints, like [`StdoutSender`]): a dev
/// mailbox that a script can read after the window has scrolled on.
#[derive(Debug)]
pub struct FileSender {
    pub path: PathBuf,
}

impl MailSender for FileSender {
    fn send_magic_link(&self, to: &str, url: &str) -> Result<(), MailError> {
        let io = |source| MailError::File {
            path: self.path.clone(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(io)?;
        writeln!(file, "{to}\t{url}").map_err(io)?;
        StdoutSender.send_magic_link(to, url)
    }
}

/// Keeps every link for tests to read back.
#[derive(Debug, Default)]
pub struct MemorySender {
    pub sent: Mutex<Vec<(String, String)>>,
}

impl MemorySender {
    /// The most recent link sent to `to`.
    #[must_use]
    pub fn last_for(&self, to: &str) -> Option<String> {
        self.sent
            .lock()
            .expect("mail mutex")
            .iter()
            .rev()
            .find(|(t, _)| t == to)
            .map(|(_, u)| u.clone())
    }
}

impl MailSender for MemorySender {
    fn send_magic_link(&self, to: &str, url: &str) -> Result<(), MailError> {
        self.sent
            .lock()
            .expect("mail mutex")
            .push((to.to_owned(), url.to_owned()));
        Ok(())
    }
}

/// Pick a sender from `ISMS_MAIL_SENDER` (`stdout` by default; `file` appends
/// to `ISMS_MAIL_FILE`).
pub fn from_env() -> Result<Box<dyn MailSender>, MailError> {
    match std::env::var("ISMS_MAIL_SENDER").as_deref() {
        Ok("stdout") | Err(_) => Ok(Box::new(StdoutSender)),
        Ok("file") => match std::env::var("ISMS_MAIL_FILE") {
            Ok(path) if !path.trim().is_empty() => Ok(Box::new(FileSender { path: path.into() })),
            _ => Err(MailError::NotConfigured(
                "file (ISMS_MAIL_FILE is not set)".to_owned(),
            )),
        },
        Ok(other) => Err(MailError::NotConfigured(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_sender_appends_one_line_per_link() {
        let dir = std::env::temp_dir().join(format!("isms-mail-{}", std::process::id()));
        let path = dir.join("nested").join("mail.log");
        let sender = FileSender { path: path.clone() };
        sender
            .send_magic_link("a@example.com", "http://x/auth/callback?token=1")
            .unwrap();
        sender
            .send_magic_link("b@example.com", "http://x/auth/callback?token=2")
            .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "a@example.com\thttp://x/auth/callback?token=1\nb@example.com\thttp://x/auth/callback?token=2\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
