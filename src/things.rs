use reqwest::blocking::Client;
use serde::Deserialize;

use crate::things::raw::subreddit::RawListing;

use self::raw::post::RawPost;

/// A handle to interact with a subreddit.
/// See the
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

    fn posts_sorted(&self, url: &str) -> Posts {
        Posts {
            url: format!("{}/{}", self.url, url),
            cached_posts: Vec::new(),
            client: self.client,
            after: String::from(""),
        }
    }
}

/// A set of posts, meant to be iterated over. This iterator never returns None. It always returns the next post unless the GET request fails.
/// # Panics
/// Currently, if you iterate over this, it may panic on you, for no good reason.
/// It is intended to implement Iterator better, but for now, make sure to use take() so you dont make too many requests. If it still panics, it's likely something is wrong with the GET request.
/// Try something simpler instead (like reddit.me()), to make sure your requests are working.
#[derive(Debug)]
pub struct Posts<'a> {
    pub url: String,
    cached_posts: Vec<Post>,
    client: &'a Client,
    after: String,
}

/// A post.
#[derive(Debug, Clone)]
pub struct Post {
    pub title: String,
    pub author: String,
    pub selftext: String,
    pub kind: String,
}

impl<'a> Iterator for Posts<'a> {
    type Item = Post;

    // Unsure how to handle potential failures better.
    // Could try using a Iterator that can fail, but seems like extra hastle for the user to match every next().
    fn next(&mut self) -> Option<Self::Item> {
        if self.cached_posts.is_empty() {
            let listing = self
                .client
                .get(self.url.as_str())
                // TODO: Limit should be configurable
                .query(&[("limit", "100")])
                .query(&[("after", self.after.as_str())])
                .send()
                // This
                .unwrap()
                .json::<RawListing>()
                // And this need to be looked at.
                .unwrap();

            self.after = listing.data.pagination.after;

            // Cache each RawPost from the listing, and convert them to a usable Post at the same time.
            for post in listing.data.children {
                self.cached_posts.push(post.into());
            }

            // Reverse it now, so we avoid self.cached_posts.remove(0) every time next() is called.
            self.cached_posts.reverse();
        }

        Some(self.cached_posts.pop().unwrap())
    }
}

#[derive(Debug, Deserialize)]
pub struct Me {
    pub name: String,
    pub total_karma: i32,
    pub link_karma: i32,
    pub comment_karma: i32,
    pub verified: bool,
}

impl From<RawPost> for Post {
    fn from(raw: RawPost) -> Self {
        Self {
            title: raw.data.title,
            author: raw.data.author,
            selftext: raw.data.selftext,
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

    pub mod subreddit {
        use super::Pagination;
        use serde::Deserialize;
        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListing {
            pub data: RawListingData,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawListingData {
            #[serde(flatten)]
            pub pagination: Pagination,
            pub children: Vec<super::post::RawPost>,
        }
    }

    pub mod post {
        use serde::Deserialize;

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawPost {
            pub data: RawPostData,
            pub kind: String,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub struct RawPostData {
            pub title: String,
            pub author: String,
            pub selftext: String,
            pub id: String,
        }
    }
}
