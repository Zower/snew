//! A Rust wrapper for the Reddit API.
//! This crate is not ready for use. For missing features, see the [Github issues page.](https://github.com/Zower/snew/issues)
//! # Example usage
//! Reddit requires you to be authenticated, but you can choose whether you wish to be 'logged in'.
//! You can do basic things like browsing without being logged in, but to create posts or vote etc. you need to be logged in.
//!
//! # Script authentication (logged in)
//! ```no_run
//! use snew::{reddit::Reddit, auth::{authenticator::ScriptAuthenticator, Credentials}};
//!
//! let script_auth = ScriptAuthenticator::new(Credentials::new(
//!     "client_id",
//!     "client_secret",
//!     "username",
//!     "password",
//! ));
//!
//! let reddit = Reddit::new(
//!     script_auth,
//!     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)").unwrap();
//!
//! // You cant do this without being logged in
//! println!("{:?}", reddit.me().unwrap());
//! ```
//! # Anonymous authentication (not logged in)
//! ```no_run
//! use snew::{reddit::Reddit, auth::{authenticator::AnonymousAuthenticator, Credentials}};
//!
//! let app_auth = AnonymousAuthenticator::new();
//!
//! let reddit = Reddit::new(
//!     app_auth,
//!     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)").unwrap();
//!
//! for post in reddit.subreddit("rust").new().take(5) {
//!     // do something    
//! }
//! ```
//! See also [`reddit::Reddit`] for more examples, and how to retrieve your client id and secret.
// #![deny(clippy::all)]
#![deny(
    missing_debug_implementations,
    unconditional_recursion,
    future_incompatible,
//     missing_docs
)]
#![deny(unsafe_code)]
pub mod auth;
pub mod reddit;
mod tests;
pub mod things;
