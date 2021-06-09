//! Reddit 'things'. In the API, a thing is a type + fullname.
use reqwest::blocking::Client;
use serde::Deserialize;

use self::raw::{listing::RawListing, generic_kind::RawKind, post::RawPostData};
use crate::reddit::{Error, Result};

/// A handle to interact with a subreddit.
/// See [`Posts`] for some gotchas when iterating over Posts.
#[derive(Debug)]
pub struct Subreddit<'a> {
    pub url: String,
    client: &'a Client,
}

impl<'a> Subreddit<'a> {
    pub fn create(url: &str, client: &'a Client) -> Self {
        Self {
            url: String::from(url),
            client,
        }
    }
    pub fn hot(&self) -> Posts {
        self.posts_sorted("hot")
    }
    pub fn new(&self) -> Posts {
        self.posts_sorted("new")
    }
    pub fn random(&self) -> Posts {
        self.posts_sorted("random")
    }
    pub fn rising(&self) -> Posts {
        self.posts_sorted("rising")
    }
    pub fn top(&self) -> Posts {
        self.posts_sorted("top")
    }

    /// Get an iterator over the comment trees of the supplied post.
    pub fn comments(&self, post: &Post) -> Comments {
        Comments {
            url: format!("{}/comments/{}", self.url, post.id.as_str()),
            client: self.client,
            cached_comments: Vec::new(),
        }

    }

    fn posts_sorted(&self, path: &str) -> Posts {
        Posts {
            limit: 100,
            url: format!("{}/{}", self.url, path),
            cached_posts: Vec::new(),
            client: self.client,
            after: String::from(""),
        }
    }
}

/// A post.
#[derive(Debug, Clone)]
pub struct Post {
    pub title: String,
    /// Upvotes.
    pub ups: i32,
    /// Downvotes.
    pub downs: i32,
    /// The associated URL of this post. It is an external website if the post is a link, otherwise the comment section.
    pub url: String,
    /// The author.
    pub author: String,
    /// The text of this post.
    pub selftext: String,
    /// The unique base 36 ID of this post
    pub id: String,
    /// The 'kind'. This should always be t3. Combine with [`Self::id`] to get the fullname of this post.
    pub kind: String,
}


/// Represents interacting with a set of posts, meant to be iterated over. As long as there are posts to iterate over, this iterator will continue. You may wish to take() some elements.
/// The iterator returns a Result<Post, Error>. The errors are either from the HTTP request or the JSON parsing.
#[derive(Debug)]
pub struct Posts<'a> {
    /// The amount of posts to request from the Reddit API. This does not mean you can only iterate over this many posts.
    /// The Iterator will simply make more requests if you iterate over more than this limit.
    /// You should set this to a specific number if you know that you will be making some exact number of requests < 100, so
    /// the iterator doesnt fetch more posts than it needs to. If you dont know how many you are iterating over, just leave it at the default
    /// which is 100, the max Reddit allows.
    pub limit: i32,
    url: String,
    cached_posts: Vec<Post>,
    client: &'a Client,
    after: String,
}

impl<'a> Iterator for Posts<'a> {
    type Item = Result<Post>;


    fn next(&mut self) -> Option<Self::Item> {
        if let Some(post) = self.cached_posts.pop() {
            Some(Ok(post))
        } else {
            let res = self
                .client
                .get(self.url.as_str())
                .query(&[("limit", self.limit)])
                .query(&[("after", self.after.as_str())])
                .send();

            // Probably some cleaner way to do this
            let listing = match res {
                Ok(response) => match response.json::<RawListing<RawKind<RawPostData>>>() {
                    Ok(raw) => raw,
                    Err(err) => return Some(Err(Error::APIParseError(err))),
                },
                Err(err) => return Some(Err(Error::RequestError(err))),
            };

            // Make sure the next HTTP request gets posts after the last one we fetched.
            self.after = listing.data.pagination.after;

            // Add posts to the cached_posts array, converting from RawPost to Post in the process
            self.cached_posts
                .extend(listing.data.children.into_iter().rev().map(From::from));

            let post = self.cached_posts.pop();
            if let Some(post) = post {
                Some(Ok(post))
            } else {
                None
            }
        }
    }
}

/// A comment.
#[derive(Debug)]
pub struct Comment {
    pub author: String,
}

#[derive(Debug)]
pub struct Comments<'a> {
    url: String,
    client: &'a Client,
    cached_comments: Vec<Comment>,
}

/// Information about the authenticated user
#[derive(Debug, Deserialize)]
pub struct Me {
    pub name: String,
    pub total_karma: i32,
    pub link_karma: i32,
    pub comment_karma: i32,
    pub verified: bool,
}


// Create a post from som raw data.
impl From<RawKind<RawPostData>> for Post {
    fn from(raw: RawKind<RawPostData>) -> Self {
        Self {
            title: raw.data.title,
            ups: raw.data.ups,
            downs: raw.data.downs,
            url: raw.data.url,
            author: raw.data.author,
            selftext: raw.data.selftext,
            id: raw.data.id,
            kind: raw.kind,
        }
    }
}

// Not used yet
// pub enum Kind {
//     Comment,
//     Account,
//     Link,
//     Message,
//     Subreddit,
//     Award,
// }

// impl std::convert::TryFrom<&str> for Kind {
//     type Error = crate::reddit::Error;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         match value {
//             "t1" => Ok(Self::Comment),
//             _ => Err(crate::reddit::Error::KindParseError),
//         }
//     }
// }

// The raw responses from Reddit. The interpreted structs like [`crate::things::Subreddit`] and [`crate::things::Post`] are meant to be used instead of these, and should cover regular usecases.
#[doc(hidden)]
pub mod raw {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct Pagination {
        pub after: String,
        // pub before: String,
    }

    pub mod listing {
        use super::Pagination;
        use serde::Deserialize;

        // Listings from Reddit take this form.
        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListing <T> {
            pub data: RawListingData<T>,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListingData <T> {
            #[serde(flatten)]
            pub pagination: Pagination,
            pub children: Vec<T>,
        }
    }

    pub mod generic_kind {
        use serde::Deserialize;
        
        #[derive(Debug, Deserialize)]
        pub struct RawKind <T> {
            pub data: T,
            pub kind: String,
        }
    }

    pub mod post {
        use serde::Deserialize;

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawPostData {
            pub title: String,
            pub ups: i32,
            pub downs: i32,
            pub url: String,
            pub author: String,
            pub selftext: String,
            pub id: String,
        }
    }

    pub mod comment {
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        pub struct RawComment {

        }
    }
}
