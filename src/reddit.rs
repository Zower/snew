//! Reddit API.
use crate::auth::{AuthenticatedClient, Authenticator, UserAuthenticator};
use crate::things::*;

use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub const URL: &str = "https://oauth.reddit.com";

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
pub struct Reddit {
    inner: Arc<AuthenticatedClient>,
}

// The API calls.
impl Reddit {
    /// Creates a new API connection, using the given authenticator.
    pub fn new<T: Authenticator + 'static>(authenticator: T, user_agent: &str) -> Result<Self> {
        let client = AuthenticatedClient::new(authenticator, user_agent)?;

        Ok(Self {
            inner: Arc::new(client),
        })
    }

    pub fn set_authenticator<T: Authenticator + 'static>(&mut self, authenticator: T) {
        self.inner.set_authenticator(authenticator);
    }

    /// Get information about the user, useful for debugging.
    pub fn me(&self) -> Result<Me> {
        if self.inner.authenticator.read().unwrap().is_logged_in() {
            Ok(serde_json::from_str(
                &self
                    .inner
                    .get(&format!("{}{}", URL, "/api/v1/me"), None::<&()>)?
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
    ///     println!("{:?}", post.selftext);
    /// }
    /// # Ok(())
    /// # }

    pub fn subreddit(&self, name: &str) -> Subreddit {
        Subreddit::create(name, self.inner.clone())
    }

    /// Posts from the frontpage.
    pub fn frontpage(&self) -> Subreddit {
        Subreddit {
            name: String::from("frontpage"),
            url: URL.to_string(),
            client: self.inner.clone(),
        }
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

    /// Returns a refresh token. Use this to store the refresh token for future use, e.g. on application shutdown.
    /// Returns none if the current authenticator has no refresh token assosciated with it.
    pub fn refresh_token(&self) -> Option<String> {
        self.inner
            .authenticator
            .read()
            .expect("Poisoned mutex")
            .refresh_token()
    }
}

#[cfg(feature = "code_flow")]
impl Reddit {
    /// A function that, from start to finish, performs the full OAuth2 code flow described in https://github.com/reddit-archive/reddit/wiki/OAuth2 and returns an Authenticator with a valid refresh token.
    /// You can retrieve the token for serialization later with [`Authenticator::get_token()`].
    ///
    /// You will need a application registered following the instructions in [`Reddit`], noting:
    /// * Choose a _installed app_
    /// * You MUST set the redirect URI to 'http://localhost:8080'.
    ///
    /// In full, this function will:
    /// * Spawn a web server on localhost, listening on port 8080.
    /// * Use the ```opener``` crate to open a URL in the users browser.
    /// * There, the user can accept that you would like to use Reddit on their behalf.
    /// * If they do, reddit makes a request to your local webserver, and gives us back a code.
    /// * We trade that code in for a refresh token.
    ///
    /// If you would rather do this work yourself, just get the refresh_token, and pass it to [`UserAuthenticator::new()`].
    pub fn perform_code_flow(
        client_id: impl std::fmt::Display,
        success_response: &'static str,
        timeout: Option<Duration>,
    ) -> std::result::Result<UserAuthenticator, Box<dyn std::error::Error + Send + Sync>> {
        use rand::Rng;
        use reqwest::blocking::Client;
        use rouille::{Response as RouilleResponse, Server};
        use std::sync::RwLock;

        use crate::auth::parse_response;

        let initial = Instant::now();
        // Somewhat jank structure that holds either (state, code) or an error.
        let result: Arc<RwLock<Option<std::result::Result<(String, String), String>>>> =
            Arc::new(RwLock::new(None));
        let copy = result.clone();

        // Spawn the server
        let server = Server::new("localhost:8080", move |request| {
            if let Some(error) = request.get_param("error") {
                let response = format!("Something went wrong: {}", error);
                *copy.write().unwrap() = Some(Err(error));

                return RouilleResponse::text(response);
            }

            let state = request.get_param("state");
            let code = request.get_param("code");

            if let Some(state) = state {
                if let Some(code) = code {
                    *copy.write().unwrap() = Some(Ok((state, code)));
                    return RouilleResponse::text(success_response);
                }
            }

            let error_msg = format!("Missing state or code parameter. This is a bug from Reddit. Try again. Parameters reddit returned: {}", request.raw_query_string());
            let response = RouilleResponse::text(&error_msg);

            *copy.write().unwrap() = Some(Err(error_msg));

            response
        })?;

        let (_, sender) = server.stoppable();

        let state: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(25)
            .map(char::from)
            .collect();

        let url = format!("https://www.reddit.com/api/v1/authorize?client_id={}&response_type=code\
                                    &state={}&redirect_uri=http://localhost:8080&duration=permanent&scope=*", client_id, state);

        // Open the url
        opener::open_browser(url)?;

        // Spin while waiting for request. Could be more efficient, send an issue if this is actually causing a problem for you, and I will fix it.
        while result.read().map_err(Into::<Error>::into)?.is_none() {
            if let Some(timeout) = timeout {
                if initial.elapsed() >= timeout {
                    break;
                }
            }
        }

        sender.send(())?;

        // Must be some by this point
        let result = result
            .write()
            .map_err(Into::<Error>::into)?
            .take()
            .unwrap()?;

        // Verify state
        if state == result.0 {
            let client = Client::new();

            // Finally, get the refresh token.
            let response = client
                .post("https://www.reddit.com/api/v1/access_token")
                .body(format!(
                    "grant_type=authorization_code&code={}&redirect_uri={}",
                    result.1, "http://localhost:8080"
                ))
                .basic_auth(&client_id, None::<String>)
                .send()?;

            let mut token = parse_response(response)?;

            Ok(UserAuthenticator::new_complete(
                token.refresh_token.take().unwrap(),
                client_id,
                token.into(),
            ))
        } else {
            Err(Box::new(CodeFlowError::StateDidNotMatch(state, result.0)))
        }
    }
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

    /// Poisoned RwLock. This shouldn't really happen.
    #[error("Poisoned RwLock, report bug at https://github.com/Zower/snew")]
    PoisonError,

    /// No content that snew knows how to handle.
    #[error("No parseable content found")]
    NoReadableContent,
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[cfg(feature = "code_flow")]
#[derive(Error, Debug)]
pub enum CodeFlowError {
    #[error("Received state did not match original state. Original:\t{0}\tReceived:\t{1}")]
    StateDidNotMatch(String, String),
    #[error("Missing state or code parameters. Received:\t{0}")]
    MissingParameters(String),
    #[error("Other error:\t{0}")]
    RedditError(#[from] Error),
}
