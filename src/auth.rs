//! Authentication towards the API.
use std::cell::{RefCell, RefMut};

use crate::reddit::{Error, Result};

use reqwest::{
    blocking::{Client, Response},
    header, StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct AuthenticatedClient<T: Authenticator> {
    pub client: RefCell<Client>,
    pub authenticator: RefCell<T>,
    user_agent: String,
}

impl<T: Authenticator> AuthenticatedClient<T> {
    pub fn new(mut authenticator: T, user_agent: &str) -> Result<Self> {
        authenticator.login()?;

        if let Some(token) = authenticator.token() {
            let client = Self::make_client(user_agent, token.access_token.as_str())?;
            Ok(Self {
                authenticator: RefCell::new(authenticator),
                client: RefCell::new(client),
                user_agent: String::from(user_agent),
            })
        } else {
            // Temporary
            // Just until I implement non-password auth
            // *Should* never happen
            panic!("Token was not set after logging in");
        }
    }

    // Make a get request to `url`
    pub fn get<Q: Serialize>(&self, url: &str, queries: Option<&Q>) -> Result<Response> {
        // Make one request
        let mut client = self.client.borrow_mut();

        let response = self.make_request(&client, url, queries)?;

        // Check if the request was successful
        if self.check_auth(&response)? {
            Ok(response)
        } else {
            // Refresh token
            let mut authenticator = self.authenticator.borrow_mut();
            authenticator.login()?;

            if let Some(token) = authenticator.token() {
                // Create a new client with correct token
                *client = Self::make_client(self.user_agent.as_str(), token.access_token.as_str())?
            } else {
                // Temporary
                // Just until I implement non-password auth
                // *Should* never happen
                panic!("Token was not set after logging in");
            }

            let response = self.make_request(&client, url, queries)?;

            if response.status() == StatusCode::OK {
                Ok(response)
            }
            // Still not authenticated correctly
            else {
                Err(Error::AuthenticationError(String::from(
                    "Failed to authenticate, even after requesting new token. Check credentials.",
                )))
            }
        }
    }

    // Checks queries and makes the actual web request
    fn make_request<Q: Serialize>(
        &self,
        client: &RefMut<Client>,
        url: &str,
        queries: Option<&Q>,
    ) -> Result<Response> {
        if let Some(queries) = queries {
            Ok(client.get(url).query(queries).send()?)
        } else {
            Ok(client.get(url).send()?)
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

    // Make a reqwest client with user_agent and bearer token set as default headers.
    fn make_client(user_agent: &str, access_token: &str) -> Result<Client> {
        let mut headers = header::HeaderMap::new();

        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(format!("bearer {}", access_token).as_str())?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent)?,
        );

        Ok(Client::builder()
            .user_agent(user_agent)
            .default_headers(headers)
            .build()?)
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

/// Login credentials
#[derive(Debug)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
}

/// Behavior of something that can provide access to the Reddit API.
pub trait Authenticator {
    fn login(&mut self) -> Result<()>;
    fn token(&self) -> Option<Token>;
}

/// Authenticator for Script applications. See reddit OAuth API.
#[derive(Debug)]
pub struct ScriptAuthenticator {
    pub creds: Credentials,
    token: Option<Token>,
}

impl ScriptAuthenticator {
    pub fn new(creds: Credentials) -> Self {
        Self { creds, token: None }
    }

    fn default_agent() -> String {
        format!(
            "{}:{}:{}:{}",
            "desktop",
            "snew",
            env!("CARGO_PKG_VERSION"),
            "(by snewScriptAuthenticator)"
        )
    }
}

impl Authenticator for ScriptAuthenticator {
    fn login(&mut self) -> Result<()> {
        let client = Client::builder()
            .user_agent(ScriptAuthenticator::default_agent())
            .build()?;

        // Make the request for the access token.
        let response = client
            .post("https://www.reddit.com/api/v1/access_token")
            .query(&[
                ("grant_type", "password"),
                ("username", self.creds.username.as_str()),
                ("password", self.creds.password.as_str()),
            ])
            .basic_auth(
                self.creds.client_id.clone(),
                Some(self.creds.client_secret.clone()),
            )
            .send()?;

        let status = response.status();
        let text = response.text()?;
        let slice = text.as_str();

        // Parse the response as JSON.
        if let Ok(token) = serde_json::from_str::<Token>(slice) {
            self.token = Some(token);
        }
        // Various errors that can occur
        else if let Ok(error) = serde_json::from_str::<OkButError>(slice) {
            return Err(Error::AuthenticationError(format!(
                "{}, Reddit returned: {}",
                "Username or password are most likely wrong", error.error
            )));
        } else if status == StatusCode::UNAUTHORIZED {
            return Err(Error::AuthenticationError(String::from(
                "Client ID or Secret are wrong. Reddit returned 401 Unauthorized",
            )));
        }
        // Unknown what went wrong
        else {
            return Err(Error::AuthenticationError(format!(
                "Unexpected error occured, text: {}, code: {}",
                text,
                status.as_str()
            )));
        }

        Ok(())
    }

    fn token(&self) -> Option<Token> {
        self.token.clone()
    }
}

// Reddit can return 200 OK even if the credentials are wrong, in which case it will include one field, "error": "message"
#[derive(Deserialize)]
struct OkButError {
    error: String,
}
