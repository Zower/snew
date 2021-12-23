use bytes::Bytes;
use reqwest::blocking::Client;

use crate::reddit::{Error, Result};

#[derive(Debug)]
pub enum Content {
    Text(String),
    Html(String),
    Image(Bytes),
}

impl Content {
    pub fn parse(client: &Client, url: &str) -> Result<Content> {
        let response = client.get(url).send()?;
        let content_type = response.headers().get("Content-Type");

        if let Some(content_type) = content_type {
            let mut split = content_type.to_str().unwrap().split("/");
            if let Some(kind) = split.next() {
                if kind == "image" {
                    return Ok(Self::Image(response.bytes()?));
                } else if kind == "text" && split.next() == Some("html") {
                    return Ok(Self::Html(response.text()?));
                } else if kind == "text" {
                    return Ok(Self::Text(response.text()?));
                }
            }
        }

        Err(Error::NoReadableContent)
    }
}
