//! skry — agentless Linux server monitoring over plain SSH.
//!
//! The crate is organised as a small pipeline: [`ssh`] opens one session per
//! host, [`collect`] builds a single batched POSIX `sh` command and parses its
//! output, [`model`] turns raw counters into rates, and [`engine`] schedules
//! everything for a fleet. Consumers ([`tui`], [`web`], [`cli`]) read the
//! resulting state.

pub mod alert;
pub mod baseline;
pub mod collect;
pub mod config;
pub mod engine;
pub mod model;
pub mod security;
pub mod snapshot;
pub mod ssh;
pub mod store;
pub mod tui;
pub mod util;
pub mod web;
