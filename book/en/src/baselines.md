# Baselines: "unusual for this host"

Static thresholds answer "is CPU above 80 %?". But 70 % CPU is normal for a
build server and alarming for a DNS server that usually idles at 3 %.
**Baselines** learn what is normal for each host and flag values that are
unusual *for that host*.

## The idea, in plain words

For every host and every metric, skry keeps two running numbers:

- the **usual value** (a moving average), and
- **how much it usually varies** (a moving standard deviation).

When a new value arrives, skry asks: *how many "usual variations" above the
usual value is this?* That number is the **z-score**.

```text
usual CPU for dns1: 3 %, usually varies by ±1.5 %
new value: 22 %
z = (22 − 3) / 5 = 3.8      ← the variation is floored at 5 points for CPU (see below)
3.8 > 3.5 → deviation: "cpu 22.0% vs usual 3.0% (z=3.8)"
```

A deviation turns the host **cyan** (`deviation`) if no threshold is
breached, shows `◆ cpu z3.8` on the tile, appears under *Problems* in the
details, and can send an alert.

## Which metrics

| Metric | Minimum variation used |
| --- | --- |
| CPU % | 5 percentage points |
| memory % | 2 percentage points |
| load per core | 0.25 |
| fullest disk % | 1 percentage point |
| network receive | 25 % of the usual value, at least 64 KiB/s |
| network send | 25 % of the usual value, at least 64 KiB/s |

The minimum variation stops perfectly flat metrics from producing huge
z-scores for tiny wobbles (a disk that is always at exactly 41 % would
otherwise flag 42 %).

**Only upward deviations are reported.** A server that suddenly goes quiet is
rarely an incident; the thresholds and the unreachable check cover real
outages.

## Warm-up

A baseline is not trusted until it has seen `baseline.warmup` samples
(default **300**, which is 10 minutes at the default 2-second interval).
During warm-up no deviations are reported for that host.

Baselines are saved in the history database, so after a restart skry
continues where it left off and does not need to warm up again. With history
disabled, every start begins from zero.

## Time of day

Many servers have daily rhythms: backups at 03:00, business traffic at noon.
With `baseline.hourly = true` (the default) skry keeps a **separate baseline
for every hour of the day**, in addition to the overall one:

- If the baseline for the current hour has finished its warm-up, the current
  value is compared with it: 80 % CPU at 03:00 is normal if it happens every
  night.
- Otherwise the overall baseline is used.

Because each hour only gets samples during that hour, hourly baselines need
a few days to fill in. The hour is your computer's local time.

## How fast it adapts

The moving average uses a smoothing factor `baseline.alpha` (default 0.02):
each new sample has a 2 % say. Roughly, the baseline "remembers" the last
50–100 samples, i.e. a few minutes at the default interval, for the overall
baseline — and correspondingly more days for the hourly ones. A new normal
(say, after a deployment that permanently raises memory) stops being flagged
after a while, because deviating values are also folded into the baseline.

## Tuning

```toml
[baseline]
enabled = true
alpha = 0.02        # higher = adapts faster, forgets faster
z_threshold = 3.5   # higher = fewer, more significant deviations
warmup = 300        # samples before judging
hourly = true
```

- Too many cyan tiles? Raise `z_threshold` to 4 or 5.
- Deviations stay flagged too long after a permanent change? Raise `alpha`
  (e.g. 0.05).
- Don't want this at all? `enabled = false`.
