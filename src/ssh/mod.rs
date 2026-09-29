//! SSH connection management: configuration, host key verification,
//! authentication, ProxyJump, and reconnection with exponential backoff.

pub mod auth;
pub mod known_hosts;
pub mod session;
pub mod sshconfig;
pub mod target;

use std::time::Duration;

use serde::Serialize;
use thiserror::Error;

pub use session::{ExecOutput, Session, SshOptions};
pub use target::{ResolvedHost, resolve};

/// Broad class of a connection failure, for display and alerting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Auth,
    HostKey,
    Timeout,
    Network,
    Protocol,
    Config,
}

impl FailureKind {
    pub fn label(self) -> &'static str {
        match self {
            FailureKind::Auth => "auth failed",
            FailureKind::HostKey => "host key",
            FailureKind::Timeout => "timeout",
            FailureKind::Network => "unreachable",
            FailureKind::Protocol => "ssh error",
            FailureKind::Config => "config",
        }
    }
}

#[derive(Debug, Error)]
pub enum SshError {
    #[error("{0}")]
    Config(String),
    #[error("cannot resolve {host}: {reason}")]
    Resolve { host: String, reason: String },
    #[error("cannot connect to {addr}: {reason}")]
    Connect { addr: String, reason: String },
    #[error("timed out while {0}")]
    Timeout(&'static str),
    #[error(
        "unknown host key for {host} ({fingerprint}); verify it, then rerun with --accept-new or add it to known_hosts"
    )]
    HostKeyUnknown { host: String, fingerprint: String },
    #[error(
        "HOST KEY MISMATCH for {host}: server offered {fingerprint}, which differs from {path}:{line}; possible man-in-the-middle, refusing to connect"
    )]
    HostKeyChanged {
        host: String,
        fingerprint: String,
        path: String,
        line: usize,
    },
    #[error("host key for {host} ({fingerprint}) is marked @revoked")]
    HostKeyRevoked { host: String, fingerprint: String },
    #[error("cannot record host key for {host}: {reason}")]
    HostKeyWrite { host: String, reason: String },
    #[error("authentication failed for {user}@{host}{detail}")]
    AuthFailed {
        user: String,
        host: String,
        detail: String,
    },
    #[error("via jump host {jump}: {source}")]
    Jump {
        jump: String,
        #[source]
        source: Box<SshError>,
    },
    #[error("ssh: {0}")]
    Protocol(String),
    #[error("connection closed")]
    Closed,
}

impl SshError {
    pub fn kind(&self) -> FailureKind {
        match self {
            SshError::Config(_) => FailureKind::Config,
            SshError::Resolve { .. } | SshError::Connect { .. } | SshError::Closed => {
                FailureKind::Network
            }
            SshError::Timeout(_) => FailureKind::Timeout,
            SshError::HostKeyUnknown { .. }
            | SshError::HostKeyChanged { .. }
            | SshError::HostKeyRevoked { .. }
            | SshError::HostKeyWrite { .. } => FailureKind::HostKey,
            SshError::AuthFailed { .. } => FailureKind::Auth,
            SshError::Jump { source, .. } => source.kind(),
            SshError::Protocol(_) => FailureKind::Protocol,
        }
    }

    /// Whether retrying soon could help. Host key and config problems need
    /// the user; they are retried only at the maximum backoff.
    pub fn is_persistent(&self) -> bool {
        matches!(self.kind(), FailureKind::HostKey | FailureKind::Config)
    }
}

/// Exponential backoff with a cap and jitter-free determinism (tests rely
/// on exact values).
#[derive(Debug, Clone)]
pub struct Backoff {
    base: Duration,
    max: Duration,
    attempt: u32,
}

impl Backoff {
    pub fn new(base: Duration, max: Duration) -> Self {
        Self {
            base,
            max,
            attempt: 0,
        }
    }

    /// Delay before the next attempt; grows 1×, 2×, 4×… up to `max`.
    pub fn next_delay(&mut self) -> Duration {
        let factor = 1u32.checked_shl(self.attempt.min(16)).unwrap_or(u32::MAX);
        self.attempt = self.attempt.saturating_add(1);
        self.base.saturating_mul(factor).min(self.max)
    }

    pub fn max_delay(&self) -> Duration {
        self.max
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        let d: Vec<u64> = (0..7).map(|_| b.next_delay().as_secs()).collect();
        assert_eq!(d, vec![1, 2, 4, 8, 16, 30, 30]);
        b.reset();
        assert_eq!(b.next_delay().as_secs(), 1);
        for _ in 0..100 {
            b.next_delay();
        }
        assert_eq!(b.next_delay().as_secs(), 30);
    }

    #[test]
    fn jump_errors_keep_inner_kind() {
        let e = SshError::Jump {
            jump: "bastion".into(),
            source: Box::new(SshError::AuthFailed {
                user: "u".into(),
                host: "h".into(),
                detail: String::new(),
            }),
        };
        assert_eq!(e.kind(), FailureKind::Auth);
        assert!(
            e.to_string()
                .starts_with("via jump host bastion: authentication failed")
        );
    }
}
