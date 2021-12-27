#[cfg(test)]
mod tests {
    use crate::{
        auth::{ApplicationAuthenticator, Credentials, ScriptAuthenticator},
        reddit::{Reddit, Result},
    };

    use std::env;

    #[test]
    fn it_works() -> Result<()> {
        let username = &env::var("REDDIT_USERNAME").unwrap();

        let script_auth = ScriptAuthenticator::new(Credentials::new(
            &env::var("REDDIT_CLIENT_ID").unwrap(),
            &env::var("REDDIT_CLIENT_SECRET").unwrap(),
            &env::var("REDDIT_USERNAME").unwrap(),
            &env::var("REDDIT_PASSWORD").unwrap(),
        ));
        let reddit = Reddit::new(
            script_auth,
            &format!("Windows:snew:v0.1.0 (by /u/{})", username),
        )
        .unwrap();

        println!("{:?}", reddit.me()?);

        Ok(())
    }

    #[test]
    fn anonymous() -> Result<()> {
        let anon_auth = ApplicationAuthenticator::new("h_Il077pxG16K1PahrHvtA");

        let reddit = Reddit::new(anon_auth, "Windows:snew:v0.1.0 (by anonymous)").unwrap();

        for post in reddit.subreddit("rust").hot().take(1) {
            let post = post?;
            println!("Post: {:?}", post.title);
        }

        for post in reddit.frontpage().best().take(1) {
            let post = post?;
            println!("Frontpage post: {}", post.title);
        }

        Ok(())
    }

    #[test]
    fn comments() -> Result<()> {
        let username = &env::var("REDDIT_USERNAME").unwrap();

        let script_auth = ScriptAuthenticator::new(Credentials::new(
            &env::var("REDDIT_CLIENT_ID").unwrap(),
            &env::var("REDDIT_CLIENT_SECRET").unwrap(),
            username,
            &env::var("REDDIT_PASSWORD").unwrap(),
        ));

        let reddit = Reddit::new(
            script_auth,
            &format!("Windows:snew:v0.1.0 (by /u/{})", username),
        )
        .unwrap();

        let hot = reddit.subreddit("globaloffensive").hot();

        for post in hot.take(3) {
            let post = post?;
            println!("Post: {}", post.title);

            for comment in post.comments().take(1) {
                let comment = comment?;
                println!("Comment: {}", comment.body);
            }
        }

        Ok(())
    }

    #[test]
    #[should_panic]
    fn unauthorized_anonoymous() {
        let anon_auth = ApplicationAuthenticator::new(env::var("REDDIT_CLIENT_ID").unwrap());
        let reddit = Reddit::new(anon_auth, "Windows:snew:v0.1.0 (by anonymous)").unwrap();

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

        println!("{:?}", reddit);

        reddit.unwrap();
    }
}
