#!/bin/sh
# skry installer for Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | sh
#
# Environment variables:
#   SKRY_VERSION      release tag to install, e.g. v0.1.0 (default: latest)
#   SKRY_INSTALL_DIR  target directory (default: /usr/local/bin if writable,
#                     otherwise ~/.local/bin; sudo is used only if you set a
#                     non-writable directory explicitly)
#
# The script downloads the release archive for this platform from GitHub,
# verifies its SHA-256 checksum, and installs the `skry` binary. It needs
# curl or wget, tar, and sha256sum or shasum.

set -eu

REPO="enderkus/skry"
BASE="https://github.com/$REPO/releases"

say() { printf 'skry-install: %s\n' "$*"; }
die() { printf 'skry-install: error: %s\n' "$*" >&2; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

download() {
    # download URL FILE
    if have curl; then
        curl -fsSL --retry 3 -o "$2" "$1"
    elif have wget; then
        wget -q -O "$2" "$1"
    else
        die "curl or wget is required"
    fi
}

latest_tag() {
    # The /releases/latest page redirects to /releases/tag/<tag>; reading the
    # redirect avoids the rate-limited GitHub API.
    if have curl; then
        url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$BASE/latest")
    else
        url=$(wget -q -S --spider "$BASE/latest" 2>&1 | sed -n 's/^ *[Ll]ocation: *//p' | tail -n 1)
    fi
    tag=${url##*/}
    tag=$(printf '%s' "$tag" | tr -d '\r')
    case "$tag" in
        v[0-9]*) printf '%s\n' "$tag" ;;
        *) die "could not determine the latest release (got '$url')" ;;
    esac
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$arch" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
        *) die "unsupported architecture: $arch (build from source: cargo install --git https://github.com/$REPO)" ;;
    esac
    case "$os" in
        Linux) printf '%s-unknown-linux-musl\n' "$arch" ;;
        Darwin) printf '%s-apple-darwin\n' "$arch" ;;
        *) die "unsupported OS: $os (on Windows use install.ps1)" ;;
    esac
}

verify() {
    # verify ARCHIVE CHECKSUM_FILE
    expected=$(cut -d ' ' -f 1 "$2")
    if have sha256sum; then
        actual=$(sha256sum "$1" | cut -d ' ' -f 1)
    elif have shasum; then
        actual=$(shasum -a 256 "$1" | cut -d ' ' -f 1)
    else
        die "sha256sum or shasum is required to verify the download"
    fi
    [ "$expected" = "$actual" ] || die "checksum mismatch for $(basename "$1")"
}

choose_dir() {
    if [ -n "${SKRY_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$SKRY_INSTALL_DIR"
    elif [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
        printf '/usr/local/bin\n'
    else
        printf '%s/.local/bin\n' "$HOME"
    fi
}

main() {
    have tar || die "tar is required"
    have curl || have wget || die "curl or wget is required (e.g. apt-get install curl)"
    target=$(detect_target)
    tag=${SKRY_VERSION:-}
    [ -n "$tag" ] || tag=$(latest_tag)
    case "$tag" in v*) ;; *) tag="v$tag" ;; esac

    name="skry-$tag-$target"
    url="$BASE/download/$tag/$name.tar.gz"
    tmp=$(mktemp -d 2>/dev/null || mktemp -d -t skry)
    trap 'rm -rf "$tmp"' EXIT INT TERM

    say "downloading skry $tag for $target"
    download "$url" "$tmp/$name.tar.gz" || die "download failed: $url"
    download "$url.sha256" "$tmp/$name.tar.gz.sha256" || die "download failed: $url.sha256"
    verify "$tmp/$name.tar.gz" "$tmp/$name.tar.gz.sha256"
    tar -xzf "$tmp/$name.tar.gz" -C "$tmp"

    dir=$(choose_dir)
    sudo=""
    if [ -d "$dir" ] && [ ! -w "$dir" ]; then
        have sudo || die "$dir is not writable; set SKRY_INSTALL_DIR to a writable directory"
        sudo="sudo"
        say "using sudo to write to $dir"
    fi
    $sudo mkdir -p "$dir"
    $sudo cp "$tmp/$name/skry" "$dir/skry.tmp"
    $sudo chmod 0755 "$dir/skry.tmp"
    $sudo mv -f "$dir/skry.tmp" "$dir/skry"
    if [ "$(uname -s)" = Darwin ] && have xattr; then
        $sudo xattr -d com.apple.quarantine "$dir/skry" 2>/dev/null || true
    fi

    say "installed $("$dir/skry" --version) to $dir/skry"
    case ":$PATH:" in
        *":$dir:"*) ;;
        *) say "note: $dir is not on your PATH; add it, for example:"
           say "  echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile" ;;
    esac
    say "get started: skry --help   (or try the UI without servers: skry demo)"
}

main "$@"
