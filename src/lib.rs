//! skry — agentless Linux server monitoring over plain SSH.
//!
//! The crate is organised as a small pipeline: [`ssh`] opens one session per
//! host, [`collect`] builds a single batched POSIX `sh` command and parses its
//! output, [`model`] turns raw counters into rates, and [`engine`] schedules
//! everything for a fleet. Consumers ([`tui`], [`web`], [`cli`]) read the
//! resulting state.
