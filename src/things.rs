use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Post {}
#[derive(Debug, Deserialize)]
pub struct Me {
    name: String,
    total_karma: i32,
    link_karma: i32,
    comment_karma: i32,
    verified: bool,
}
