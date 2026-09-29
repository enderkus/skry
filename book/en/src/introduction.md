# Introduction

**skry** shows you what your Linux servers are doing — CPU, memory, disks,
network, processes, containers, failed services, security problems — for one
server or a hundred at once, **without installing anything on them**.

It connects with plain SSH, the same way you already log in, runs one small
read-only shell script, reads the answer, and draws it on your screen. When
you close skry, nothing is left behind on the servers.

![skry in the terminal](https://raw.githubusercontent.com/enderkus/skry/main/docs/demo.gif)

## Who is it for?

- **You manage a handful to a few hundred Linux servers** and want to see at a
  glance which ones are healthy, which are struggling and which are down.
- **You cannot or do not want to install monitoring agents** — maybe the
  servers belong to a customer, maybe change control is strict, maybe you just
  want something that works in two minutes.
- **You are debugging an incident** and need a live, fleet-wide view right
  now, plus a written snapshot of the situation afterwards.
- **You want quick answers** such as "which servers listen on port 5432?",
  "where is nginx running?", "which hosts have pending security updates?".

## What skry is not

- **Not a replacement for a full monitoring platform** with months of
  retention, dashboards for every metric and on-call scheduling. skry keeps
  24 hours of history by default and focuses on the here and now. It can,
  however, feed Prometheus if you have it.
- **Not a remote administration tool.** skry never changes anything on a
  server and deliberately has no feature to run commands across your fleet.
- **Not an agent.** Nothing runs on your servers between two measurements.

## The three promises

1. **Agentless.** Nothing is installed, copied or written on the servers. No
   temporary files, no sudo.
2. **Read-only.** skry only reads files like `/proc/meminfo` and runs
   harmless inspection commands like `df` and `ps`.
3. **Secure by default.** skry verifies server identities with your
   `known_hosts` file and refuses servers it does not know unless you
   explicitly allow it. It never stores passwords.

## How to read this documentation

- New to skry? Read **Getting started** from top to bottom. It assumes no
  prior knowledge beyond "I can log in to my servers with SSH". If even that
  is new, the [SSH basics](ssh-basics.md) page explains it.
- Want to understand exactly what skry does on your servers and how every
  number is computed? Read **How skry works**.
- Looking for a specific feature? Jump to **Using skry**.
- Something is not working? Go to [Troubleshooting](troubleshooting.md) and
  the [FAQ](faq.md).

Every page can be read in Turkish with the **Türkçe** link at the top right.
