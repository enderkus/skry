# Bastions and ProxyJump

Many networks allow SSH only through one hardened entry server, a
**bastion** (or jump host). skry supports this with the standard
`ProxyJump` setting, exactly like `ssh -J`.

## Setting it up

In `~/.ssh/config`:

```text
Host bastion
    HostName bastion.example.com
    User ops

Host db1 db2 10.0.5.*
    ProxyJump bastion
    User postgres
```

Now `skry db1 db2` connects to the bastion first and reaches `db1` and `db2`
through it. Nothing extra is needed on the bastion either: it only has to
allow TCP forwarding (`AllowTcpForwarding yes`, the OpenSSH default; note
that Alpine's default config sets it to `no`).

## How it works

```text
your computer ══SSH══▶ bastion ──TCP tunnel inside SSH──▶ db1:22
      └──────────────── second SSH session, end to end ─────────┘
```

1. skry opens an SSH connection to the bastion, verifies its host key and
   logs in.
2. Through that connection it asks the bastion to open a TCP tunnel to
   `db1:22` (a "direct-tcpip" channel — the same thing `ssh -J` does).
3. It runs a **second, complete SSH session** through the tunnel, directly
   with `db1`: its own host key check, its own login. The bastion only
   passes encrypted bytes along; it cannot read them.
4. Your keys never leave your computer. No agent forwarding is used.

The bastion connection stays open as long as the target connection does.

## Chains

`ProxyJump` accepts several hops separated by commas, and a jump host may
itself have a `ProxyJump`:

```text
Host inner
    ProxyJump edge,middle
```

skry follows the chain in order (edge → middle → inner), up to 8 hops. A loop
(A jumps through B, B jumps through A) is detected and reported as a config
error.

`ProxyJump none` disables jumping for a host that would otherwise match a
wildcard rule.

## Things to know

- **Each hop has its own timeout** (`connect_timeout`, default 10 s), so the
  total time allowed grows with the number of hops.
- **Errors say where they happened**: `via jump host bastion: authentication failed …`
  means the bastion refused you, not the target.
- **Host keys for targets behind a bastion** are looked up under the name the
  bastion connects to (the `HostName` of the target, or the name itself).
  Use `--accept-new` once or connect with `ssh` once.
- **`ProxyCommand` is not supported.** Use `ProxyJump`, which covers the
  common cases (`ssh -W %h:%p bastion` is equivalent to `ProxyJump bastion`).
