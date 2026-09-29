//! Per-host collection task.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::time::MissedTickBehavior;

use super::{Event, HostError, HostPlan, Settings};
use crate::collect::{RawSample, Section, script};
use crate::model::HostModel;
use crate::ssh::{Backoff, FailureKind, Session, SshError, SshOptions};

pub(crate) struct Collector {
    pub idx: usize,
    pub plan: HostPlan,
    pub ssh: SshOptions,
    pub settings: Settings,
    pub semaphore: Arc<Semaphore>,
    pub events: mpsc::Sender<Event>,
    pub shutdown: watch::Receiver<bool>,
}

/// Runs one collection command and parses it. Treats output without any
/// recognisable section as an error (e.g. a restricted shell).
pub(crate) async fn collect(
    session: &Session,
    sections: &[Section],
    timeout: Duration,
) -> Result<RawSample, SshError> {
    let nonce = script::new_nonce();
    let cmd = script::build(sections, &nonce);
    let out = session.exec(&cmd, timeout).await?;
    let raw = RawSample::parse(&out.stdout, &nonce);
    if raw.identity.is_none() && raw.stat.is_none() && !raw.complete {
        let hint: String = out
            .stderr
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect();
        return Err(SshError::Protocol(if hint.is_empty() {
            "remote command produced no usable output".into()
        } else {
            format!("remote command failed: {hint}")
        }));
    }
    Ok(raw)
}

impl Collector {
    async fn send(&self, ev: Event) {
        let _ = self.events.send(ev).await;
    }

    async fn fail(&self, e: &SshError) {
        tracing::debug!(host = %self.plan.target.spec, error = %e, "collection failed");
        self.send(Event::Failed(self.idx, HostError::from_ssh(e, Utc::now())))
            .await;
    }

    /// Sleeps for `d`, returning false if shutdown was requested.
    async fn sleep(&mut self, d: Duration) -> bool {
        tokio::select! {
            _ = tokio::time::sleep(d) => true,
            _ = self.shutdown.changed() => false,
        }
    }

    pub async fn run(mut self) {
        let host = match self.plan.resolved.clone() {
            Ok(h) => h,
            Err(e) => {
                let err = HostError {
                    kind: FailureKind::Config,
                    message: e,
                    since: Utc::now(),
                };
                self.send(Event::Failed(self.idx, err)).await;
                return;
            }
        };
        let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(60));
        let mut model = HostModel::new();
        let mut session: Option<Session> = None;
        let mut last_slow: Option<Instant> = None;
        let mut ticker = tokio::time::interval(self.settings.interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if *self.shutdown.borrow() {
                break;
            }
            if session.as_ref().is_none_or(Session::is_closed) {
                session = None;
                self.send(Event::Connecting(self.idx)).await;
                let result = {
                    let _permit = self.semaphore.acquire().await;
                    Session::connect(&host, &self.ssh).await
                };
                match result {
                    Ok(s) => {
                        backoff.reset();
                        session = Some(s);
                        ticker.reset_immediately();
                    }
                    Err(e) => {
                        self.fail(&e).await;
                        let delay = if e.is_persistent() {
                            backoff.max_delay()
                        } else {
                            backoff.next_delay()
                        };
                        if !self.sleep(delay).await {
                            break;
                        }
                        continue;
                    }
                }
            }

            tokio::select! {
                _ = ticker.tick() => {}
                _ = self.shutdown.changed() => break,
            }
            let Some(s) = session.as_ref() else { continue };

            let slow_due = last_slow.is_none_or(|t| t.elapsed() >= self.settings.slow_interval);
            let mut sections = Section::fast();
            if slow_due {
                sections.extend(Section::slow(self.settings.security.rpm_updates));
            }
            let result = {
                let _permit = self.semaphore.acquire().await;
                collect(s, &sections, self.settings.command_timeout).await
            };
            match result {
                Ok(raw) => {
                    if slow_due {
                        last_slow = Some(Instant::now());
                    }
                    let first = !model.has_baseline();
                    let metrics = model.update(&raw, Utc::now());
                    self.send(Event::Sample(self.idx, Box::new(metrics))).await;
                    // Rates need two samples: take the second one quickly
                    // instead of waiting a full interval.
                    if first && self.settings.interval > Duration::from_secs(1) {
                        ticker.reset_after(Duration::from_secs(1));
                    }
                }
                Err(e) => {
                    self.fail(&e).await;
                    if let Some(s) = session.take() {
                        s.close().await;
                    }
                    if !self.sleep(backoff.next_delay()).await {
                        break;
                    }
                }
            }
        }
        if let Some(s) = session.take() {
            s.close().await;
        }
    }
}

/// Checks the configured TLS endpoints now and then every hour.
pub(crate) async fn tls_loop(
    settings: Settings,
    events: mpsc::Sender<Event>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        let checks = settings
            .tls_endpoints
            .iter()
            .map(|e| crate::security::check_tls(e, Duration::from_secs(10), &settings.security));
        let results = futures::future::join_all(checks).await;
        if events.send(Event::Tls(results)).await.is_err() {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
            _ = shutdown.changed() => break,
        }
    }
}
