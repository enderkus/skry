//! One-shot collection for the query and report commands (`--once`,
//! `find`, `security`, `snapshot`).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Semaphore;

use super::collector::collect;
use super::{ConnState, HostError, HostPlan, HostState};
use crate::collect::{RawSample, Section};
use crate::config::SecurityConfig;
use crate::model::{Health, HostModel, Thresholds};
use crate::security::host_findings;
use crate::ssh::{FailureKind, Session, SshOptions};

#[derive(Debug, Clone)]
pub struct OnceOptions {
    pub sections: Vec<Section>,
    /// Two samples are needed for rates (CPU, network, disk I/O).
    pub samples: u32,
    pub gap: Duration,
    pub concurrency: usize,
    pub command_timeout: Duration,
    pub thresholds: Thresholds,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone)]
pub struct OnceResult {
    pub state: HostState,
    /// The last raw sample, for commands that need sections the model does
    /// not keep (process arguments, unit status).
    pub raw: Option<RawSample>,
}

async fn one(
    plan: HostPlan,
    ssh: SshOptions,
    opts: Arc<OnceOptions>,
    sem: Arc<Semaphore>,
) -> OnceResult {
    let addr = plan.resolved.as_ref().ok().map(|r| r.display_addr());
    let mut state = HostState::new(&plan.target, addr);
    let fail = |mut state: HostState, kind, message: String| {
        state.conn = ConnState::Failed;
        state.error = Some(HostError {
            kind,
            message,
            since: Utc::now(),
        });
        state.refresh_status();
        OnceResult { state, raw: None }
    };
    let host = match plan.resolved {
        Ok(h) => h,
        Err(e) => return fail(state, FailureKind::Config, e),
    };
    let _permit = sem.acquire().await;
    let session = match Session::connect(&host, &ssh).await {
        Ok(s) => s,
        Err(e) => return fail(state, e.kind(), e.to_string()),
    };
    let mut model = HostModel::new();
    let mut raw = None;
    for i in 0..opts.samples.max(1) {
        if i > 0 {
            tokio::time::sleep(opts.gap).await;
        }
        // Slow sections only need to be read once.
        let sections: Vec<Section> = if i + 1 == opts.samples.max(1) {
            opts.sections.clone()
        } else {
            opts.sections
                .iter()
                .filter(|s| Section::fast().contains(s))
                .cloned()
                .collect()
        };
        match collect(&session, &sections, opts.command_timeout).await {
            Ok(r) => {
                let m = model.update(&r, Utc::now());
                state.metrics = Some(m);
                raw = Some(r);
            }
            Err(e) => {
                session.close().await;
                return fail(state, e.kind(), e.to_string());
            }
        }
    }
    session.close().await;
    state.conn = ConnState::Connected;
    state.last_update = Some(Utc::now());
    if let Some(m) = &state.metrics {
        state.health = Health::evaluate(m, &opts.thresholds);
        let allowed: Option<BTreeSet<u16>> = state
            .allowed_ports
            .as_ref()
            .map(|p| p.iter().copied().collect());
        state.findings = host_findings(m, allowed.as_ref(), &opts.security);
    }
    state.refresh_status();
    OnceResult { state, raw }
}

/// Connects to every host (bounded by `concurrency`), collects, and
/// returns results in the order of `plans`.
pub async fn collect_once(
    plans: Vec<HostPlan>,
    ssh: SshOptions,
    opts: OnceOptions,
) -> Vec<OnceResult> {
    let sem = Arc::new(Semaphore::new(opts.concurrency.max(1)));
    let opts = Arc::new(opts);
    let tasks: Vec<_> = plans
        .into_iter()
        .map(|p| tokio::spawn(one(p, ssh.clone(), opts.clone(), sem.clone())))
        .collect();
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        if let Ok(r) = t.await {
            out.push(r);
        }
    }
    out
}
