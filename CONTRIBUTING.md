# Contributing to skry

Thanks for helping! Bug reports, parser fixes for distributions we have not
seen yet, and focused pull requests are all welcome.

## Ground rules

skry's promises shape every change:

1. **Agentless.** Nothing may be installed, copied or written on a remote
   host. No temp files, no sudo.
2. **Read-only.** The remote script only reads. Never add anything that
   changes state (`apt update`, `dnf makecache`, …) or that lets a user run
   arbitrary commands across the fleet. Any user input that reaches the
   remote shell must be validated against a strict allowlist.
3. **POSIX sh only.** The remote script must run under `dash`, `bash --posix`
   and BusyBox `ash`. No bashisms (`[[`, `$'…'`, arrays, `local`, process
   substitution). Missing tools or permissions must degrade to `n/a`.
4. **Parsers are pure.** `src/collect/parse.rs` does no I/O and every parser
   is tested against real output in `tests/fixtures`.

## Getting started

```sh
git clone https://github.com/enderkus/skry
cd skry
cargo test
cargo run -- demo        # the TUI on a synthetic fleet
```

You need a stable Rust toolchain (1.85+). Docker is needed only for the
integration tests.

## Tests

| Command | What it runs |
| --- | --- |
| `cargo test` | Unit tests, parser tests against fixtures, TUI and report snapshot tests |
| `SKRY_DOCKER_TESTS=1 cargo test --test integration_docker` | Starts `tests/docker/compose.yml` (Debian, Ubuntu, Rocky, Alpine, a bastion and a host reachable only through it) and runs skry against them: key auth, ProxyJump, unknown and changed host keys, the engine, the CLI, and a check that collection writes nothing on the host |
| `SKRY_NETWORK_TESTS=1 cargo test security` | TLS checks against public endpoints |

The Docker fleet is left running after the tests for inspection; remove it
with `docker compose -p skry-it down`.

### Snapshot tests

TUI screens (rendered with ratatui's `TestBackend`), the Markdown snapshot
report and the Prometheus output are checked with
[insta](https://insta.rs). After an intentional change:

```sh
cargo install cargo-insta   # once
cargo insta test --review
```

Commit the updated `.snap` files together with the change.

### Adding or refreshing fixtures

Fixtures are real command output, never hand-written. To capture a new
distribution:

1. Start a container or VM with an SSH server.
2. Print the command with a fixed nonce:
   `cargo run --example dump_script -- 0123456789abcdef > /tmp/cmd.sh`
3. Run it over SSH and save the output:
   `ssh user@host "$(cat /tmp/cmd.sh)" > tests/fixtures/<distro>/full.txt`
4. Add a test in `src/collect/parse.rs` and a line to
   `tests/fixtures/README.md`.

Please strip anything private (hostnames, addresses, user names) that you do
not want in a public repository.

## Style

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must be clean;
  CI enforces both.
- Library code uses `thiserror`; the binary uses `anyhow`.
- Keep comments short and about *why*.
- Never log secrets. Webhook URLs go through `config::redact_url`.

## Commits and pull requests

- Small, focused commits with conventional messages that describe the change,
  for example `feat(collect): parse /proc/pressure/cpu` or
  `fix(ssh): honour HostKeyAlias for jump hosts`.
- Include tests with every behaviour change.
- Describe what you tested (distributions, platforms) in the pull request.

By contributing you agree that your contributions are dual licensed under
the MIT and Apache-2.0 licenses, as described in the README.
