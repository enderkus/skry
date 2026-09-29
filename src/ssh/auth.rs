//! Client authentication material: ssh-agent identities and key files.
//!
//! Key files are loaded once per run. Passphrase-protected keys are
//! decrypted up front (prompting on the terminal) unless the agent already
//! holds them, so that the TUI never has to stop for input. Passphrases are
//! used once and never stored or logged.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{PrivateKey, PublicKey};

pub type DynAgent = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

/// Asks the user for a key passphrase. Returns `None` to skip the key.
pub trait PassphrasePrompt: Send + Sync {
    fn prompt(&self, path: &Path, attempt: u32) -> Option<String>;
}

/// Prompts on the controlling terminal.
pub struct TerminalPrompt;

impl PassphrasePrompt for TerminalPrompt {
    fn prompt(&self, path: &Path, attempt: u32) -> Option<String> {
        let msg = if attempt == 0 {
            format!("Enter passphrase for key '{}': ", path.display())
        } else {
            format!("Bad passphrase, try again for '{}': ", path.display())
        };
        rpassword::prompt_password(msg)
            .ok()
            .filter(|p| !p.is_empty())
    }
}

/// Never prompts (non-interactive use).
pub struct NoPrompt;

impl PassphrasePrompt for NoPrompt {
    fn prompt(&self, _: &Path, _: u32) -> Option<String> {
        None
    }
}

#[derive(Clone)]
enum KeyState {
    Ready(Arc<PrivateKey>),
    /// Encrypted and either held by the agent or skipped by the user.
    Locked(Option<PublicKey>),
    Unusable(String),
}

/// Loaded identity files, shared by all connections.
#[derive(Default)]
pub struct KeyStore {
    keys: Mutex<HashMap<PathBuf, KeyState>>,
}

/// Connects to the user's ssh-agent, if any.
pub async fn connect_agent() -> Option<DynAgent> {
    #[cfg(unix)]
    {
        std::env::var_os("SSH_AUTH_SOCK")?;
        AgentClient::connect_env().await.ok().map(|a| a.dynamic())
    }
    #[cfg(windows)]
    {
        if let Ok(a) = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
            return Some(a.dynamic());
        }
        AgentClient::connect_pageant()
            .await
            .ok()
            .map(|a| a.dynamic())
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Public keys currently offered by the agent.
pub async fn agent_keys(agent: &mut DynAgent) -> Vec<PublicKey> {
    agent
        .request_identities()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| match id {
            AgentIdentity::PublicKey { key, .. } => Some(key),
            AgentIdentity::Certificate { .. } => None,
        })
        .collect()
}

fn read_public(path: &Path) -> Option<PublicKey> {
    let mut pub_path = path.as_os_str().to_owned();
    pub_path.push(".pub");
    russh::keys::load_public_key(PathBuf::from(pub_path)).ok()
}

impl KeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads every file in `paths` that is not loaded yet. Encrypted keys
    /// not held by the agent are decrypted with a passphrase from `prompt`.
    pub fn prepare(
        &self,
        paths: &[PathBuf],
        agent_keys: &[PublicKey],
        prompt: &dyn PassphrasePrompt,
    ) {
        for path in paths {
            if self.keys.lock().expect("keystore lock").contains_key(path) {
                continue;
            }
            let state = load_key(path, agent_keys, prompt);
            self.keys
                .lock()
                .expect("keystore lock")
                .insert(path.clone(), state);
        }
    }

    /// Decrypted keys for the given files, in order.
    pub fn ready_keys(&self, paths: &[PathBuf]) -> Vec<(PathBuf, Arc<PrivateKey>)> {
        let keys = self.keys.lock().expect("keystore lock");
        paths
            .iter()
            .filter_map(|p| match keys.get(p) {
                Some(KeyState::Ready(k)) => Some((p.clone(), k.clone())),
                _ => None,
            })
            .collect()
    }

    /// Public keys for the given files, whether decrypted or not.
    pub fn public_keys(&self, paths: &[PathBuf]) -> Vec<PublicKey> {
        let keys = self.keys.lock().expect("keystore lock");
        paths
            .iter()
            .filter_map(|p| match keys.get(p) {
                Some(KeyState::Ready(k)) => Some(k.public_key().clone()),
                Some(KeyState::Locked(pk)) => pk.clone(),
                _ => None,
            })
            .collect()
    }

    /// Why keys could not be used, for error messages.
    pub fn problems(&self, paths: &[PathBuf]) -> Vec<String> {
        let keys = self.keys.lock().expect("keystore lock");
        paths
            .iter()
            .filter_map(|p| match keys.get(p) {
                Some(KeyState::Unusable(e)) => Some(format!("{}: {e}", p.display())),
                _ => None,
            })
            .collect()
    }
}

fn load_key(path: &Path, agent_keys: &[PublicKey], prompt: &dyn PassphrasePrompt) -> KeyState {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return KeyState::Unusable(e.kind().to_string()),
    };
    match russh::keys::decode_secret_key(&text, None) {
        Ok(k) => return KeyState::Ready(Arc::new(k)),
        Err(russh::keys::Error::KeyIsEncrypted) => {}
        Err(e) => return KeyState::Unusable(e.to_string()),
    }
    // OpenSSH-format keys keep the public half in clear text.
    let public = PrivateKey::from_openssh(text.as_bytes())
        .ok()
        .map(|k| k.public_key().clone())
        .or_else(|| read_public(path));
    if let Some(pk) = &public
        && agent_keys.iter().any(|a| a.key_data() == pk.key_data())
    {
        return KeyState::Locked(public);
    }
    for attempt in 0..3 {
        let Some(pass) = prompt.prompt(path, attempt) else {
            return KeyState::Locked(public);
        };
        if let Ok(k) = russh::keys::decode_secret_key(&text, Some(&pass)) {
            return KeyState::Ready(Arc::new(k));
        }
    }
    KeyState::Unusable("wrong passphrase".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPrompt(&'static str, std::sync::atomic::AtomicU32);

    impl PassphrasePrompt for FixedPrompt {
        fn prompt(&self, _: &Path, _: u32) -> Option<String> {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(self.0.to_string())
        }
    }

    fn keygen(dir: &Path, name: &str, pass: &str) -> Option<PathBuf> {
        let p = dir.join(name);
        let ok = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", pass, "-C", "", "-f"])
            .arg(&p)
            .status()
            .ok()?
            .success();
        ok.then_some(p)
    }

    #[test]
    fn plain_and_encrypted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let (Some(plain), Some(enc)) = (
            keygen(dir.path(), "plain", ""),
            keygen(dir.path(), "enc", "s3cret"),
        ) else {
            eprintln!("ssh-keygen not available; skipping");
            return;
        };
        let store = KeyStore::new();
        let prompt = FixedPrompt("s3cret", Default::default());
        store.prepare(&[plain.clone(), enc.clone()], &[], &prompt);
        assert_eq!(store.ready_keys(&[plain.clone(), enc.clone()]).len(), 2);
        assert_eq!(prompt.1.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Wrong passphrase three times: unusable, with a reason.
        let store = KeyStore::new();
        store.prepare(
            std::slice::from_ref(&enc),
            &[],
            &FixedPrompt("nope", Default::default()),
        );
        assert!(store.ready_keys(std::slice::from_ref(&enc)).is_empty());
        assert_eq!(store.problems(std::slice::from_ref(&enc)).len(), 1);
        // The public key is still known (from the OpenSSH container).
        assert_eq!(store.public_keys(std::slice::from_ref(&enc)).len(), 0);
    }

    #[test]
    fn agent_held_key_is_not_prompted() {
        let dir = tempfile::tempdir().unwrap();
        let Some(enc) = keygen(dir.path(), "enc", "s3cret") else {
            return;
        };
        let public = read_public(&enc).unwrap();
        let store = KeyStore::new();
        let prompt = FixedPrompt("s3cret", Default::default());
        store.prepare(std::slice::from_ref(&enc), &[public], &prompt);
        assert_eq!(prompt.1.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(store.ready_keys(std::slice::from_ref(&enc)).is_empty());
        assert_eq!(store.public_keys(&[enc]).len(), 1);
    }

    #[test]
    fn missing_file() {
        let store = KeyStore::new();
        let p = PathBuf::from("/nonexistent/id_ed25519");
        store.prepare(std::slice::from_ref(&p), &[], &NoPrompt);
        assert_eq!(store.problems(&[p]).len(), 1);
    }
}
