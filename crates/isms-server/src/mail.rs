//! Magic-link delivery behind a trait (TDD T16). Phase 1 dev and tests use
//! the stdout and memory senders; the SMTP sender arrives with the deploy
//! card (S1.14) once Chris picks a provider.

use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail sender {0:?} is not configured (S1.14)")]
    NotConfigured(String),
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

/// Pick a sender from `ISMS_MAIL_SENDER` (`stdout` by default).
pub fn from_env() -> Result<Box<dyn MailSender>, MailError> {
    match std::env::var("ISMS_MAIL_SENDER").as_deref() {
        Ok("stdout") | Err(_) => Ok(Box::new(StdoutSender)),
        Ok(other) => Err(MailError::NotConfigured(other.to_owned())),
    }
}
