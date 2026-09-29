# Installation

skry is a single program (one file). You install it **on your own computer**
— your laptop, your workstation, or a jump server you log in to. You do
**not** install anything on the servers you want to watch.

> [!NOTE]
> The servers you monitor must be **Linux** and reachable over SSH. The
> computer you run skry on can be **Linux, macOS or Windows**.

## The quick way (recommended)

### Linux and macOS

Open a terminal and paste:

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | sh
```

What this does, in order:

1. Detects your operating system and processor (for example "macOS on Apple
   Silicon" or "Linux on x86_64").
2. Finds the latest skry release on GitHub.
3. Downloads the matching archive **and** its SHA-256 checksum file.
4. Checks that the archive matches the checksum. If a single byte differs,
   it stops and installs nothing.
5. Copies the `skry` program to `/usr/local/bin` if you are allowed to write
   there, otherwise to `~/.local/bin` in your home directory.
6. Prints the installed version.

If it says the directory is not on your `PATH`, follow the command it prints
(it adds the directory to your shell's search path), then open a new
terminal.

### Windows

Open **PowerShell** (not the old "Command Prompt") and paste:

```powershell
irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

It does the same checks and installs `skry.exe` into
`%LOCALAPPDATA%\Programs\skry`, then adds that folder to your user `PATH`.
**Open a new terminal window** afterwards so the new `PATH` takes effect.

### Choosing a version or a folder

Two optional settings change what the installer does:

| Variable | Meaning | Example |
| --- | --- | --- |
| `SKRY_VERSION` | Install this release instead of the latest | `v0.1.0` |
| `SKRY_INSTALL_DIR` | Install into this folder | `/opt/bin` |

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | SKRY_VERSION=v0.1.0 SKRY_INSTALL_DIR=/opt/bin sh
```

```powershell
$env:SKRY_VERSION = 'v0.1.0'; irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

> [!TIP]
> Not comfortable piping a script from the internet into your shell? Good
> instinct. Download [`install.sh`](https://github.com/enderkus/skry/blob/main/install.sh)
> or [`install.ps1`](https://github.com/enderkus/skry/blob/main/install.ps1),
> read it (it is short and commented), then run the local copy with
> `sh install.sh` or `./install.ps1`.

## The manual way

Every release on the [releases page](https://github.com/enderkus/skry/releases)
contains one archive per platform plus a `.sha256` file:

| Your computer | File to download |
| --- | --- |
| Linux, Intel/AMD 64-bit | `skry-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM 64-bit (e.g. Raspberry Pi 4/5, Graviton) | `skry-<version>-aarch64-unknown-linux-musl.tar.gz` |
| macOS, Apple Silicon (M1/M2/M3/M4) | `skry-<version>-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `skry-<version>-x86_64-apple-darwin.tar.gz` |
| Windows, 64-bit | `skry-<version>-x86_64-pc-windows-msvc.zip` |

Not sure which processor you have? Run `uname -m` on Linux or macOS:
`x86_64` means Intel/AMD, `aarch64` or `arm64` means ARM.

Example for Linux x86_64 and version v0.1.0:

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
sha256sum -c skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256     # must print "OK"
tar xzf skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
sudo install skry-v0.1.0-x86_64-unknown-linux-musl/skry /usr/local/bin/
```

On macOS use `shasum -a 256 -c` instead of `sha256sum -c`. If you downloaded
with a browser, macOS may refuse to open the file ("cannot be opened because
the developer cannot be verified"). Remove the download quarantine once:

```sh
xattr -d com.apple.quarantine /usr/local/bin/skry
```

On Windows, extract the zip and move `skry.exe` to any folder that is on your
`PATH`.

The Linux builds are **static**: they do not depend on your distribution's
libraries and run on any Linux (glibc or musl, old or new).

## From source

If you have a Rust toolchain (1.85 or newer):

```sh
cargo install --locked --git https://github.com/enderkus/skry
```

## Check that it works

```sh
skry --version
skry demo
```

`skry demo` opens the interface with an imaginary fleet — no servers, no SSH.
Press `?` for help and `q` to quit.

## Upgrading

Run the installer again. It overwrites the old binary. Your configuration
and history are kept.

## Uninstalling

1. Delete the program: `/usr/local/bin/skry` or `~/.local/bin/skry`
   (Windows: the folder `%LOCALAPPDATA%\Programs\skry`, and optionally its
   entry in your user `PATH`).
2. Optionally delete your configuration and history. See
   [Files skry keeps on your machine](data.md) for the exact locations.

Nothing needs to be removed from your servers, because nothing was ever put
there.
