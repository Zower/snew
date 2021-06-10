//! A Rust wrapper for the Reddit API.
//! This crate is not ready for use. For missing features, see the [Github issues page.](https://github.com/Zower/snew/issues)
//! # Example usage
//! ```no_run
//! use snew::{reddit::Reddit, auth::{ScriptAuthenticator, Credentials}};
//!
//! let script_auth = ScriptAuthenticator::new(
//!     Credentials {
//!         client_id: String::from("client_id"),
//!         client_secret: String::from("client_secret"),
//!         username: String::from("reddit username"),
//!         password: String::from("reddit password")
//!     });
//!
//! let reddit = Reddit::new(
//!     script_auth,
//!     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)").unwrap();
//!
//!  println!("{:?}", reddit.me().unwrap());
//! ```
//!
//! See also [`reddit::Reddit`] for more examples.
// #![deny(clippy::all)]
#![deny(
    missing_debug_implementations,
//     unconditional_recursion,
//     future_incompatible,
//     missing_docs
)]
// #![deny(unsafe_code)]
pub mod auth;
pub mod reddit;
pub mod things;
mod unsafe_tests;
