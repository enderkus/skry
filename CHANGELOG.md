# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- One-line installers: `install.sh` for Linux and macOS, `install.ps1` for
  Windows, with checksum verification.
- Website with English and Turkish documentation, published to GitHub Pages.

## [0.1.0]

First release.

### Added

- Agentless collection over SSH with one batched POSIX `sh` command per tick:
  CPU (total and per core), memory and swap, load, uptime, network, disk
  usage and I/O, top processes, listening ports, Docker/Podman containers,
  failed systemd units, failed SSH logins and pending updates from cached
  package metadata.
- SSH client with `known_hosts` verification (hashed entries, wildcards,
  `@revoked`), `--accept-new`, ssh-agent and key file authentication,
  passphrase prompts, `~/.ssh/config` support including `Include` and
  `ProxyJump` chains, and reconnection with exponential backoff.
- Terminal UI with fleet grid, host details, security pulse, timeline
  scrubbing over recorded history, help overlay and snapshots.
- SQLite history with automatic downsampling and retention.
- EWMA baselines with hour-of-day buckets and z-score deviation detection.
- Slack, Discord and generic JSON webhooks with deduplication and cooldown.
- `find port|proc|service`, `security`, `snapshot`, `serve`, `script` and
  `config` commands; `--once --json` output.
- Read-only web dashboard with server-sent events and a Prometheus
  `/metrics` endpoint, token-protected when not bound to loopback.
- Local TLS certificate expiry checks.

[Unreleased]: https://github.com/enderkus/skry/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/enderkus/skry/releases/tag/v0.1.0
