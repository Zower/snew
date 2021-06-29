//! Authentication towards the API.
use std::sync::RwLock;

use crate::reddit::{Error, Result};

use reqwest::{
    blocking::{Client, Response},
    header::AUTHORIZATION,
    StatusCode,
};
use serde::{Deserialize, Serialize};

/// Behavior of something that can provide access to the Reddit API.
pub trait Authenticator: std::fmt::Debug + 'static {
    /// Refresh/fetch the token from the Reddit API.
    fn login(&self, client: &Client) -> Result<()>;
    /// Provide a token to authenticate to the reddit API with.
    /// If this is invalid(outdated) or None, [`login`] should refresh it.
    fn token(&self) -> Option<Token>;
    /// This authenticator can make requests that pertain to a user, such as posting a comment etc.
    fn is_user(&self) -> bool;

    /// Convenience
    fn parse_response(&self, response: Response) -> Result<Token> {
        let status = response.status();
        let slice = &response.text()?;

        // Parse the response as JSON.
        if let Ok(token) = serde_json::from_str::<Token>(slice) {
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
                "Client ID or Secret are wrong. Reddit returned 401 Unauthorized",
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
}

/// An access token.
#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub expires_in: i32,
    scope: String,
    token_type: String,
}

/// Authenticated interaction with the Reddit API. Use [`crate::reddit::Reddit`] instead.
/// This is shared by all current interactors with what reddit calls 'things', so they can make requests for more posts, comments, etc.
#[derive(Debug)]
pub struct AuthenticatedClient {
    pub(crate) client: Client,
    pub(crate) authenticator: Box<dyn Authenticator>,
}

impl AuthenticatedClient {
    pub fn new<T: Authenticator>(authenticator: T, user_agent: &str) -> Result<Self> {
        let client = Self::make_client(user_agent)?;

        authenticator.login(&client)?;

        Ok(Self {
            authenticator: Box::new(authenticator) as Box<dyn Authenticator>,
            client,
        })
    }

    /// Make a get request to `url`
    /// Errors if the status code was unexpected, the client cannot re-initialize or make the request, or if the authentication fails.
    pub fn get<Q: Serialize>(&self, url: &str, queries: Option<&Q>) -> Result<Response> {
        // Make one request
        if let Some(token) = &self.authenticator.token() {
            let response = self.make_request(&self.client, token, url, queries)?;

            if self.check_auth(&response)? {
                return Ok(response);
            }
        }

        // Refresh token
        self.authenticator.login(&self.client)?;

        if let Some(ref token) = self.authenticator.token() {
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
        let bearer = format!("bearer {}", token.access_token);

        if let Some(queries) = queries {
            Ok(client
                .get(url)
                .header(AUTHORIZATION, bearer)
                .query(queries)
                .send()?)
        } else {
            Ok(client.get(url).header(AUTHORIZATION, bearer).send()?)
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
        Ok(Client::builder().user_agent(user_agent).build()?)
    }
}

/// Client ID and Secret for the application
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub client_id: String,
    pub client_secret: String,
}

/// Login credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    client_info: ClientInfo,
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(client_id: &str, client_secret: &str, username: &str, password: &str) -> Self {
        Self {
            client_info: ClientInfo {
                client_id: String::from(client_id),
                client_secret: String::from(client_secret),
            },
            username: String::from(username),
            password: String::from(password),
        }
    }
}

/// Authenticator for Script applications.
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
            .basic_auth(
                self.creds.client_info.client_id.clone(),
                Some(self.creds.client_info.client_secret.clone()),
            )
            .send()?;

        *self
            .token
            .write()
            .expect("Poisoned RwLock, report bug at https://github.com/Zower/snew") =
            Some(self.parse_response(response)?);

        Ok(())
    }

    fn token(&self) -> Option<Token> {
        self.token
            .read()
            .expect("Poisoned mutex, report bug at https://github.com/Zower/snew")
            .clone()
    }

    fn is_user(&self) -> bool {
        true
    }
}

/// Anonymous authentication.
/// You will still need a client ID and secret, but you will not be logged in as some user. You can browse reddit, but not e.g. vote.
#[derive(Debug)]
pub struct AnonymousAuthenticator {
    client_info: ClientInfo,
    token: RwLock<Option<Token>>,
}

impl AnonymousAuthenticator {
    pub fn new(client_id: &str, client_secret: &str) -> Self {
        Self {
            client_info: ClientInfo {
                client_id: String::from(client_id),
                client_secret: String::from(client_secret),
            },
            token: RwLock::new(None),
        }
    }
}

impl Authenticator for AnonymousAuthenticator {
    fn login(&self, client: &Client) -> Result<()> {
        // Make the request for the access token.
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .query(&[("grant_type", "client_credentials")])
            .basic_auth(
                self.client_info.client_id.clone(),
                Some(self.client_info.client_secret.clone()),
            )
            .send()?;

        *self
            .token
            .write()
            .expect("Poisoned RwLock, report bug at https://github.com/Zower/snew") =
            Some(self.parse_response(response)?);
        Ok(())
    }
    fn token(&self) -> Option<Token> {
        self.token
            .read()
            .expect("Poisoned mutex, report bug at https://github.com/Zower/snew")
            .clone()
    }

    fn is_user(&self) -> bool {
        false
    }
}

// Reddit can return 200 OK even if the credentials are wrong, in which case it will include one field, "error": "message"
#[derive(Deserialize)]
struct OkButError {
    error: String,
}
