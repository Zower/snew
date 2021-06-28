//! Reddit API.
use crate::auth::{AuthenticatedClient, Authenticator};
use crate::things::*;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub const URL: &str = "https://oauth.reddit.com/";
/// Communicate with the Reddit API.
/// # Creating a script application
/// Go to [the reddit OAuth guide](https://github.com/reddit-archive/reddit/wiki/OAuth2-Quick-Start-Example#first-steps). Follow the instructions under "First Steps".
///
/// After following the instructions, you should be on [the reddit prefs page](https://www.reddit.com/prefs/apps). The client_id will be in the top left corner under the name you chose. The secret is marked clearly. Username and password are your regular login credentials.
/// # Usage
/// ```no_run
/// use snew::{reddit::Reddit, auth::{ScriptAuthenticator, Credentials}};
///
/// let script_auth = ScriptAuthenticator::new(Credentials::new(
///     "client_id",
///     "client_secret",
///     "username",
///     "password",
/// ));
///
/// let reddit = Reddit::new(
///     script_auth,
///     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)"
///     ).unwrap();
///
/// println!("{:?}", reddit.me().unwrap());
/// ```
/// See also [`Reddit::subreddit`].
#[derive(Debug, Clone)]
pub struct Reddit<T: Authenticator> {
    client: AuthenticatedClient<T>,
    url: String,
}

// The API calls.
impl<T: Authenticator> Reddit<T> {
    /// Creates a new API connection, using the given authenticator.
    pub fn new(authenticator: T, user_agent: &str) -> Result<Self> {
        let client = AuthenticatedClient::new(authenticator, user_agent)?;

        Ok(Self {
            client,
            url: String::from(URL),
        })
    }

    /// Get information about the user, useful for debugging.
    pub fn me(&self) -> Result<Me> {
        if self
            .client
            .authenticator
            .lock()
            .expect("Poisoned mutex, report bug at https://github.com/Zower/snew")
            .is_user()
        {
            Ok(serde_json::from_str(
                &self
                    .client
                    .get(&format!("{}{}", self.url, "api/v1/me"), None::<&()>)?
                    .text()?,
            )?)
        } else {
            Err(Error::NotLoggedInError)
        }
    }

    /// Create a handle into a specific subreddit.
    /// # Usage
    /// ```no_run
    /// # fn main() -> snew::reddit::Result<()> {
    /// # use snew::{reddit::Reddit, auth::{ScriptAuthenticator, Credentials}};
    /// # let script_auth = ScriptAuthenticator::new(Credentials::new(
    /// #    "client_id",
    /// #   "client_secret",
    /// #   "username",
    /// #   "password",
    /// # ));
    /// # let reddit = Reddit::new(
    /// #    script_auth,
    /// #    "<Operating system>:snew:v0.1.0 (by u/<reddit username>)"
    /// #    ).unwrap();
    /// // login process omitted
    ///
    /// let rust = reddit.subreddit("rust");
    ///
    /// // You probably want to take() some elements, otherwise the Iterator will go as long as there are posts.
    /// for post in rust.hot().take(20) {
    ///     let post = post?;
    ///     println!("{}", post.title);
    /// }
    /// // You can also set the request limit.
    /// // It changes how many posts are fetched from the Reddit API at once.
    /// let mut top = rust.top();
    /// top.limit = 25;
    ///
    /// for post in top.take(20) {
    ///     let post = post?;
    ///     println!("{}", post.selftext);
    /// }
    /// # Ok(())
    /// # }

    pub fn subreddit(&self, name: &str) -> Subreddit<T> {
        Subreddit::create(name, &self.client)
    }

    // /// Submit a text post.
    // /// Equivalent to calling [`Subreddit::submit`], prefer using that if you already have a handle into the subreddit.
    // pub fn submit(&self, subreddit: &str, title: &str, text: &str) -> Post<T> {
    //     Subreddit::create(
    //         &format!("{}r/{}", self.url, subreddit),
    //         &self.client,
    //     )
    //     .submit(title, text)
    // }
}

/// All errors that can occur when using Snew. The source error (e.g. from a separate library), if any, can be found by calling error.source().
#[derive(Error, Debug)]
pub enum Error {
    /// A reqwest error. Will make more specific
    #[error("Reqwest returned an error.\nCaused by:\t{0}")]
    RequestError(#[from] reqwest::Error),

    /// A authentication error
    #[error("Failed to authenticate towards the Reddit API.\nReason:\t{0}")]
    AuthenticationError(String),

    /// A JSON parsing error. Usually this means the response was missing some necessary fields, e.g. because you are not authenticated correctly.
    #[error(
        "Malformed JSON response from the Reddit API. Are you authenticated correctly?\nCaused by:\t{0}"
    )]
    APIParseError(#[from] serde_json::Error),

    /// A invalid user_agent, usually
    #[error("Invalid header value. Either your user agent is malformed (only ASCII 32-127 allowed), or Reddit is returning disallowed characters in the access token. \nCaused by:\t{0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    /// This error occurs if you attempt to make some request that requires you to be logged in (e.g. script authentication), but you are authenticated anonymously.
    #[error("This action is only allowed when logged in, not with anonymous authentication.")]
    NotLoggedInError,
}
