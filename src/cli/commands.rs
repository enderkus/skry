//! Command implementations.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::Utc;
use clap::CommandFactory;
use serde::Serialize;

use skry::collect::{Probe, ProcArgs, Section, UnitStatus, script};
use skry::config::Config;
use skry::engine::once::{OnceOptions, OnceResult, collect_once};
use skry::engine::{ConnState, Engine, Settings};
use skry::security::{HostSecurity, TlsResult, check_tls};
use skry::snapshot::Snapshot;

use super::setup::{Prepared, load_config, prepare};
use super::{Cli, Command, ConfigAction, Find, Global, report};

/// Exit status when at least one host could not be reached.
const EXIT_UNREACHABLE: u8 = 2;

fn init_logging(g: &Global, tui: bool) -> anyhow::Result<()> {
    use tracing_subscriber::EnvFilter;
    let level = match g.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_env("SKRY_LOG")
        .unwrap_or_else(|_| EnvFilter::new(format!("skry={level},warn")));
    if let Some(path) = &g.log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("cannot open log file {}", path.display()))?;
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init();
    } else if !tui {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    }
    Ok(())
}

pub async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let tui = matches!(cli.command, Some(Command::Demo))
        || (cli.command.is_none() && !cli.once && !cli.global.json);
    init_logging(&cli.global, tui)?;
    let g = &cli.global;
    match cli.command {
        None if cli.targets.is_empty() => {
            eprintln!("{}", Cli::command().render_help());
            Ok(ExitCode::from(1))
        }
        None if cli.once || g.json => once(g, &cli.targets).await,
        None => monitor(g, &cli.targets).await,
        Some(Command::Find { what }) => find(g, what).await,
        Some(Command::Security { targets }) => security(g, &targets).await,
        Some(Command::Snapshot { targets, out, post }) => snapshot(g, &targets, out, post).await,
        Some(Command::Serve { targets, bind }) => serve(g, &targets, bind).await,
        Some(Command::Config { action }) => config(g, action),
        Some(Command::Demo) => {
            let source = skry::tui::DemoSource::start();
            skry::tui::run(&source, &Config::default()).await?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn once_options(config: &Config, sections: Vec<Section>, samples: u32) -> OnceOptions {
    OnceOptions {
        sections,
        samples,
        gap: Duration::from_secs(1),
        concurrency: config.concurrency,
        command_timeout: Duration::from_secs_f64(config.command_timeout),
        thresholds: config.thresholds,
        security: config.security.clone(),
    }
}

fn exit_for(results: &[OnceResult]) -> ExitCode {
    if results.iter().any(|r| r.state.conn == ConnState::Failed) {
        ExitCode::from(EXIT_UNREACHABLE)
    } else {
        ExitCode::SUCCESS
    }
}

fn print_json(v: &impl Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

async fn monitor(g: &Global, targets: &[String]) -> anyhow::Result<ExitCode> {
    let Prepared {
        config,
        targets,
        plans,
        ssh,
    } = prepare(g, targets).await?;
    let settings = Settings::from_config(&config, config.tls_endpoints(&targets));
    let engine = Engine::start(plans, settings, ssh);
    let result = skry::tui::run(&engine, &config).await;
    engine.shutdown().await;
    result?;
    Ok(ExitCode::SUCCESS)
}

async fn once(g: &Global, targets: &[String]) -> anyhow::Result<ExitCode> {
    let p = prepare(g, targets).await?;
    let opts = once_options(&p.config, Section::all(p.config.security.rpm_updates), 2);
    let results = collect_once(p.plans, p.ssh, opts).await;
    let states: Vec<_> = results.iter().map(|r| r.state.clone()).collect();
    if g.json {
        print_json(&states)?;
    } else {
        print!("{}", report::fleet(&states));
    }
    Ok(exit_for(&results))
}

#[derive(Serialize)]
struct PortResult {
    host: String,
    listening: Option<bool>,
    addresses: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ProcResult {
    host: String,
    matches: Vec<ProcArgs>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ServiceResult {
    host: String,
    unit: Option<Probe<UnitStatus>>,
    error: Option<String>,
}

async fn find(g: &Global, what: Find) -> anyhow::Result<ExitCode> {
    match what {
        Find::Port { port, targets } => {
            let p = prepare(g, &targets).await?;
            let opts = once_options(&p.config, vec![Section::Meta, Section::Ports], 1);
            let results = collect_once(p.plans, p.ssh, opts).await;
            let out: Vec<PortResult> = results
                .iter()
                .map(|r| {
                    let ports = r.state.metrics.as_ref().map(|m| &m.ports);
                    let (listening, addresses) = match ports {
                        Some(Probe::Ok(list)) => {
                            let addrs: Vec<String> = list
                                .iter()
                                .filter(|l| l.port == port)
                                .map(ToString::to_string)
                                .collect();
                            (Some(!addrs.is_empty()), addrs)
                        }
                        _ => (None, Vec::new()),
                    };
                    PortResult {
                        host: r.state.name.clone(),
                        listening,
                        addresses,
                        error: report::error_of(&r.state),
                    }
                })
                .collect();
            if g.json {
                print_json(&out)?;
            } else {
                let rows: Vec<Vec<String>> = out
                    .iter()
                    .map(|x| {
                        let state = match (x.listening, &x.error) {
                            (_, Some(_)) => "unreachable",
                            (Some(true), _) => "listening",
                            (Some(false), _) => "no",
                            (None, _) => "unknown",
                        };
                        vec![
                            x.host.clone(),
                            state.into(),
                            x.error.clone().unwrap_or_else(|| x.addresses.join(", ")),
                        ]
                    })
                    .collect();
                print!(
                    "{}",
                    report::table(&["HOST", &format!("PORT {port}"), "ADDRESSES"], &rows)
                );
            }
            Ok(exit_for(&results))
        }
        Find::Proc { name, targets } => {
            let p = prepare(g, &targets).await?;
            let opts = once_options(
                &p.config,
                vec![Section::Meta, Section::ProcStat, Section::PsArgs],
                1,
            );
            let results = collect_once(p.plans, p.ssh, opts).await;
            let needle = name.to_lowercase();
            let out: Vec<ProcResult> = results
                .iter()
                .map(|r| {
                    let mut matches = Vec::new();
                    if let Some(raw) = &r.raw {
                        let args = raw.ps_args.clone().unwrap_or_default();
                        for proc in &raw.procs {
                            let a = args.iter().find(|a| a.pid == proc.pid);
                            let cmdline = a.map(|a| a.args.as_str()).unwrap_or("");
                            if proc.comm.to_lowercase().contains(&needle)
                                || cmdline.to_lowercase().contains(&needle)
                            {
                                matches.push(ProcArgs {
                                    pid: proc.pid,
                                    user: a
                                        .map(|a| a.user.clone())
                                        .or_else(|| raw.users.get(&proc.pid).cloned())
                                        .unwrap_or_default(),
                                    args: if cmdline.is_empty() {
                                        proc.comm.clone()
                                    } else {
                                        cmdline.to_string()
                                    },
                                });
                            }
                        }
                        matches.sort_by_key(|m| m.pid);
                    }
                    ProcResult {
                        host: r.state.name.clone(),
                        matches,
                        error: report::error_of(&r.state),
                    }
                })
                .collect();
            if g.json {
                print_json(&out)?;
            } else {
                let mut rows = Vec::new();
                for x in &out {
                    if let Some(e) = &x.error {
                        rows.push(vec![x.host.clone(), "-".into(), "-".into(), e.clone()]);
                    } else if x.matches.is_empty() {
                        rows.push(vec![
                            x.host.clone(),
                            "-".into(),
                            "-".into(),
                            "not running".into(),
                        ]);
                    }
                    for m in &x.matches {
                        rows.push(vec![
                            x.host.clone(),
                            m.pid.to_string(),
                            m.user.clone(),
                            m.args.clone(),
                        ]);
                    }
                }
                print!(
                    "{}",
                    report::table(&["HOST", "PID", "USER", "COMMAND"], &rows)
                );
            }
            Ok(exit_for(&results))
        }
        Find::Service { name, targets } => {
            if !script::valid_unit_name(&name) {
                bail!("invalid unit name {name:?}: use letters, digits and @ . _ : -");
            }
            let p = prepare(g, &targets).await?;
            let opts = once_options(
                &p.config,
                vec![Section::Meta, Section::Unit(name.clone())],
                1,
            );
            let results = collect_once(p.plans, p.ssh, opts).await;
            let out: Vec<ServiceResult> = results
                .iter()
                .map(|r| ServiceResult {
                    host: r.state.name.clone(),
                    unit: r.raw.as_ref().and_then(|raw| raw.unit.clone()),
                    error: report::error_of(&r.state),
                })
                .collect();
            if g.json {
                print_json(&out)?;
            } else {
                let rows: Vec<Vec<String>> = out
                    .iter()
                    .map(|x| match (&x.unit, &x.error) {
                        (_, Some(e)) => vec![
                            x.host.clone(),
                            "unreachable".into(),
                            String::new(),
                            e.clone(),
                        ],
                        (Some(Probe::Ok(u)), _) => vec![
                            x.host.clone(),
                            format!("{}/{}", u.active_state, u.sub_state),
                            u.unit_file_state.clone(),
                            if u.load_state == "not-found" {
                                "not installed".into()
                            } else {
                                u.description.clone()
                            },
                        ],
                        (Some(p), _) => vec![
                            x.host.clone(),
                            "n/a".into(),
                            String::new(),
                            p.describe().unwrap_or_default(),
                        ],
                        (None, _) => {
                            vec![x.host.clone(), "n/a".into(), String::new(), String::new()]
                        }
                    })
                    .collect();
                print!(
                    "{}",
                    report::table(&["HOST", "STATE", "ENABLED", "DESCRIPTION"], &rows)
                );
            }
            Ok(exit_for(&results))
        }
    }
}

async fn tls_checks(config: &Config, endpoints: Vec<String>) -> Vec<TlsResult> {
    let checks = endpoints
        .iter()
        .map(|e| check_tls(e, Duration::from_secs(10), &config.security));
    futures::future::join_all(checks).await
}

#[derive(Serialize)]
struct SecurityReport {
    generated: chrono::DateTime<Utc>,
    hosts: Vec<HostSecurity>,
    tls: Vec<TlsResult>,
}

async fn security(g: &Global, targets: &[String]) -> anyhow::Result<ExitCode> {
    let p = prepare(g, targets).await?;
    let sections = vec![
        Section::Meta,
        Section::OsRelease,
        Section::Ports,
        Section::FailedLogins,
        Section::Updates {
            rpm: p.config.security.rpm_updates,
        },
    ];
    let opts = once_options(&p.config, sections, 1);
    let endpoints = p.config.tls_endpoints(&p.targets);
    let (results, tls) = tokio::join!(
        collect_once(p.plans, p.ssh, opts),
        tls_checks(&p.config, endpoints)
    );
    let hosts: Vec<HostSecurity> = results
        .iter()
        .map(|r| match (&r.state.metrics, report::error_of(&r.state)) {
            (Some(m), None) => {
                let allowed = r
                    .state
                    .allowed_ports
                    .as_ref()
                    .map(|v| v.iter().copied().collect());
                HostSecurity::from_metrics(&r.state.name, m, allowed.as_ref(), &p.config.security)
            }
            (_, e) => HostSecurity::unreachable(&r.state.name, e.unwrap_or_default()),
        })
        .collect();
    let report_data = SecurityReport {
        generated: Utc::now(),
        hosts,
        tls,
    };
    if g.json {
        print_json(&report_data)?;
    } else {
        print!("{}", report::security(&report_data.hosts, &report_data.tls));
    }
    Ok(exit_for(&results))
}

fn expand_home(p: PathBuf) -> PathBuf {
    match (p.strip_prefix("~"), directories::BaseDirs::new()) {
        (Ok(rest), Some(b)) => b.home_dir().join(rest),
        _ => p,
    }
}

async fn snapshot(
    g: &Global,
    targets: &[String],
    out: Option<PathBuf>,
    post: bool,
) -> anyhow::Result<ExitCode> {
    let p = prepare(g, targets).await?;
    let opts = once_options(&p.config, Section::all(p.config.security.rpm_updates), 2);
    let endpoints = p.config.tls_endpoints(&p.targets);
    let (results, tls) = tokio::join!(
        collect_once(p.plans, p.ssh, opts),
        tls_checks(&p.config, endpoints)
    );
    let states = results.iter().map(|r| r.state.clone()).collect();
    let snap = Snapshot::new(states, tls, Utc::now());
    let dir = out
        .or_else(|| p.config.snapshot.dir.clone())
        .map(expand_home)
        .unwrap_or_else(|| PathBuf::from("."));
    let (md, json) = snap.write(&dir).context("cannot write the snapshot")?;
    if g.json {
        println!("{}", snap.to_json());
    } else {
        println!("{}\n{}", md.display(), json.display());
    }
    if post {
        if !p.config.webhooks.iter().any(|w| w.snapshot) {
            eprintln!("warning: --post given but no webhook has `snapshot = true`");
        }
        for e in skry::snapshot::post(&snap, &p.config.webhooks).await {
            eprintln!("warning: {e}");
        }
    }
    Ok(exit_for(&results))
}

async fn serve(g: &Global, targets: &[String], bind: Option<String>) -> anyhow::Result<ExitCode> {
    let p = prepare(g, targets).await?;
    let addr = bind.unwrap_or_else(|| p.config.web.bind.clone());
    let token = p.config.web.effective_token();
    let listener = skry::web::bind(&addr, token.as_ref()).await?;
    let local = listener.local_addr()?;
    let settings = Settings::from_config(&p.config, p.config.tls_endpoints(&p.targets));
    let engine = Engine::start(p.plans, settings, p.ssh);
    eprintln!(
        "skry dashboard on http://{local}/ and metrics on http://{local}/metrics{}",
        if token.is_some() {
            " (access token required)"
        } else {
            ""
        }
    );
    let result = skry::web::serve(listener, engine.fleet(), engine.changes(), token, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await;
    engine.shutdown().await;
    result?;
    Ok(ExitCode::SUCCESS)
}

fn config(g: &Global, action: ConfigAction) -> anyhow::Result<ExitCode> {
    match action {
        ConfigAction::Path => {
            match g.config.clone().or_else(Config::default_path) {
                Some(p) => println!(
                    "{}{}",
                    p.display(),
                    if p.exists() { "" } else { " (not created yet)" }
                ),
                None => bail!("cannot determine the config directory"),
            }
            Ok(ExitCode::SUCCESS)
        }
        ConfigAction::Example => {
            print!("{}", skry::config::EXAMPLE);
            Ok(ExitCode::SUCCESS)
        }
        ConfigAction::Check => {
            let c = load_config(g)?;
            let hosts: usize = c.groups.values().map(|g| g.hosts.len()).sum();
            println!(
                "config ok: {} groups ({} entries), {} webhooks, history {}",
                c.groups.len(),
                hosts,
                c.webhooks.len(),
                if c.history.enabled {
                    format!("{} h", c.history.retention_hours)
                } else {
                    "off".into()
                }
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}
