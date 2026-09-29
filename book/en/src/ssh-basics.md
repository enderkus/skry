# SSH basics in five minutes

skry uses SSH exactly like the `ssh` command does. If you can type
`ssh myserver` and get a shell without typing a password, skry will work
with no extra setup. This page explains the pieces involved, in plain words,
so that error messages make sense later.

## SSH in one paragraph

SSH is an encrypted connection between your computer (the **client**) and a
server. Two questions are answered when you connect:

1. **"Is this really my server?"** — the server proves its identity with a
   **host key**. Your computer remembers host keys in a file called
   `known_hosts`.
2. **"Is this really you?"** — you prove your identity, usually with a
   **key pair** (a private key that stays on your computer and a public key
   placed on the server).

skry needs both questions answered, automatically, without a human typing
anything. That is why it relies on keys and never on passwords.

## Key pairs

A key pair is two files, for example:

| File | Where it lives | Secret? |
| --- | --- | --- |
| `~/.ssh/id_ed25519` | On your computer | **Yes.** Never share it. |
| `~/.ssh/id_ed25519.pub` | Its content goes into `~/.ssh/authorized_keys` on each server | No |

Don't have one yet? Create it:

```sh
ssh-keygen -t ed25519
```

Press Enter to accept the default location. You may set a **passphrase**
(a password that protects the private key file). skry supports
passphrase-protected keys: it asks for the passphrase once when it starts.

Copy the public key to a server:

```sh
ssh-copy-id deploy@203.0.113.10
```

Afterwards `ssh deploy@203.0.113.10` should log you in without asking for
the account password.

## The SSH agent

Typing a passphrase every time is tedious. An **SSH agent** is a small
program on your computer that holds your unlocked keys in memory and signs
login requests for you.

- macOS: the agent runs automatically; `ssh-add --apple-use-keychain ~/.ssh/id_ed25519`
  stores the passphrase in the keychain.
- Linux desktops usually start one automatically. Otherwise:
  `eval "$(ssh-agent)" && ssh-add`.
- Windows: enable the "OpenSSH Authentication Agent" service, then run
  `ssh-add`.

skry talks to the agent if one is running (it looks at the `SSH_AUTH_SOCK`
variable on Linux/macOS, and the OpenSSH agent pipe or Pageant on Windows).
If the agent holds your key, skry never needs your passphrase at all.

## known_hosts

The first time you connect to a server, `ssh` shows something like:

```text
The authenticity of host '203.0.113.10' can't be established.
ED25519 key fingerprint is SHA256:Zm9vYmFy...
Are you sure you want to continue connecting (yes/no)?
```

Answering "yes" writes the server's host key to `~/.ssh/known_hosts`. From
then on, if the server ever presents a different key, `ssh` refuses loudly —
someone might be impersonating your server.

skry reads the same `known_hosts` file. Servers you have already connected
to with `ssh` are trusted. For new servers skry **refuses to connect** unless
you add `--accept-new`, which records the key the first time (like answering
"yes"). A **changed** key is always refused. The details are on
[Host keys and known_hosts](host-keys.md).

## ~/.ssh/config: names instead of addresses

Instead of remembering `deploy@203.0.113.10 -p 2222`, you can give servers
names in `~/.ssh/config`:

```text
Host web1
    HostName 203.0.113.10
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host db-*
    User postgres
    ProxyJump bastion
```

Now `ssh web1` works — and so does `skry web1`. skry understands the
settings that matter for connecting:

| Setting | What it does |
| --- | --- |
| `Host` | The name(s) or pattern(s) this block applies to (`*` and `?` wildcards, `!` to exclude) |
| `HostName` | The real address to connect to |
| `User` | The account to log in as |
| `Port` | The SSH port (default 22) |
| `IdentityFile` | Which private key(s) to use |
| `IdentitiesOnly` | `yes` = only offer the keys listed in `IdentityFile` |
| `ProxyJump` | Go through another server first (a bastion) |
| `Include` | Read more config files |
| `UserKnownHostsFile` | Use a different `known_hosts` file |
| `HostKeyAlias` | Look the host up in `known_hosts` under another name |
| `ConnectTimeout` | Seconds to wait for a connection |

Anything else in the file (for example `ForwardAgent` or `LocalForward`) is
simply ignored by skry; it does not break anything.

## The five-second test

Before using skry, check each server once with plain SSH:

```sh
ssh web1 true && echo works
```

If that prints `works` without asking for a password, skry will work too.
If it asks for a password, set up a key first (see above). If it complains
about the host key, see [Host keys](host-keys.md).
