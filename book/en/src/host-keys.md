# Host keys and known_hosts

## Why host keys matter

When you connect to `web1`, how do you know you are really talking to
`web1` and not to an attacker who redirected your traffic? The server proves
its identity with its **host key**. Your computer keeps a list of known host
keys in `~/.ssh/known_hosts`. If the key matches, it is really your server.

skry follows the same rules as OpenSSH, strictly.

## The three possible outcomes

| Situation | Without `--accept-new` | With `--accept-new` |
| --- | --- | --- |
| Key is in `known_hosts` and matches | connect | connect |
| Host is **not** in `known_hosts` at all | **refuse** ("unknown host key") | record the key, then connect |
| Host is in `known_hosts` with a **different** key | **refuse** ("HOST KEY MISMATCH") | **refuse** — always |
| Key is marked `@revoked` | **refuse** | **refuse** |

`--accept-new` behaves exactly like OpenSSH's
`StrictHostKeyChecking=accept-new`: it is safe for **first contact**, and it
never overrides a mismatch.

## First contact with new servers

Option A — connect once with `ssh` and answer "yes" after checking the
fingerprint. skry will then trust that host.

Option B — let skry record the keys:

```sh
skry --accept-new @new-servers
```

For real safety, compare the fingerprint skry shows (`SHA256:...`) with the
one printed on the server:

```sh
# on the server, e.g. via the cloud console
ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
```

New keys are appended to the first `known_hosts` file (normally
`~/.ssh/known_hosts`) as ordinary lines like `[203.0.113.10]:2222 ssh-ed25519 AAAA…`.
If you run skry against many new hosts at once, the writes are serialised so
lines never get mixed up.

## "HOST KEY MISMATCH" — what now?

```text
HOST KEY MISMATCH for web1: server offered SHA256:abc…, which differs from
/home/you/.ssh/known_hosts:17; possible man-in-the-middle, refusing to connect
```

Possible reasons, from harmless to serious:

1. The server was **reinstalled** or its SSH keys were regenerated.
2. The address now belongs to a **different machine** (cloud IPs get reused).
3. Someone is **intercepting** your connection.

Do not just delete the line blindly. Confirm with the server's owner or the
console that the key really changed, then remove the old entry and connect
again:

```sh
ssh-keygen -R web1                     # port 22
ssh-keygen -R "[203.0.113.10]:2222"    # other ports
skry --accept-new web1
```

The message tells you the file and line number of the old key.

## What skry understands in known_hosts

- Plain host names and addresses: `web1,203.0.113.10 ssh-ed25519 AAAA…`
- Non-standard ports: `[203.0.113.10]:2222 ssh-ed25519 AAAA…`
- **Hashed** entries (`|1|…|…`), as written by `HashKnownHosts yes` (the
  default on Debian and Ubuntu)
- Wildcards and negation: `*.example.com,!evil.example.com …`
- `@revoked` markers
- Multiple files: `~/.ssh/known_hosts` and `~/.ssh/known_hosts2`, or whatever
  `UserKnownHostsFile` names
- `HostKeyAlias` from `~/.ssh/config`

Not supported: `@cert-authority` (SSH host certificates) lines are ignored.
If your servers use host certificates only, add their plain keys, or connect
once with `--accept-new`.

## Key types

If `known_hosts` already has, say, an ECDSA key for a host, skry asks the
server for an ECDSA key first — just like OpenSSH — so a server that has
several key types does not trigger a false mismatch.

## Jump hosts

When you connect through a bastion, **both** the bastion and the target are
verified. The target's key is looked up under the name the bastion connects
to (for example `db1` or `10.0.5.20`), so it must be in your `known_hosts`
under that name.
