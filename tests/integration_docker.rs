//! Integration tests against real SSH servers in Docker containers.
//!
//! Run with `SKRY_DOCKER_TESTS=1 cargo test --test integration_docker`.
//! The fleet in `tests/docker/compose.yml` is (re)created once per run with
//! a freshly generated key. Containers are left running for inspection;
//! remove them with `docker compose -p skry-it down`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use skry::collect::{RawSample, Section, script};
use skry::ssh::auth::{KeyStore, NoPrompt};
use skry::ssh::sshconfig::SshConfig;
use skry::ssh::{FailureKind, Session, SshError, SshOptions, resolve};

const PORT_BASE: &str = "2230";

struct Fleet {
    dir: tempfile::TempDir,
    key: PathBuf,
}

fn enabled() -> bool {
    std::env::var("SKRY_DOCKER_TESTS").is_ok_and(|v| v == "1")
}

fn fleet() -> &'static Fleet {
    static FLEET: OnceLock<Fleet> = OnceLock::new();
    FLEET.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("id_ed25519");
        let ok = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "skry-it", "-f"])
            .arg(&key)
            .status()
            .unwrap()
            .success();
        assert!(ok, "ssh-keygen failed");
        let pubkey = std::fs::read_to_string(key.with_extension("pub")).unwrap();
        let compose = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/docker/compose.yml");
        let status = Command::new("docker")
            .args(["compose", "-f"])
            .arg(&compose)
            .args(["up", "-d", "--build", "--force-recreate", "--wait"])
            .env("SKRY_TEST_PUBKEY", pubkey.trim())
            .env("SKRY_IT_PORT_BASE", PORT_BASE)
            .status()
            .expect("docker compose");
        assert!(status.success(), "docker compose up failed");
        // sshd needs a moment to generate host keys.
        let deadline = Instant::now() + Duration::from_secs(60);
        for port in 0..=4 {
            let addr = format!("127.0.0.1:{PORT_BASE}{port}");
            loop {
                if let Ok(mut s) = std::net::TcpStream::connect(&addr) {
                    use std::io::Read;
                    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = [0u8; 4];
                    if s.read_exact(&mut buf).is_ok() && &buf == b"SSH-" {
                        break;
                    }
                }
                assert!(Instant::now() < deadline, "{addr} did not come up");
                std::thread::sleep(Duration::from_millis(300));
            }
        }
        Fleet { dir, key }
    })
}

fn known_hosts(name: &str) -> PathBuf {
    fleet().dir.path().join(format!("known_hosts_{name}"))
}

fn options(known: PathBuf, accept_new: bool) -> SshOptions {
    let f = fleet();
    let keys = Arc::new(KeyStore::new());
    keys.prepare(std::slice::from_ref(&f.key), &[], &NoPrompt);
    let mut o = SshOptions::new(keys);
    o.accept_new = accept_new;
    o.use_agent = false;
    o.known_hosts_override = Some(vec![known]);
    o
}

fn ssh_config(extra: &str) -> SshConfig {
    let f = fleet();
    let text = format!(
        "Host *\n  IdentityFile {}\n  User skry\n{extra}",
        f.key.display()
    );
    SshConfig::parse(&text, f.dir.path())
}

async fn collect(session: &Session) -> RawSample {
    let nonce = script::new_nonce();
    let cmd = script::build(&Section::all(false), &nonce);
    let out = session.exec(&cmd, Duration::from_secs(60)).await.unwrap();
    RawSample::parse(&out.stdout, &nonce)
}

macro_rules! require_docker {
    () => {
        if !enabled() {
            eprintln!("set SKRY_DOCKER_TESTS=1 to run Docker integration tests");
            return;
        }
    };
}

#[tokio::test]
async fn key_auth_and_collection_on_every_distro() {
    require_docker!();
    let cfg = ssh_config("");
    for (port, os) in [(1, "debian"), (2, "ubuntu"), (3, "rocky"), (4, "alpine")] {
        let host = resolve(&format!("127.0.0.1:{PORT_BASE}{port}"), &cfg).unwrap();
        let opts = options(known_hosts(os), true);
        let session = Session::connect(&host, &opts)
            .await
            .unwrap_or_else(|e| panic!("{os}: {e}"));
        let raw = collect(&session).await;
        assert!(raw.complete, "{os}: truncated output");
        assert_eq!(raw.os.as_ref().unwrap().id, os);
        assert!(raw.stat.is_some(), "{os}: no cpu");
        assert!(raw.mem.is_some(), "{os}: no mem");
        assert!(!raw.procs.is_empty(), "{os}: no procs");
        let ports = raw.ports.unwrap();
        assert!(
            ports.ok().unwrap().iter().any(|p| p.port == 22),
            "{os}: sshd not listening?"
        );
        session.close().await;

        // The recorded key is now trusted without --accept-new.
        let opts = options(known_hosts(os), false);
        Session::connect(&host, &opts)
            .await
            .unwrap_or_else(|e| panic!("{os} strict: {e}"));
    }
}

#[tokio::test]
async fn nothing_is_written_on_the_remote_host() {
    require_docker!();
    let cfg = ssh_config("");
    let host = resolve(&format!("127.0.0.1:{PORT_BASE}1"), &cfg).unwrap();
    let session = Session::connect(&host, &options(known_hosts("rw"), true))
        .await
        .unwrap();
    let list = "find / -xdev -newer /proc/1/cmdline -type f ! -path '/proc/*' ! -path '/sys/*' ! -path '/run/*' ! -path '/var/log/*' ! -path '/dev/*' 2>/dev/null | sort";
    let before = session
        .exec(list, Duration::from_secs(30))
        .await
        .unwrap()
        .stdout;
    for _ in 0..3 {
        collect(&session).await;
    }
    let after = session
        .exec(list, Duration::from_secs(30))
        .await
        .unwrap()
        .stdout;
    assert_eq!(before, after, "collection modified files on the host");
}

#[tokio::test]
async fn unknown_host_key_is_refused() {
    require_docker!();
    let cfg = ssh_config("");
    let host = resolve(&format!("127.0.0.1:{PORT_BASE}1"), &cfg).unwrap();
    let known = known_hosts("empty");
    let err = Session::connect(&host, &options(known.clone(), false))
        .await
        .err()
        .expect("unknown key accepted");
    assert!(matches!(err, SshError::HostKeyUnknown { .. }), "{err}");
    assert_eq!(err.kind(), FailureKind::HostKey);
    assert!(
        !known.exists(),
        "known_hosts must not be written without --accept-new"
    );
}

#[tokio::test]
async fn changed_host_key_is_refused_even_with_accept_new() {
    require_docker!();
    let cfg = ssh_config("");
    let host = resolve(&format!("127.0.0.1:{PORT_BASE}2"), &cfg).unwrap();
    let known = known_hosts("changed");
    std::fs::write(
        &known,
        format!(
            "[127.0.0.1]:{PORT_BASE}2 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG+jDkXzDAML+meMRLYztjniTgabJ/ejAVZ3JTyYqqEg\n"
        ),
    )
    .unwrap();
    let err = Session::connect(&host, &options(known, true))
        .await
        .err()
        .expect("changed key accepted");
    assert!(matches!(err, SshError::HostKeyChanged { .. }), "{err}");
}

#[tokio::test]
async fn wrong_key_fails_authentication() {
    require_docker!();
    let f = fleet();
    let other = f.dir.path().join("other_key");
    Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&other)
        .status()
        .unwrap();
    let cfg = SshConfig::parse(
        &format!("Host *\n  IdentityFile {}\n  User skry\n", other.display()),
        f.dir.path(),
    );
    let host = resolve(&format!("127.0.0.1:{PORT_BASE}4"), &cfg).unwrap();
    let keys = Arc::new(KeyStore::new());
    keys.prepare(&host.identity_files, &[], &NoPrompt);
    let mut opts = SshOptions::new(keys);
    opts.use_agent = false;
    opts.accept_new = true;
    opts.known_hosts_override = Some(vec![known_hosts("wrongkey")]);
    let err = Session::connect(&host, &opts)
        .await
        .err()
        .expect("wrong key accepted");
    assert_eq!(err.kind(), FailureKind::Auth, "{err}");
}

#[tokio::test]
async fn proxy_jump_through_bastion() {
    require_docker!();
    let cfg = ssh_config(&format!(
        "Host bastion\n  HostName 127.0.0.1\n  Port {PORT_BASE}0\nHost hidden\n  ProxyJump bastion\n"
    ));
    let host = resolve("hidden", &cfg).unwrap();
    assert_eq!(host.jumps.len(), 1);
    let known = known_hosts("jump");
    let session = Session::connect(&host, &options(known.clone(), true))
        .await
        .unwrap();
    let raw = collect(&session).await;
    assert!(raw.complete);
    assert_eq!(raw.os.unwrap().id, "debian");
    let recorded = std::fs::read_to_string(&known).unwrap();
    assert!(
        recorded.contains(&format!("[127.0.0.1]:{PORT_BASE}0 ")),
        "bastion key recorded"
    );
    assert!(
        recorded.lines().any(|l| l.starts_with("hidden ")),
        "target key recorded"
    );

    // An unknown bastion key stops the chain with a jump error.
    let err = Session::connect(&host, &options(known_hosts("jump-empty"), false))
        .await
        .err()
        .unwrap();
    assert!(matches!(err, SshError::Jump { .. }), "{err}");
    assert_eq!(err.kind(), FailureKind::HostKey);
}

#[tokio::test]
async fn engine_keeps_healthy_hosts_running_when_others_fail() {
    use skry::config::{Config, Target};
    use skry::engine::{ConnState, Engine, HostPlan, HostStatus, Settings};
    require_docker!();
    let cfg = ssh_config("");
    let known = known_hosts("engine");
    // Pre-trust the Debian host so it can be checked strictly.
    let debian = format!("127.0.0.1:{PORT_BASE}1");
    Session::connect(
        &resolve(&debian, &cfg).unwrap(),
        &options(known.clone(), true),
    )
    .await
    .unwrap();
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_port = closed.local_addr().unwrap().port();
    drop(closed);
    let specs = [
        debian.clone(),
        format!("127.0.0.1:{PORT_BASE}3"),  // unknown host key
        format!("127.0.0.1:{closed_port}"), // connection refused
    ];
    let plans: Vec<HostPlan> = specs
        .iter()
        .map(|s| HostPlan {
            target: Target {
                spec: s.clone(),
                groups: vec![],
                allowed_ports: Some([22].into()),
            },
            resolved: resolve(s, &cfg).map_err(|e| e.to_string()),
        })
        .collect();
    let mut config = Config {
        interval: 1.0,
        ..Default::default()
    };
    config.history.path = Some(fleet().dir.path().join("engine-history.db"));
    let settings = Settings::from_config(&config, vec![]);
    let engine = Engine::start(plans, settings, options(known, false));
    let mut changes = engine.changes();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let f = engine.snapshot();
        let ready = f.hosts[0].metrics.as_ref().is_some_and(|m| m.cpu.is_some())
            && f.hosts[1].conn == ConnState::Failed
            && f.hosts[2].conn == ConnState::Failed;
        if ready {
            break;
        }
        assert!(Instant::now() < deadline, "engine did not converge: {f:#?}");
        let _ = tokio::time::timeout(Duration::from_secs(1), changes.changed()).await;
    }
    let f = engine.snapshot();
    assert!(matches!(
        f.hosts[0].status,
        HostStatus::Ok | HostStatus::Warning | HostStatus::Critical
    ));
    assert!(
        f.hosts[0].findings.is_empty(),
        "only port 22 listens: {:?}",
        f.hosts[0].findings
    );
    assert_eq!(f.hosts[1].status, HostStatus::Unreachable);
    assert_eq!(
        f.hosts[1].error.as_ref().unwrap().kind,
        FailureKind::HostKey
    );
    assert_eq!(
        f.hosts[2].error.as_ref().unwrap().kind,
        FailureKind::Network
    );
    let history = engine.history_path().cloned().unwrap();
    engine.shutdown().await;

    let store = skry::store::Store::open(&history).unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    let rows = store.fleet_at(now, 60_000).unwrap();
    assert_eq!(rows.len(), 3, "every host has history: {rows:?}");
    assert!(store.detail_at(&debian, now, 60_000).unwrap().is_some());
}

#[tokio::test]
async fn cli_once_json_and_find_end_to_end() {
    require_docker!();
    let f = fleet();
    let dir = f.dir.path();
    let known = dir.join("known_hosts_cli");
    let ssh_config = dir.join("ssh_config_cli");
    let mut text = format!(
        "Host *\n  User skry\n  IdentityFile {}\n  UserKnownHostsFile {}\n",
        f.key.display(),
        known.display()
    );
    for (name, port) in [
        ("debian", 1),
        ("ubuntu", 2),
        ("rocky", 3),
        ("alpine", 4),
        ("bastion", 0),
    ] {
        text.push_str(&format!(
            "Host {name}\n  HostName 127.0.0.1\n  Port {PORT_BASE}{port}\n"
        ));
    }
    text.push_str("Host hidden\n  ProxyJump bastion\n");
    std::fs::write(&ssh_config, text).unwrap();
    let config = dir.join("config_cli.toml");
    std::fs::write(
        &config,
        "[history]\nenabled = false\n[groups.lab]\nhosts = [\"debian\", \"ubuntu\", \"rocky\", \"alpine\", \"hidden\"]\nallowed_ports = [22]\n",
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_skry"))
            .arg("--config")
            .arg(&config)
            .arg("--ssh-config")
            .arg(&ssh_config)
            .arg("--no-agent")
            .args(args)
            .output()
            .unwrap()
    };

    // Unknown keys are refused: exit status 2, nothing recorded.
    let out = run(&["@lab", "--once", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!known.exists());

    let out = run(&["--accept-new", "@lab", "--once", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let hosts: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hosts = hosts.as_array().unwrap();
    assert_eq!(hosts.len(), 5);
    for h in hosts {
        assert_eq!(h["conn"], "connected", "{h}");
        assert!(
            h["metrics"]["cpu"]["total_pct"].is_number(),
            "rates need two samples: {h}"
        );
    }

    let out = run(&["find", "port", "22", "@lab", "--json"]);
    assert!(out.status.success());
    let found: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        found
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["listening"] == true)
    );

    let out = run(&["find", "proc", "sshd", "alpine"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("/usr/sbin/sshd"));

    let out = run(&["find", "service", "x;id", "debian"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid unit name"));

    let out = run(&["security", "@lab", "--json"]);
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["hosts"].as_array().unwrap().len(), 5);
}
