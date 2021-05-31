use crate::reddit::Error;

use reqwest::blocking::Client;
use serde::Deserialize;

/// An access token.
#[derive(Debug, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub expires_in: i32,
    scope: String,
    token_type: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
}

pub trait Authenticator {
    fn get_token(&self) -> Result<Token, Error>;
}

#[derive(Debug)]
pub struct ScriptAuth {
    pub creds: Credentials,
}

impl ScriptAuth {
    pub fn new(creds: Credentials) -> Self {
        Self { creds }
    }

    fn default_agent() -> String {
        format!(
            "{}:{}:{}:{}",
            "desktop",
            "snew",
            env!("CARGO_PKG_VERSION"),
            "(by snewScriptAuth)"
        )
    }
}

impl Authenticator for ScriptAuth {
    fn get_token(&self) -> Result<Token, Error> {
        let client = Client::builder()
            .user_agent(ScriptAuth::default_agent())
            .build()
            // Expect here, as I assume reqwest almost never fails to build the TLS backend etc.
            .expect("The reqwest backend failed to build. See the reqwest documentation (https://docs.rs/reqwest/0.11.3/reqwest/blocking/struct.Client.html#method.new).");

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
            .send()
            .map_err(|err| Error::RequestError(err))?;

        // Parse the response as JSON.
        Ok(response
            .json::<Token>()
            .map_err(|err| Error::APIParseError(err))?)
    }
}
