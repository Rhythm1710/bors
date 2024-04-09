use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use arc_swap::ArcSwap;
use axum::async_trait;
use octocrab::models::{App, AppId, InstallationRepositories, Repository};
use octocrab::Octocrab;
use secrecy::{ExposeSecret, SecretVec};

use client::GithubRepositoryClient;

use crate::bors::event::PullRequestComment;
use crate::bors::{BorsState, RepositoryClient, RepositoryState};
use crate::database::{DbClient, SeaORMClient};
use crate::github::GithubRepoName;
use crate::permissions::TeamApiPermissionResolver;

pub mod client;
pub(crate) mod operations;

type GHRepositoryState = RepositoryState<GithubRepositoryClient>;

type RepositoryMap = HashMap<GithubRepoName, Arc<GHRepositoryState>>;

fn base_github_html_url() -> &'static str {
    "https://github.com"
}

fn base_github_url() -> &'static str {
    "https://api.github.com"
}

/// Provides access to managed GitHub repositories.
pub struct GithubAppState {
    app: App,
    client: Octocrab,
    repositories: ArcSwap<RepositoryMap>,
    db: Arc<SeaORMClient>,
}

impl GithubAppState {
    /// Loads repositories managed by the Bors GitHub app with the given ID.
    pub async fn load(
        app_id: AppId,
        private_key: SecretVec<u8>,
        db: SeaORMClient,
    ) -> anyhow::Result<GithubAppState> {
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.expose_secret().as_ref())
            .context("Could not encode private key")?;

        let client = Octocrab::builder()
            .app(app_id, key)
            .build()
            .context("Could not create octocrab builder")?;

        let app = client
            .current()
            .app()
            .await
            .context("Could not load Github App")?;

        let repositories = load_repositories(&client).await?;
        Ok(GithubAppState {
            app,
            client,
            repositories: ArcSwap::new(Arc::new(repositories)),
            db: Arc::new(db),
        })
    }
}

/// Loads repositories that are connected to the given GitHub App client.
pub async fn load_repositories(client: &Octocrab) -> anyhow::Result<RepositoryMap> {
    let installations = client
        .apps()
        .installations()
        .send()
        .await
        .context("Could not load app installations")?;

    let mut repositories = HashMap::default();
    for installation in installations {
        if let Some(ref repositories_url) = installation.repositories_url {
            let installation_client = client.installation(installation.id);

            match installation_client
                .get::<InstallationRepositories, _, ()>(repositories_url, None)
                .await
            {
                Ok(repos) => {
                    for repo in repos.repositories {
                        match create_repo_state(installation_client.clone(), repo.clone())
                            .await
                            .map_err(|error| {
                                anyhow::anyhow!(
                                    "Cannot load repository {:?}: {error:?}",
                                    repo.full_name
                                )
                            }) {
                            Ok(repo_state) => {
                                tracing::info!("Loaded repository {}", repo_state.repository);

                                if let Some(existing) = repositories
                                    .insert(repo_state.repository.clone(), Arc::new(repo_state))
                                {
                                    return Err(anyhow::anyhow!(
                                        "Repository {} found in multiple installations!",
                                        existing.repository
                                    ));
                                }
                            }
                            Err(error) => {
                                tracing::error!(
                                    "Could not load repository {}: {error:?}",
                                    repo.full_name.unwrap_or_default()
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(
                        "Could not load repositories of installation {}: {error:?}",
                        installation.id
                    );
                }
            };
        }
    }
    Ok(repositories)
}

async fn create_repo_state(
    repo_client: Octocrab,
    repo: Repository,
) -> anyhow::Result<GHRepositoryState> {
    let Some(owner) = repo.owner.clone() else {
        return Err(anyhow::anyhow!("Repository {} has no owner", repo.name));
    };

    let name = GithubRepoName::new(&owner.login, &repo.name);
    tracing::info!("Found repository {name}");

    let client = GithubRepositoryClient {
        client: repo_client,
        repo_name: name.clone(),
        repository: repo,
    };

    let config = match client.load_config().await {
        Ok(config) => {
            tracing::info!("Loaded repository config for {name}: {config:#?}");
            config
        }
        Err(error) => {
            return Err(anyhow::anyhow!(
                "Could not load repository config for {name}: {error:?}"
            ));
        }
    };

    let permissions_resolver = TeamApiPermissionResolver::load(name.clone())
        .await
        .map_err(|error| anyhow::anyhow!("Could not load permissions for {name}: {error:?}"))?;

    Ok(RepositoryState {
        repository: name,
        client,
        config: RwLock::new(config),
        permissions_resolver: Box::new(permissions_resolver),
    })
}

#[async_trait]
impl BorsState<GithubRepositoryClient> for GithubAppState {
    fn is_comment_internal(&self, comment: &PullRequestComment) -> bool {
        comment.author.html_url == self.app.html_url
    }

    fn get_repo_state(
        &self,
        repo: &GithubRepoName,
    ) -> Option<(
        Arc<RepositoryState<GithubRepositoryClient>>,
        Arc<dyn DbClient>,
    )> {
        self.repositories
            .load()
            .get(repo)
            .map(|repo| (Arc::clone(&repo), Arc::clone(&self.db) as Arc<dyn DbClient>))
    }

    fn get_all_repos(
        &self,
    ) -> (
        Vec<Arc<RepositoryState<GithubRepositoryClient>>>,
        Arc<dyn DbClient>,
    ) {
        (
            self.repositories.load().values().cloned().collect(),
            Arc::clone(&self.db) as Arc<dyn DbClient>,
        )
    }

    /// Re-download information about repositories connected to this GitHub app.
    async fn reload_repositories(&self) -> anyhow::Result<()> {
        self.repositories
            .store(Arc::new(load_repositories(&self.client).await?));
        Ok(())
    }
}
