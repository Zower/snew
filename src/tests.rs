#[cfg(test)]
mod tests {
    use crate::{
        auth::{ApplicationAuthenticator, Credentials, ScriptAuthenticator},
        reddit::{Reddit, Result},
    };

    use std::env;

    #[test]
    fn it_works() -> Result<()> {
        let script_auth = ScriptAuthenticator::new(Credentials::new(
            env::var("REDDIT_CLIENT_ID").unwrap().as_str(),
            env::var("REDDIT_CLIENT_SECRET").unwrap().as_str(),
            env::var("REDDIT_USERNAME").unwrap().as_str(),
            env::var("REDDIT_PASSWORD").unwrap().as_str(),
        ));
        let reddit = Reddit::new(script_auth, "Windows:snew:v0.1.0 (by /u/zower98)").unwrap();

        println!("{:?}", reddit.me()?);

        Ok(())
    }

    #[test]
    fn anonymous() -> Result<()> {
        let application_auth = ApplicationAuthenticator::new(
            env::var("REDDIT_CLIENT_ID").unwrap().as_str(),
            env::var("REDDIT_CLIENT_SECRET").unwrap().as_str(),
        );
        let reddit = Reddit::new(application_auth, "Windows:snew:v0.1.0 (by /u/zower98)").unwrap();

        for post in reddit.subreddit("all").new().take(1) {
            let post = post?;
            println!("{:?}", post.title);
        }

        Ok(())
    }

    #[test]
    fn comments() -> Result<()> {
        let script_auth = ScriptAuthenticator::new(Credentials::new(
            env::var("REDDIT_CLIENT_ID").unwrap().as_str(),
            env::var("REDDIT_CLIENT_SECRET").unwrap().as_str(),
            env::var("REDDIT_USERNAME").unwrap().as_str(),
            env::var("REDDIT_PASSWORD").unwrap().as_str(),
        ));

        let reddit = Reddit::new(script_auth, "Windows:snew:v0.1.0 (by /u/zower98)").unwrap();

        let global = reddit.subreddit("globaloffensive");

        for post in global.hot().take(3) {
            let post = post?;
            println!("Post: {}", post.title);
            for comment in post.comments().take(1) {
                let comment = comment?;
                println!("By: {}, {}", comment.author, comment.body);
            }
        }

        Ok(())
    }

    #[test]
    #[should_panic]
    fn unauthorized_anonoymous() {
        let application_auth = ApplicationAuthenticator::new(
            env::var("REDDIT_CLIENT_ID").unwrap().as_str(),
            env::var("REDDIT_CLIENT_SECRET").unwrap().as_str(),
        );
        let reddit = Reddit::new(application_auth, "Windows:snew:v0.1.0 (by /u/zower98)").unwrap();

        reddit.me().unwrap();
    }

    #[test]
    #[should_panic]
    fn not_authenticated() {
        let script_auth = ScriptAuthenticator::new(Credentials::new(
            "fake_client_id",
            "fake_client_secret",
            "fake_username",
            "fake_password",
        ));

        let reddit = Reddit::new(
            script_auth,
            "<Operating system>:snew:v0.1.0 (by /u/<reddit username>)",
        );

        reddit.unwrap();
    }
}
