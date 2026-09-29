//! Shared setup: configuration, target resolution, keys and SSH options.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};

use skry::config::{Config, Target};
use skry::engine::HostPlan;
use skry::ssh::auth::{
    KeyStore, NoPrompt, PassphrasePrompt, TerminalPrompt, agent_keys, connect_agent,
};
use skry::ssh::sshconfig::SshConfig;
use skry::ssh::{ResolvedHost, SshOptions, resolve};

use super::Global;

pub struct Prepared {
    pub config: Config,
    pub targets: Vec<Target>,
    pub plans: Vec<HostPlan>,
    pub ssh: SshOptions,
}

pub fn home_dir() -> anyhow::Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .context("cannot determine the home directory")
}

pub fn load_config(global: &Global) -> anyhow::Result<Config> {
    let mut config = Config::load(global.config.as_deref())?;
    if let Some(i) = global.interval {
        config.interval = i;
    }
    if let Some(c) = global.concurrency {
        config.concurrency = c;
    }
    if global.no_history {
        config.history.enabled = false;
    }
    config.validate()?;
    Ok(config)
}

fn collect_identity_files(host: &ResolvedHost, out: &mut BTreeSet<PathBuf>) {
    out.extend(host.identity_files.iter().cloned());
    for j in &host.jumps {
        collect_identity_files(j, out);
    }
}

/// Loads config, expands targets, resolves them against the SSH config and
/// loads keys (prompting for passphrases before any UI starts).
pub async fn prepare(global: &Global, args: &[String]) -> anyhow::Result<Prepared> {
    let config = load_config(global)?;
    let targets = config.resolve_targets(args)?;
    if targets.is_empty() {
        bail!("no hosts given (try `skry web1` or `skry @group`)");
    }
    let home = home_dir()?;
    let ssh_config = SshConfig::load(global.ssh_config.as_deref(), &home)
        .context("cannot read the SSH client config")?;
    let plans: Vec<HostPlan> = targets
        .iter()
        .map(|t| HostPlan {
            target: t.clone(),
            resolved: resolve(&t.spec, &ssh_config).map_err(|e| e.to_string()),
        })
        .collect();

    let mut agent_public = Vec::new();
    if !global.no_agent
        && let Some(mut agent) = connect_agent().await
    {
        agent_public = agent_keys(&mut agent).await;
    }
    let mut files = BTreeSet::new();
    for p in &plans {
        if let Ok(h) = &p.resolved {
            collect_identity_files(h, &mut files);
        }
    }
    let files: Vec<PathBuf> = files.into_iter().collect();
    let keys = Arc::new(KeyStore::new());
    let prompt: Box<dyn PassphrasePrompt> = if std::io::stdin().is_terminal() {
        Box::new(TerminalPrompt)
    } else {
        Box::new(NoPrompt)
    };
    let keys_for_prompt = keys.clone();
    tokio::task::spawn_blocking(move || {
        keys_for_prompt.prepare(&files, &agent_public, prompt.as_ref())
    })
    .await
    .context("key loading failed")?;

    let mut ssh = SshOptions::new(keys);
    ssh.accept_new = global.accept_new;
    ssh.use_agent = !global.no_agent;
    ssh.connect_timeout = Duration::from_secs_f64(config.connect_timeout.max(1.0));
    Ok(Prepared {
        config,
        targets,
        plans,
        ssh,
    })
}
