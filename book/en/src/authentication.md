# Authentication: agent, keys and passphrases

skry logs in with **SSH keys only**. It never asks for, uses or stores an
account password. (Keyboard-interactive, one-time passwords, GSSAPI/Kerberos
and SSH certificates are not supported either.)

## Where the keys come from

1. **Your SSH agent**, if one is running.
2. **Key files**: the `IdentityFile` entries from `~/.ssh/config` for that
   host. If a host has no `IdentityFile`, skry uses the default files that
   exist: `~/.ssh/id_ed25519`, `~/.ssh/id_ecdsa`, `~/.ssh/id_rsa` (the same
   defaults as OpenSSH).

## The order keys are tried in

```text
1. agent keys that match one of the host's IdentityFile keys
2. all other agent keys          (skipped if IdentitiesOnly yes)
3. key files that were not already offered through the agent
```

The first key the server accepts wins.

> [!WARNING]
> Servers allow only a limited number of attempts per connection
> (`MaxAuthTries`, default **6**). If your agent holds many keys, the server
> may disconnect before the right one is offered, and you see
> `authentication failed … server closed the connection`. Fix it by naming
> the right key and adding `IdentitiesOnly yes` for that host:
>
> ```text
> Host web1
>     IdentityFile ~/.ssh/work_ed25519
>     IdentitiesOnly yes
> ```

## Passphrase-protected keys

A passphrase protects the private key file. skry handles it like this, once,
**when it starts** (before the interface appears):

- If the key is **already loaded in your agent**, nothing is asked. skry
  recognises it by its public half.
- Otherwise skry asks:

  ```text
  Enter passphrase for key '/home/you/.ssh/id_ed25519':
  ```

  You get three attempts. An empty answer skips the key.
- The decrypted key is kept **in memory** for this run only. The passphrase
  itself is forgotten immediately and is never written or logged.

If skry is not running in an interactive terminal (a cron job, a CI
pipeline, a systemd service), it cannot ask. Encrypted keys that are not in
an agent are then skipped. For unattended use, load the key into an agent or
use a dedicated key without a passphrase that is only authorised for a
monitoring account.

## Which user?

In order of priority:

1. The user in the target: `deploy@web1`
2. `User` in `~/.ssh/config`
3. Your local user name (`$USER`, `%USERNAME%` on Windows)

## A dedicated monitoring account (recommended for teams)

skry does not need root. A normal account is enough, and a few group
memberships unlock more data (see [Permissions](permissions.md)). A typical
setup:

```sh
# on each server
sudo useradd -m -s /bin/sh skry
sudo usermod -aG adm skry          # read auth logs / journal (Debian, Ubuntu)
sudo mkdir -p ~skry/.ssh
echo 'ssh-ed25519 AAAA... monitoring' | sudo tee ~skry/.ssh/authorized_keys
sudo chown -R skry: ~skry/.ssh && sudo chmod 700 ~skry/.ssh && sudo chmod 600 ~skry/.ssh/authorized_keys
```

and on your computer:

```text
Host web* db*
    User skry
    IdentityFile ~/.ssh/skry_monitoring
    IdentitiesOnly yes
```

The login shell must be a real shell (`/bin/sh` or `/bin/bash`). Accounts
with `nologin`, `rbash` or a forced command in `authorized_keys` cannot run
the collection script.

## Turning the agent off

`--no-agent` makes skry ignore the agent and use key files only. Useful for
testing exactly which key works.
