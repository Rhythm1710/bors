use serde::Serialize;
use url::Url;

#[derive(Serialize)]
pub struct GitHubUser {
    login: String,
    id: u64,
    node_id: String,
    avatar_url: Url,
    gravatar_id: String,
    url: Url,
    html_url: Url,
    followers_url: Url,
    following_url: Url,
    gists_url: Url,
    starred_url: Url,
    subscriptions_url: Url,
    organizations_url: Url,
    repos_url: Url,
    events_url: Url,
    received_events_url: Url,
    r#type: String,
    site_admin: bool,
}

impl GitHubUser {
    pub fn new(name: &str) -> GitHubUser {
        GitHubUser {
            id: 101,
            login: name.to_string(),
            node_id: "MDQ6VXNlcjQ1MzkwNTc=".to_string(),
            avatar_url: "https://avatars.githubusercontent.com/u/4539057?v=4"
                .parse()
                .unwrap(),
            gravatar_id: "".to_string(),
            url: format!("https://api.github.com/users/{name}")
                .parse()
                .unwrap(),
            html_url: format!("https://github.com/{name}").parse().unwrap(),
            followers_url: format!("https://api.github.com/users/{name}/followers")
                .parse()
                .unwrap(),
            following_url: format!("https://api.github.com/users/{name}/following{{/other_user}}")
                .parse()
                .unwrap(),
            gists_url: format!("https://api.github.com/users/{name}/gists{{/gist_id}}")
                .parse()
                .unwrap(),
            starred_url: format!("https://api.github.com/users/{name}/starred{{/owner}}{{/repo}}")
                .parse()
                .unwrap(),
            subscriptions_url: format!("https://api.github.com/users/{name}/subscriptions")
                .parse()
                .unwrap(),
            organizations_url: format!("https://api.github.com/users/{name}/orgs")
                .parse()
                .unwrap(),
            repos_url: format!("https://api.github.com/users/{name}/repos")
                .parse()
                .unwrap(),
            events_url: format!("https://api.github.com/users/{name}/events{{/privacy}}")
                .parse()
                .unwrap(),
            received_events_url: format!("https://api.github.com/users/{name}/received_events")
                .parse()
                .unwrap(),
            r#type: "User".to_string(),
            site_admin: false,
        }
    }
}

impl Default for GitHubUser {
    fn default() -> Self {
        Self::new("user")
    }
}
