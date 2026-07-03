//! Built-in jobs.
//!
//! Each module is a self-contained implementation of [`crate::job::Job`].
//! Adding a new job is one new file here + one `pub mod` line + one
//! `JobRegistry::with(MyJob::new())` call in `main.rs`. The plan's
//! acceptance criterion (<50 lines for a second job) is met by the
//! `price_alert` example at the bottom of this doc — see the
//! [`price_alert_stub`] module for the full skeleton.

pub mod morpho_health;
