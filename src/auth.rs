//! Authentication towards the API.

use crate::reddit::{Error, Result};

use reqwest::{
    blocking::{Client, Response},
    header::{HeaderValue, AUTHORIZATION},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// An access token.
#[derive(Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub expires_in: i32,
    pub scope: String,
    pub token_type: String,
}

/// Authenticated interaction with the Reddit API. Use [`crate::reddit::Reddit`] instead.
/// This is shared by all current interactors with what reddit calls 'things', so they can make requests for more posts, comments, etc.
#[derive(Debug)]
pub struct AuthenticatedClient {
    pub(crate) client: Client,
    pub(crate) authenticator: RwLock<Box<dyn Authenticator>>,
}

impl AuthenticatedClient {
    pub fn new<T: Authenticator + 'static>(authenticator: T, user_agent: &str) -> Result<Self> {
        let client = Self::make_client(user_agent)?;

        authenticator.login(&client)?;

        Ok(Self {
            authenticator: RwLock::new(Box::new(authenticator) as Box<dyn Authenticator>),
            client,
        })
    }

    pub fn set_authenticator<T: Authenticator + 'static>(&self, authenticator: T) {
        *self.authenticator.write().unwrap() = Box::new(authenticator);
        // self.authenticator = Box::new(authenticator);
    }

    /// Make a get request to `url`
    /// Errors if the status code was unexpected, the client cannot re-initialize or make the request, or if the authentication fails.
    pub(crate) fn get<Q: Serialize>(&self, url: &str, queries: Option<&Q>) -> Result<Response> {
        // Make one request
        if let Some(token) = &self.authenticator.read().unwrap().token() {
            let response = self.make_request(&self.client, token, url, queries)?;

            if self.check_auth(&response)? {
                return Ok(response);
            }
        }

        // Refresh token
        self.authenticator.read().unwrap().login(&self.client)?;

        if let Some(ref token) = self.authenticator.read().unwrap().token() {
            let response = self.make_request(&self.client, token, url, queries)?;

            if response.status() == StatusCode::OK {
                Ok(response)
            } else {
                // Still not authenticated correctly
                Err(Error::AuthenticationError(String::from(
                    "Failed to authenticate, even after requesting new token. Check credentials.",
                )))
            }
        } else {
            // Pretty sure this can never happen, but better safe than sorry? :D
            Err(Error::AuthenticationError(String::from("Token was not set after logging in, but no error was returned. Report bug at https://github.com/Zower/snew")))
        }
    }

    // Checks queries and makes the actual web request
    fn make_request<Q: Serialize>(
        &self,
        client: &Client,
        token: &Token,
        url: &str,
        queries: Option<&Q>,
    ) -> Result<Response> {
        let mut authorization = HeaderValue::from_str(&format!("bearer {}", token.access_token))?;

        authorization.set_sensitive(true);

        if let Some(queries) = queries {
            Ok(client
                .get(url)
                .header(AUTHORIZATION, authorization)
                .query(queries)
                .send()?)
        } else {
            Ok(client
                .get(url)
                .header(AUTHORIZATION, authorization)
                .send()?)
        }
    }

    // Checks that the response is OK. Errors if status code is not expected.
    fn check_auth(&self, response: &Response) -> Result<bool> {
        let status = response.status();

        if status == StatusCode::OK {
            Ok(true)
        } else if status == StatusCode::FORBIDDEN || status == StatusCode::UNAUTHORIZED {
            Ok(false)
        } else {
            return Err(Error::AuthenticationError(format!(
                "Reddit returned an unexpected code: {}",
                status
            )));
        }
    }

    // Make a reqwest client with user_agent set as a default header.
    fn make_client(user_agent: &str) -> Result<Client> {
        Ok(Client::builder()
            .user_agent(user_agent)
            .gzip(true)
            .build()?)
    }
}

/// Login credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(
        client_id: impl ToString,
        client_secret: impl ToString,
        username: impl ToString,
        password: impl ToString,
    ) -> Self {
        Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

/// Behavior of something that can provide access to the Reddit API.
pub trait Authenticator: std::fmt::Debug + Send + Sync {
    /// Refresh/fetch the token from the Reddit API.
    fn login(&self, client: &Client) -> Result<()>;
    /// Provide a token to authenticate to the reddit API with.
    /// If this is invalid(outdated) or None, [`login`] should refresh it.
    fn token(&self) -> Option<Token>;
    /// This authenticator can make requests that pertain to a user, such as posting a comment etc.
    fn is_logged_in(&self) -> bool;
    /// Return a refresh token, if one exists.
    fn refresh_token(&self) -> Option<String>;
}

/// Authenticate on behalf of a user. Use this if you're creating a application that others will use, e.g. a desktop app.
#[derive(Debug)]
pub struct UserAuthenticator {
    // Duplicated because meh
    refresh_token: String,
    token: RwLock<Option<Token>>,
    /// Needed to get an access token, even if we have refresh_token
    client_id: String,
}

impl UserAuthenticator {
    pub fn new(refresh_token: impl ToString, client_id: impl ToString) -> Self {
        Self {
            refresh_token: refresh_token.to_string(),
            token: RwLock::new(None),
            client_id: client_id.to_string(),
        }
    }

    pub fn new_complete(
        refresh_token: impl ToString,
        client_id: impl ToString,
        token: Token,
    ) -> Self {
        Self {
            refresh_token: refresh_token.to_string(),
            token: RwLock::new(Some(token)),
            client_id: client_id.to_string(),
        }
    }
}

impl Authenticator for UserAuthenticator {
    fn login(&self, client: &Client) -> Result<()> {
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .query(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &self.refresh_token),
            ])
            .basic_auth(&self.client_id, None::<String>)
            .send()?;

        *self.token.write()? = Some(parse_response(response)?.into());

        Ok(())
    }

    fn token(&self) -> Option<Token> {
        self.token.read().unwrap().clone()
    }

    fn is_logged_in(&self) -> bool {
        self.token.read().unwrap().is_some()
    }

    fn refresh_token(&self) -> Option<String> {
        Some(self.refresh_token.clone())
    }
}

/// Authenticator for Script applications, e.g. bots or other apps that you control.
/// This includes username and password, which means you are logged in, and can perform actions such as voting.
/// See also reddit OAuth API docs.
#[derive(Debug)]
pub struct ScriptAuthenticator {
    creds: Credentials,
    token: RwLock<Option<Token>>,
}

impl ScriptAuthenticator {
    pub fn new(creds: Credentials) -> Self {
        Self {
            creds,
            token: RwLock::new(None),
        }
    }
}

impl Authenticator for ScriptAuthenticator {
    fn login(&self, client: &Client) -> Result<()> {
        // Make the request for the access token.
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .query(&[
                ("grant_type", "password"),
                ("username", &self.creds.username),
                ("password", &self.creds.password),
            ])
            .basic_auth(&self.creds.client_id, Some(&self.creds.client_secret))
            .send()?;

        *self.token.write()? = Some(parse_response(response)?.into());

        Ok(())
    }

    fn token(&self) -> Option<Token> {
        self.token.read().unwrap().clone()
    }

    fn is_logged_in(&self) -> bool {
        self.token.read().unwrap().is_some()
    }

    fn refresh_token(&self) -> Option<String> {
        None
    }
}

/// Anonymous application authentication.
/// You will not be logged in as some user, but as your application on behalf of the user. You can browse reddit, but not e.g. vote.
#[derive(Debug)]
pub struct ApplicationAuthenticator {
    client_id: String,
    token: RwLock<Option<Token>>,
}

impl ApplicationAuthenticator {
    pub fn new(client_id: impl ToString) -> Self {
        Self {
            token: RwLock::new(None),
            client_id: client_id.to_string(),
        }
    }
}

impl Authenticator for ApplicationAuthenticator {
    fn login(&self, client: &Client) -> Result<()> {
        // Make the request for the access token.
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .basic_auth(&self.client_id, None::<String>)
            .query(&[
                (
                    "grant_type",
                    "https://oauth.reddit.com/grants/installed_client",
                ),
                ("device_id", "DO_NOT_TRACK_THIS_DEVICE"),
            ])
            .send()?;

        *self.token.write()? = Some(parse_response(response)?.into());
        Ok(())
    }
    fn token(&self) -> Option<Token> {
        self.token.read().unwrap().clone()
    }

    fn is_logged_in(&self) -> bool {
        false
    }

    fn refresh_token(&self) -> Option<String> {
        None
    }
}

pub(crate) fn parse_response(response: Response) -> Result<TokenJson> {
    let status = response.status();
    let slice = &response.text()?;

    // Parse the response as JSON.
    if let Ok(token) = serde_json::from_str::<TokenJson>(slice) {
        Ok(token)
    }
    // Various errors that can occur
    else if let Ok(error) = serde_json::from_str::<OkButError>(slice) {
        Err(Error::AuthenticationError(format!(
            "{}, Reddit returned: {}",
            "Username or password are most likely wrong", error.error
        )))
    } else if status == StatusCode::UNAUTHORIZED {
        Err(Error::AuthenticationError(String::from(
            "Reddit returned 401 Unauthorized, are client ID and secret correct?",
        )))
    }
    // Unknown what went wrong
    else {
        return Err(Error::AuthenticationError(format!(
            "Unexpected error occured, text: {}, code: {}",
            slice, &status
        )));
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TokenJson {
    pub access_token: String,
    pub expires_in: i32,
    pub scope: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
}

impl From<TokenJson> for Token {
    fn from(token: TokenJson) -> Self {
        Self {
            access_token: token.access_token,
            expires_in: token.expires_in,
            scope: token.scope,
            token_type: token.token_type,
        }
    }
}

// Reddit can return 200 OK even if the credentials are wrong, in which case it will include one field, "error": "message"
#[derive(Deserialize)]
struct OkButError {
    error: String,
}
