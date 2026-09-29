//! One authenticated SSH session per host, optionally through jump hosts.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate};
use russh::{ChannelMsg, Disconnect, Preferred};
use tracing::debug;

use super::SshError;
use super::auth::{KeyStore, agent_keys, connect_agent};
use super::known_hosts::{self, HostKeyStatus};
use super::target::ResolvedHost;

/// Upper bound on captured stdout; protects against a misbehaving host.
const MAX_STDOUT: usize = 32 * 1024 * 1024;
const MAX_STDERR: usize = 64 * 1024;

#[derive(Clone)]
pub struct SshOptions {
    /// Record unknown host keys instead of refusing them
    /// (OpenSSH `StrictHostKeyChecking=accept-new`).
    pub accept_new: bool,
    pub connect_timeout: Duration,
    pub keys: Arc<KeyStore>,
    pub use_agent: bool,
    /// Replaces the known_hosts files from the SSH config (used by tests).
    pub known_hosts_override: Option<Vec<PathBuf>>,
    pub keepalive: Duration,
}

impl SshOptions {
    pub fn new(keys: Arc<KeyStore>) -> Self {
        Self {
            accept_new: false,
            connect_timeout: Duration::from_secs(10),
            keys,
            use_agent: true,
            known_hosts_override: None,
            keepalive: Duration::from_secs(15),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<u32>,
}

#[derive(Debug)]
pub enum HandlerError {
    Russh(russh::Error),
    HostKey(SshError),
}

impl From<russh::Error> for HandlerError {
    fn from(e: russh::Error) -> Self {
        HandlerError::Russh(e)
    }
}

struct Handler {
    name: String,
    port: u16,
    files: Vec<PathBuf>,
    accept_new: bool,
}

impl client::Handler for Handler {
    type Error = HandlerError;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key: PublicKey = server_key.public_key();
        let fingerprint = known_hosts::fingerprint(&key);
        match known_hosts::check(&self.files, &self.name, self.port, &key) {
            HostKeyStatus::Known => Ok(true),
            HostKeyStatus::Unknown if self.accept_new => {
                let file = self.files.first().cloned().unwrap_or_default();
                known_hosts::learn(&file, &self.name, self.port, &key).map_err(|e| {
                    HandlerError::HostKey(SshError::HostKeyWrite {
                        host: self.name.clone(),
                        reason: e.to_string(),
                    })
                })?;
                tracing::info!(host = %self.name, %fingerprint, "recorded new host key");
                Ok(true)
            }
            HostKeyStatus::Unknown => Err(HandlerError::HostKey(SshError::HostKeyUnknown {
                host: known_hosts::host_port_name(&self.name, self.port),
                fingerprint,
            })),
            HostKeyStatus::Changed { path, line } => {
                Err(HandlerError::HostKey(SshError::HostKeyChanged {
                    host: known_hosts::host_port_name(&self.name, self.port),
                    fingerprint,
                    path: path.display().to_string(),
                    line,
                }))
            }
            HostKeyStatus::Revoked => Err(HandlerError::HostKey(SshError::HostKeyRevoked {
                host: known_hosts::host_port_name(&self.name, self.port),
                fingerprint,
            })),
        }
    }
}

fn map_russh(e: russh::Error) -> SshError {
    match e {
        russh::Error::IO(io) => SshError::Connect {
            addr: String::new(),
            reason: io.to_string(),
        },
        russh::Error::Disconnect | russh::Error::HUP => SshError::Closed,
        other => SshError::Protocol(other.to_string()),
    }
}

fn map_handler(e: HandlerError, addr: &str) -> SshError {
    match e {
        HandlerError::HostKey(e) => e,
        HandlerError::Russh(russh::Error::IO(io)) => SshError::Connect {
            addr: addr.to_string(),
            reason: io.to_string(),
        },
        HandlerError::Russh(e) => map_russh(e),
    }
}

/// An authenticated connection. Jump host sessions are kept alive for as
/// long as the target session exists.
pub struct Session {
    handle: Handle<Handler>,
    _jumps: Vec<Handle<Handler>>,
    host: ResolvedHost,
}

impl Session {
    /// Connects and authenticates, through any configured jump hosts.
    pub async fn connect(host: &ResolvedHost, opts: &SshOptions) -> Result<Session, SshError> {
        let per_hop = host
            .connect_timeout
            .map(Duration::from_secs)
            .unwrap_or(opts.connect_timeout);
        let total = per_hop * (host.jumps.len() as u32 + 1);
        tokio::time::timeout(total, Self::connect_chain(host, opts))
            .await
            .map_err(|_| SshError::Timeout("connecting"))?
    }

    async fn connect_chain(host: &ResolvedHost, opts: &SshOptions) -> Result<Session, SshError> {
        let mut jumps: Vec<Handle<Handler>> = Vec::new();
        for j in &host.jumps {
            let h = hop(j, jumps.last(), opts)
                .await
                .map_err(|e| SshError::Jump {
                    jump: j.name.clone(),
                    source: Box::new(e),
                })?;
            jumps.push(h);
        }
        let handle = hop(host, jumps.last(), opts).await?;
        Ok(Session {
            handle,
            _jumps: jumps,
            host: host.clone(),
        })
    }

    pub fn host(&self) -> &ResolvedHost {
        &self.host
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    /// Runs one command and collects its output.
    pub async fn exec(&self, command: &str, timeout: Duration) -> Result<ExecOutput, SshError> {
        tokio::time::timeout(timeout, self.exec_inner(command))
            .await
            .map_err(|_| SshError::Timeout("running the collection command"))?
    }

    async fn exec_inner(&self, command: &str) -> Result<ExecOutput, SshError> {
        let mut ch = self
            .handle
            .channel_open_session()
            .await
            .map_err(map_russh)?;
        ch.exec(true, command).await.map_err(map_russh)?;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut status = None;
        while let Some(msg) = ch.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    if out.len() < MAX_STDOUT {
                        out.extend_from_slice(&data);
                    }
                }
                ChannelMsg::ExtendedData { data, ext: 1 } => {
                    if err.len() < MAX_STDERR {
                        err.extend_from_slice(&data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        Ok(ExecOutput {
            stdout: String::from_utf8_lossy(&out).into_owned(),
            stderr: String::from_utf8_lossy(&err).into_owned(),
            exit_status: status,
        })
    }

    pub async fn close(self) {
        let _ = self
            .handle
            .disconnect(Disconnect::ByApplication, "", "en")
            .await;
    }
}

async fn open_tcp(host: &ResolvedHost) -> Result<tokio::net::TcpStream, SshError> {
    let addr = host.display_addr();
    let addrs: Vec<_> = tokio::net::lookup_host((host.hostname.as_str(), host.port))
        .await
        .map_err(|e| SshError::Resolve {
            host: host.hostname.clone(),
            reason: e.to_string(),
        })?
        .collect();
    let mut last = None;
    for a in addrs {
        match tokio::net::TcpStream::connect(a).await {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                return Ok(s);
            }
            Err(e) => last = Some(e),
        }
    }
    Err(SshError::Connect {
        addr,
        reason: last
            .map(|e| e.to_string())
            .unwrap_or_else(|| "no addresses".into()),
    })
}

async fn hop(
    host: &ResolvedHost,
    via: Option<&Handle<Handler>>,
    opts: &SshOptions,
) -> Result<Handle<Handler>, SshError> {
    let files = opts
        .known_hosts_override
        .clone()
        .unwrap_or_else(|| host.known_hosts_files.clone());

    // Prefer key types already recorded for this host, like OpenSSH.
    let known = known_hosts::known_algorithms(&files, &host.known_hosts_name, host.port);
    let mut preferred = Preferred::default();
    if !known.is_empty() {
        let mut order: Vec<_> = preferred
            .key
            .iter()
            .filter(|a| known.contains(a))
            .cloned()
            .collect();
        order.extend(preferred.key.iter().filter(|a| !known.contains(a)).cloned());
        preferred.key = order.into();
    }
    let config = Arc::new(client::Config {
        preferred,
        keepalive_interval: Some(opts.keepalive),
        keepalive_max: 3,
        inactivity_timeout: None,
        nodelay: true,
        ..Default::default()
    });
    let handler = Handler {
        name: host.known_hosts_name.clone(),
        port: host.port,
        files,
        accept_new: opts.accept_new,
    };
    let addr = host.display_addr();
    let mut handle = match via {
        None => {
            let stream = open_tcp(host).await?;
            client::connect_stream(config, stream, handler)
                .await
                .map_err(|e| map_handler(e, &addr))?
        }
        Some(jump) => {
            let ch = jump
                .channel_open_direct_tcpip(host.hostname.clone(), host.port as u32, "127.0.0.1", 0)
                .await
                .map_err(|e| SshError::Connect {
                    addr: addr.clone(),
                    reason: format!("jump host could not open a tunnel: {e}"),
                })?;
            client::connect_stream(config, ch.into_stream(), handler)
                .await
                .map_err(|e| map_handler(e, &addr))?
        }
    };
    authenticate(&mut handle, host, opts).await?;
    debug!(host = %addr, "authenticated");
    Ok(handle)
}

async fn authenticate(
    handle: &mut Handle<Handler>,
    host: &ResolvedHost,
    opts: &SshOptions,
) -> Result<(), SshError> {
    let fail = |detail: String| SshError::AuthFailed {
        user: host.user.clone(),
        host: host.display_addr(),
        detail,
    };
    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();
    let file_publics = opts.keys.public_keys(&host.identity_files);
    let mut tried: Vec<PublicKey> = Vec::new();

    let mut agent = if opts.use_agent {
        connect_agent().await
    } else {
        None
    };
    if let Some(agent) = agent.as_mut() {
        let keys = agent_keys(agent).await;
        let (matching, others): (Vec<_>, Vec<_>) = keys
            .into_iter()
            .partition(|k| file_publics.iter().any(|f| f.key_data() == k.key_data()));
        let mut order = matching;
        if !host.identities_only {
            order.extend(others);
        }
        for key in order {
            let hash = if key.algorithm().is_rsa() {
                rsa_hash
            } else {
                None
            };
            match handle
                .authenticate_publickey_with(host.user.clone(), key.clone(), hash, agent)
                .await
            {
                Ok(r) if r.success() => return Ok(()),
                Ok(_) => tried.push(key),
                Err(e) => {
                    debug!(error = %e, "agent signing failed");
                    tried.push(key);
                    if handle.is_closed() {
                        return Err(fail(format!(" (server closed after {} keys)", tried.len())));
                    }
                }
            }
        }
    }

    for (path, key) in opts.keys.ready_keys(&host.identity_files) {
        let public = key.public_key().clone();
        if tried.iter().any(|k| k.key_data() == public.key_data()) {
            continue;
        }
        tried.push(public);
        let hash = if key.algorithm().is_rsa() {
            rsa_hash
        } else {
            None
        };
        match handle
            .authenticate_publickey(host.user.clone(), PrivateKeyWithHashAlg::new(key, hash))
            .await
        {
            Ok(r) if r.success() => return Ok(()),
            Ok(_) => debug!(key = %path.display(), "key rejected"),
            Err(e) => {
                return Err(fail(format!(
                    " (server closed the connection after {} keys: {e})",
                    tried.len()
                )));
            }
        }
    }

    let mut detail = if tried.is_empty() {
        " (no usable keys: start ssh-agent or set IdentityFile)".to_string()
    } else {
        format!(" (tried {} keys)", tried.len())
    };
    let problems = opts.keys.problems(&host.identity_files);
    if !problems.is_empty() {
        detail.push_str(&format!("; {}", problems.join(", ")));
    }
    Err(fail(detail))
}
