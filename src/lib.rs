//! A Rust wrapper for the Reddit API.
//! This crate is not ready for use. For missing features, see the [Github issues page.](https://github.com/Zower/snew/issues)
//! # Example usage
//! ```no_run
//! use snew::{reddit::Reddit, auth::Credentials};
//! let reddit = Reddit::script(
//!     Credentials {
//!         client_id: String::from("client_id"),
//!         client_secret: String::from("client_secret"),
//!         username: String::from("reddit username"),
//!         password: String::from("reddit password")
//!    },
//!     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)"
//!     )
//!     .unwrap();
//! println!("{:?}", reddit.me());
//! ```
//!
//! See also [`reddit::Reddit`] for more examples.

pub mod auth;
pub mod reddit;
pub mod things;
