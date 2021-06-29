//! Reddit 'things'. In the API, a thing is a type + fullname.
use serde::Deserialize;

use self::raw::{
    comment::RawCommentData, generic_kind::RawKind, listing::RawListing, post::RawPostData,
};
use crate::{auth::AuthenticatedClient, reddit::Result};

use std::sync::Arc;

/// A handle to interact with a subreddit.
/// See [`PostFeed`] for some gotchas when iterating over Posts.
#[derive(Debug)]
pub struct Subreddit {
    pub name: String,
    pub url: String,
    pub(crate) client: Arc<AuthenticatedClient>,
}

impl Subreddit {
    /// Create a instance of a subreddit
    /// Use [`crate::reddit::Reddit::subreddit()`] instead.
    pub fn create(name: &str, client: Arc<AuthenticatedClient>) -> Self {
        Self {
            name: String::from(name),
            url: format!("{}r/{}", crate::reddit::URL, name),
            client,
        }
    }

    pub fn hot(&self) -> PostFeed {
        self.posts_sorted("hot")
    }

    // new() is usually reserved for creating a instance of the struct
    // Inconsistent to put new_sorting, and much easier to use this way than to use x_sorting for all the functions
    #[allow(clippy::clippy::new_ret_no_self)]
    pub fn new(&self) -> PostFeed {
        self.posts_sorted("new")
    }

    pub fn random(&self) -> PostFeed {
        self.posts_sorted("random")
    }

    pub fn rising(&self) -> PostFeed {
        self.posts_sorted("rising")
    }

    pub fn top(&self) -> PostFeed {
        self.posts_sorted("top")
    }

    pub fn best(&self) -> PostFeed {
        self.posts_sorted("best")
    }

    // /// Submit a text post.
    // pub fn submit(&self, title: &str, text: &str) -> Post<T> {
    //     self.client.get(
    //         &format!("{}/api/submit", crate::reddit::URL),
    //         Some(&[("sr", self.name)]),
    //     );
    //     todo!()
    // }

    fn posts_sorted(&self, path: &str) -> PostFeed {
        PostFeed {
            limit: 100,
            url: format!("{}/{}", self.url, path),
            cached_posts: Vec::new(),
            client: self.client.clone(),
            after: String::from(""),
        }
    }
}

/// A post.
#[derive(Debug, Clone)]
pub struct Post {
    client: Arc<AuthenticatedClient>,
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
    /// The subreddit this post belongs to
    pub subreddit: String,
    /// The unique base 36 ID of this post
    pub id: String,
    /// The 'kind'. This should always be t3. Combine with [`Self::id`] to get the fullname of this post.
    pub kind: String,
}

impl Post {
    /// Get the comments for this post.
    /// Currently these are only the top level comments.
    pub fn comments(&self) -> CommentFeed {
        CommentFeed {
            client: self.client.clone(),
            url: format!(
                "{}r/{}/comments/{}",
                crate::reddit::URL,
                self.subreddit,
                self.id
            ),
            cached_comments: Vec::new(),
        }
    }
}

/// Represents interacting with a set of posts, meant to be iterated over. As long as there are posts to iterate over, this iterator will continue. You may wish to take() some elements.
/// The iterator returns a Result<Post, Error>. The errors are either from the HTTP request or the JSON parsing.
#[derive(Debug)]
pub struct PostFeed {
    /// The amount of posts to request from the Reddit API. This does not mean you can only iterate over this many posts.
    /// The Iterator will simply make more requests if you iterate over more than this limit.
    /// You should set this to a specific number if you know that you will be making some exact number of requests < 100, so
    /// the iterator doesnt fetch more posts than it needs to. If you dont know how many you are iterating over, just leave it at the default
    /// which is 100, the max Reddit allows.
    pub limit: i32,
    url: String,
    cached_posts: Vec<Post>,
    client: Arc<AuthenticatedClient>,
    after: String,
}

impl Iterator for PostFeed {
    type Item = Result<Post>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cached_posts.pop().map(Ok).or_else_transpose(|| {
            let text = self
                .client
                .get(
                    &self.url,
                    Some(&[
                        ("limit", self.limit.to_string()),
                        ("after", self.after.clone()),
                    ]),
                )?
                .text()?;

            let listing: RawListing<RawKind<RawPostData>> = serde_json::from_str(&text)?;

            // Make sure the next HTTP request gets posts after the last one we fetched.
            if let Some(after) = listing.data.pagination.after {
                self.after = after;
            }

            let client = &self.client;

            // Add posts to the cached_posts array, converting from RawPost to Post in the process
            self.cached_posts.extend(
                listing
                    .data
                    .children
                    .into_iter()
                    .rev()
                    .map(|raw| (raw, client.clone()))
                    .map(From::from),
            );
            Ok(self.cached_posts.pop())
        })
    }
}

/// A comment.
#[derive(Debug)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub id: String,
}

/// A set of comments, meant to be iterated over.
#[derive(Debug)]
pub struct CommentFeed {
    url: String,
    client: Arc<AuthenticatedClient>,
    cached_comments: Vec<Comment>,
}
impl Iterator for CommentFeed {
    type Item = Result<Comment>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cached_comments.pop().map(Ok).or_else_transpose(|| {
            let text = self.client.get(&self.url, None::<&()>)?.text()?;

            // The first listing returned by reddit is the post the comments belong to (smh..), the second listing are the comments.
            // So we just toss away all the json from the first element of the tuple.
            let listings: (Empty, RawListing<RawKind<RawCommentData>>) =
                serde_json::from_str(&text)?;

            // Add comments to the cached_commments array, converting from RawComment to Comment in the process
            self.cached_comments
                .extend(listings.1.data.children.into_iter().rev().map(From::from));

            Ok(self.cached_comments.pop())
        })
    }
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

pub trait Transpose<T> {
    fn or_else_transpose<F: FnOnce() -> Result<Option<T>>>(self, f: F) -> Option<Result<T>>;
}

impl<T> Transpose<T> for Option<Result<T>> {
    fn or_else_transpose<F: FnOnce() -> Result<Option<T>>>(self, f: F) -> Option<Result<T>> {
        if self.is_none() {
            f().transpose()
        } else {
            self
        }
    }
}

// fn or_else_transpose<T, F>(opt: Option<Result<T>>, f: F) -> Option<Result<T>>
// where
//     F: FnOnce() -> Result<Option<T>>,
// {
//     if let None = opt {
//         f().transpose()
//     } else {
//         opt
//     }
// }

// Create a post from som raw data.
impl From<(RawKind<RawPostData>, Arc<AuthenticatedClient>)> for Post {
    fn from(raw: (RawKind<RawPostData>, Arc<AuthenticatedClient>)) -> Self {
        let (raw, client) = raw;
        Self {
            client,
            title: raw.data.title,
            ups: raw.data.ups,
            downs: raw.data.downs,
            url: raw.data.url,
            author: raw.data.author,
            subreddit: raw.data.subreddit,
            selftext: raw.data.selftext,
            id: raw.data.id,
            kind: raw.kind,
        }
    }
}

// Create a comment from som raw data.
impl From<RawKind<RawCommentData>> for Comment {
    fn from(raw: RawKind<RawCommentData>) -> Self {
        Self {
            author: raw.data.author,
            id: raw.data.id,
            body: raw.data.body,
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

// Discard all the JSON data
#[derive(Deserialize, Debug)]
struct Empty {}

// The raw responses from Reddit. The interpreted structs like [`crate::things::Subreddit`] and [`crate::things::Post`] are meant to be used.
#[doc(hidden)]
pub mod raw {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct Pagination {
        pub(crate) after: Option<String>,
        pub(crate) before: Option<String>,
    }

    pub mod listing {
        use super::Pagination;
        use serde::Deserialize;

        // Listings from Reddit take this form.
        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListing<T> {
            pub(crate) data: RawListingData<T>,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListingData<T> {
            #[serde(flatten)]
            pub(crate) pagination: Pagination,
            pub(crate) children: Vec<T>,
        }
    }

    pub mod generic_kind {
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        pub struct RawKind<T> {
            pub(crate) data: T,
            pub(crate) kind: String,
        }
    }

    pub mod post {
        use serde::Deserialize;

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawPostData {
            pub(crate) title: String,
            pub(crate) ups: i32,
            pub(crate) downs: i32,
            pub(crate) url: String,
            pub(crate) author: String,
            pub(crate) subreddit: String,
            pub(crate) selftext: String,
            pub(crate) id: String,
        }
    }

    pub mod comment {
        use serde::Deserialize;

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawCommentData {
            pub(crate) author: String,
            pub(crate) body: String,
            pub(crate) id: String,
        }
    }
}
