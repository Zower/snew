//! Reddit API.
use crate::auth::{AuthenticatedClient, Authenticator};
use crate::things::*;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;
/// Communicate with the Reddit API.
/// # Creating a script application
/// Go to [the reddit OAuth guide](https://github.com/reddit-archive/reddit/wiki/OAuth2-Quick-Start-Example#first-steps). Follow the instructions under "First Steps".
///
/// After following the instructions, you should be on [the reddit prefs page](https://www.reddit.com/prefs/apps). The client_id will be in the top left corner under the name you chose. The secret is marked clearly. Username and password are your regular login credentials.
/// # Usage
/// ```no_run
/// use snew::{reddit::Reddit, auth::{ScriptAuthenticator, Credentials}};
///
/// let script_auth = ScriptAuthenticator::new(
///     Credentials {
///         client_id: String::from("client_id"),
///         client_secret: String::from("client_secret"),
///         username: String::from("reddit username"),
///         password: String::from("reddit password")
///     });
///
/// let reddit = Reddit::new(
///     script_auth,
///     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)"
///     ).unwrap();
///
/// println!("{:?}", reddit.me().unwrap());
/// ```
/// See also [`Reddit::subreddit`].
#[derive(Debug)]
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
            url: String::from("https://oauth.reddit.com/"),
        })
    }

    /// Get information about the user, useful for debugging.
    pub fn me(&self) -> Result<Me> {
        let response = self
            .client
            .get(format!("{}{}", self.url, "api/v1/me").as_str(), None::<&()>)?;
        Ok(response.json()?)
    }

    /// Create a handle into a specific subreddit.
    /// # Usage
    /// ```no_run
    /// # fn main() -> snew::reddit::Result<()> {
    /// # use snew::{reddit::Reddit, auth::{ScriptAuthenticator, Credentials}};
    /// # let script_auth = ScriptAuthenticator::new(
    /// #   Credentials {
    /// #        client_id: String::from("client_id"),
    /// #        client_secret: String::from("client_secret"),
    /// #        username: String::from("reddit username"),
    /// #        password: String::from("reddit password")
    /// #    });
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
        Subreddit::create(format!("{}r/{}", self.url, name).as_str(), &self.client)
    }
}

/// All errors that can occur when using Snew. The source error (e.g. from a separate library), if any, can be found by calling error.source().
#[derive(Error, Debug)]
pub enum Error {
    /// A reqwest error. Will make more specific
    #[error("Reqwest returned an error.\nCaused by:\t{0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to authenticate towards the Reddit API.\nReason:\t{0}")]
    AuthenticationError(String),
    /// A JSON parsing error. Usually this means the response was missing some necessary fields, e.g. because you are not authenticated correctly.
    #[error(
        "Malformed JSON response from the Reddit API. Are you authenticated correctly?\nCaused by:\t{0}"
    )]
    APIParseError(#[from] serde_json::Error),
    #[error("Invalid header value. Either your user agent is malformed (only ASCII 32-127 allowed), or Reddit is returning disallowed characters in the access token. \nCaused by:\t{0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    // KindParseError
}
