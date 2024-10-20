use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, Debug)]
pub struct Repo {
    pub name: String,
    pub stars: u32,
    pub img: String,
    pub owner: String,
    pub repo_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Repos {
    pub all_public_dependent_repos: Vec<Repo>,
}

pub fn dependents_info(info: String) -> Result<Repos, Box<dyn Error>> {
    let repos: Repos = serde_json::from_str(&info).unwrap();
    Ok(repos)
}
