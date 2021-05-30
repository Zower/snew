use crate::auth::{Authenticator, Credentials, ScriptAuth, Token};
use crate::things;

use reqwest::{
    blocking::{Client, Response},
    header,
};

/// Communicate with the Reddit API.
/// # Creating a script application
/// Go to https://www.reddit.com/prefs/apps and create a new application.
/// Give it the name 'snew' and whatever description and about url (can be empty) you like, and use http://www.example.com/unused/redirect/uri as the redirect uri.
/// The client_id will be in the top left corner under the name. The secret is marked clearly. Username and password are your regular login credentials.
/// ```no_run
/// use snew::{reddit::Reddit, auth::Credentials};
/// let reddit = Reddit::script(
///     Credentials {
///         client_id: String::from("client_id"),
///         client_secret: String::from("client_secret"),
///         username: String::from("reddit username"),
///         password: String::from("reddit password")
///     },
///     "<Operating system>:snew:v0.1.0 (by u/<reddit username>)"
///     )
///     .unwrap();
/// println!("{:?}", reddit.me());
/// ```
#[derive(Debug)]
pub struct Reddit {
    client: Client,
    token: Token,
    url: String,
}

// The API calls.
impl Reddit {
    /// Get information about the user, useful for debugging.
    pub fn me(&self) -> Result<things::Me, Error> {
        println!("{:?}", self.token);

        let response = self.get("api/v1/me")?;

        Ok(response
            .json::<things::Me>()
            .map_err(|err| Error::APIParseError(err))?)
    }

    // Make a get request to self.url with the given path.
    fn get(&self, path: &str) -> Result<Response, Error> {
        Ok(self
            .client
            .get(format!("{}{}", self.url, path))
            .send()
            .map_err(|err| Error::RequestError(err))?)
    }
}

// General implementations
impl Reddit {
    /// Creates a new API connection, using the given authenticator.
    pub fn new<T>(authenticator: T, user_agent: &str) -> Result<Self, Error>
    where
        T: Authenticator,
    {
        match authenticator.get_token() {
            Ok(token) => {
                let client = Reddit::make_client(user_agent, token.access_token.as_str());
                Ok(Self {
                    client,
                    token,
                    url: String::from("https://oauth.reddit.com/"),
                })
            }
            Err(err) => Err(err),
        }
    }

    /// Convenience method for creating a new script API connection.
    pub fn script(creds: Credentials, user_agent: &str) -> Result<Self, Error> {
        let script_auth = ScriptAuth::new(creds);

        Reddit::new(script_auth, user_agent)
    }

    fn make_client(user_agent: &str, access_token: &str) -> Client {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            // Unwrap is OK here, as the token always comes from Reddit, and as such, should never contain illegal characters.
            header::HeaderValue::from_str(format!("bearer {}", access_token).as_str()).unwrap(),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent)
                .expect("User agent can only contain visible ASCII characters (32-127)"),
        );

        Client::builder()
            .user_agent(user_agent)
            .default_headers(headers)
            .build()
            // Expect here, as I assume reqwest almost never fails to build the TLS backend etc.
            .expect("The reqwest backend failed to build. See the reqwest documentation (https://docs.rs/reqwest/0.11.3/reqwest/blocking/struct.Client.html#method.new).")
    }
}

/// All errors that can occur when using Snew. The source error (e.g. from a separate library), if any, can be found by calling error.source().
#[derive(Debug)]
pub enum Error {
    /// A HTTPS request error.
    RequestError(reqwest::Error),
    /// A JSON parsing error. Usually this means the response was missing some necessary fields, e.g. because you are not authenticated correctly.
    APIParseError(reqwest::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RequestError(err) => Some(err),
            Self::APIParseError(err) => Some(err),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::RequestError(err) => {
                write!(f, "Failed to make a HTTPS request. \nCaused by: {}", err)
            }
            Self::APIParseError(err) => write!(
                f,
                "Malformed response from the Reddit API. Are you authenticated correctly? \nCaused by: {}",
                err
            ),
        }
    }
}
