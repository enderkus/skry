//! Command-line entry points.

mod commands;
mod report;
mod setup;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "skry",
    version,
    about = "See every server. Install nothing.",
    long_about = "Agentless Linux server monitoring over plain SSH. Nothing is installed or written on the \
                  remote hosts: skry reads /proc and a few standard commands in one batched POSIX sh \
                  command per tick.",
    after_help = "Examples:\n  skry web1                      monitor one host from ~/.ssh/config\n  \
                  skry deploy@10.0.0.5:2222 db1  several hosts\n  skry @production               a host group from the config file\n  \
                  skry @production --once --json one sample as JSON\n  skry find port 5432 @production which hosts listen on 5432\n  \
                  skry security @production      security pulse report\n  skry serve @production         web dashboard and /metrics"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: Global,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Hosts to monitor: names from ~/.ssh/config, user@host:port, or @group.
    pub targets: Vec<String>,

    /// Collect one sample, print it and exit.
    #[arg(long)]
    pub once: bool,
}

#[derive(Debug, Clone, Args)]
pub struct Global {
    /// Seconds between samples (overrides the config file).
    #[arg(long, global = true, value_name = "SECS")]
    pub interval: Option<f64>,

    /// Record unknown host keys in known_hosts instead of refusing them
    /// (like StrictHostKeyChecking=accept-new). Changed keys are always refused.
    #[arg(long, global = true)]
    pub accept_new: bool,

    /// Config file (default: ~/.config/skry/config.toml or the platform config directory).
    #[arg(
        long,
        short = 'c',
        global = true,
        value_name = "FILE",
        env = "SKRY_CONFIG"
    )]
    pub config: Option<PathBuf>,

    /// Print machine-readable JSON (query and report commands).
    #[arg(long, global = true)]
    pub json: bool,

    /// SSH client config (default: ~/.ssh/config).
    #[arg(long, global = true, value_name = "FILE")]
    pub ssh_config: Option<PathBuf>,

    /// Maximum hosts contacted at once (overrides the config file).
    #[arg(long, global = true, value_name = "N")]
    pub concurrency: Option<usize>,

    /// Do not record history.
    #[arg(long, global = true)]
    pub no_history: bool,

    /// Do not use ssh-agent.
    #[arg(long, global = true)]
    pub no_agent: bool,

    /// Write logs to this file (the TUI never logs to the terminal).
    #[arg(long, global = true, value_name = "FILE")]
    pub log_file: Option<PathBuf>,

    /// More log output (-v info, -vv debug).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Search the fleet for a listening port, a process or a service.
    Find {
        #[command(subcommand)]
        what: Find,
    },
    /// Security pulse: failed SSH logins, pending security updates,
    /// unexpected listening ports and TLS certificate expiry.
    Security {
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
    /// Capture an incident snapshot (Markdown and JSON).
    Snapshot {
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
        /// Directory for the snapshot files.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Also post the snapshot to webhooks configured with `snapshot = true`.
        #[arg(long)]
        post: bool,
    },
    /// Serve the web dashboard and a Prometheus /metrics endpoint.
    Serve {
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
        /// Listen address (default: 127.0.0.1:9187). Non-loopback addresses
        /// require an access token.
        #[arg(long, value_name = "ADDR")]
        bind: Option<String>,
    },
    /// Print the read-only POSIX sh script that skry runs on each host, for
    /// auditing. Nothing is executed.
    Script {
        /// Include the slow sections (ports, containers, units, logins, updates).
        #[arg(long)]
        all: bool,
    },
    /// Show the config file location, print an example, or validate it.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Run the TUI on a synthetic fleet (no SSH).
    #[command(hide = true)]
    Demo,
}

#[derive(Debug, Subcommand)]
pub enum Find {
    /// Which hosts are listening on a TCP port.
    Port {
        port: u16,
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
    /// Which hosts run a process (matched against name and command line).
    Proc {
        name: String,
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
    /// State of a systemd unit across hosts.
    Service {
        name: String,
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the config file path in use.
    Path,
    /// Print a commented example config.
    Example,
    /// Validate the config file.
    Check,
}

pub async fn run(cli: Cli) -> anyhow::Result<std::process::ExitCode> {
    commands::run(cli).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_consistent() {
        Cli::command().debug_assert();
    }

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("skry").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn targets_and_subcommands() {
        let c = parse(&["web1", "deploy@db:2222", "@prod", "--once", "--json"]);
        assert!(c.command.is_none());
        assert_eq!(c.targets, vec!["web1", "deploy@db:2222", "@prod"]);
        assert!(c.once && c.global.json);

        let c = parse(&["--accept-new", "find", "port", "5432", "@prod"]);
        assert!(c.global.accept_new);
        match c.command {
            Some(Command::Find {
                what: Find::Port { port, targets },
            }) => {
                assert_eq!(port, 5432);
                assert_eq!(targets, vec!["@prod"]);
            }
            other => panic!("{other:?}"),
        }

        let c = parse(&[
            "serve",
            "@prod",
            "--bind",
            "0.0.0.0:9187",
            "--interval",
            "5",
        ]);
        assert_eq!(c.global.interval, Some(5.0));
        assert!(matches!(c.command, Some(Command::Serve { .. })));

        assert!(
            Cli::try_parse_from(["skry", "security"]).is_err(),
            "targets are required"
        );

        // Global flags may come before the subcommand.
        let c = parse(&["--config", "/tmp/x.toml", "config", "path"]);
        assert!(matches!(
            c.command,
            Some(Command::Config {
                action: ConfigAction::Path
            })
        ));
        assert_eq!(
            c.global.config.as_deref(),
            Some(std::path::Path::new("/tmp/x.toml"))
        );
        let c = parse(&["--no-agent", "web1"]);
        assert_eq!(c.targets, vec!["web1"]);
        assert!(c.command.is_none());
    }
}
