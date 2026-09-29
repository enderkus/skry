# Compatibility

## Your computer (where skry runs)

| Platform | Status |
| --- | --- |
| Linux x86_64 and aarch64 | Supported. Static binaries, run on any distribution. |
| macOS Apple Silicon and Intel | Supported. |
| Windows 10/11 x86_64 | Supported. Uses the Windows OpenSSH agent or Pageant, and `%USERPROFILE%\.ssh`. On ARM64 Windows the x86_64 build runs under emulation. |

Any terminal with Unicode and colours works; on Windows use Windows Terminal.

## Monitored servers

The servers must be Linux with an SSH server and a POSIX `sh`. Tested
continuously (real SSH servers in the integration tests, real output in the
parser tests):

| Distribution | Tested version | Notes |
| --- | --- | --- |
| Debian | 12 | Everything supported. |
| Ubuntu | 24.04 | Everything supported. |
| Rocky Linux (RHEL, AlmaLinux) | 9 | Pending updates are opt-in (`rpm_updates`) and need root. |
| Alpine | 3.20 (BusyBox) | Uses BusyBox `ps` and `netstat`; no security metadata for updates; no systemd. |

Other distributions based on these (Linux Mint, Raspberry Pi OS, CentOS
Stream, Oracle Linux, Amazon Linux, Fedora…) generally work, because skry
relies on the kernel's `/proc` files and very common tools. Anything missing
shows as `n/a`.

### What the server needs

| Required | Optional (feature is `n/a` without it) |
| --- | --- |
| sshd, `/bin/sh` (POSIX) | `ss` or `netstat` (ports; `/proc/net/tcp` is the last fallback) |
| `cat`, `grep`, `awk`, `sed`, `head` | `ip` (addresses) |
| `df`, `ps`, `uname`, `date`, `id` | `timeout` (protection against hanging commands) |
| a readable `/proc` | `getconf` (page size, clock ticks; 4096 and 100 are assumed otherwise) |
| | `systemctl`, `journalctl` (systemd features) |
| | `docker` or `podman` (containers) |
| | `apt-get`, `apk` or `dnf` (updates) |

All of these are present on standard installs; minimal container images may
lack a few.

### Kernel

skry reads long-standing `/proc` files, so older kernels generally work. Only
kernels older than 3.14 need a fallback: `MemAvailable` is missing there and
memory is estimated (see [calculations](calculations.md#memory)). The
kernels tested continuously are current 6.x kernels.

### Not supported as monitored hosts

- Non-Linux systems (BSD, macOS, Windows servers): skry reads Linux `/proc`.
- Network devices (routers, switches) with their own CLI.
- Accounts with restricted shells or forced commands.

## Containers and VMs as targets

skry can monitor a container that runs sshd, but many values (CPU, memory,
disks, load) then describe the **host kernel**, because containers share it.
Monitor the host itself for meaningful numbers. Virtual machines are
monitored like physical servers; watch the `steal` value.
