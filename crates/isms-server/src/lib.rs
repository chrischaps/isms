//! `isms-server`: society actors, the tick scheduler, and (from S1.3) the API.
//! One binary, subsystems by subcommand; see `main.rs`.

pub mod actor;
pub mod api;
pub mod auth;
pub mod error;
pub mod limiter;
pub mod mail;
pub mod runtime;
pub mod scheduler;
pub mod state;
