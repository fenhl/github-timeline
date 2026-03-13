#![allow(unused)]

use {
    std::{
        collections::{
            BTreeSet,
            btree_map::BTreeMap,
        },
        str::FromStr,
        time::Duration,
    },
    chrono::prelude::*,
    if_chain::if_chain,
    itertools::Itertools as _,
    lazy_regex::regex_captures,
    rand::prelude::*,
    serde::{
        Deserialize,
        Serialize,
    },
    url::Url,
    wheel::traits::{
        RequestBuilderExt as _,
        ReqwestResponseExt as _,
    },
};

#[derive(Serialize)]
struct DataPoint {
    day: String,
    open_issues: usize,
    open_prs: usize,
    issue_labels: BTreeMap<String, usize>,
    pr_labels: BTreeMap<String, usize>,
}

#[derive(Default, Deserialize, Serialize)]
struct Report {
    #[serde(default)]
    last_updated: BTreeMap<u32, DateTime<Utc>>,
    #[serde(default)]
    issue_events_cache: BTreeMap<u32, Vec<IssueEvent>>,
    #[serde(skip_deserializing)]
    labels: BTreeSet<String>,
    #[serde(skip_deserializing)]
    timeline: Vec<DataPoint>,
}

#[derive(Clone)]
struct Repo {
    org: String,
    repo: String,
}

#[derive(Debug, thiserror::Error)]
#[error("missing slash in GitHub repository")]
struct RepoParseError;

impl FromStr for Repo {
    type Err = RepoParseError;

    fn from_str(s: &str) -> Result<Self, RepoParseError> {
        let (org, repo) = s.split_once('/').ok_or(RepoParseError)?;
        Ok(Self {
            org: org.to_owned(),
            repo: repo.to_owned(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct Issue {
    html_url: Url,
    title: String,
}

#[derive(Deserialize, Serialize)]
struct IssueEvent {
    created_at: DateTime<Utc>,
    #[serde(flatten)]
    kind: IssueEventKind,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum IssueEventKind {
    Labeled {
        label: Label,
    },
    Unlabeled {
        label: Label,
    },
    Closed,
    Reopened,
    #[serde(
        alias = "assigned",
        alias = "base_ref_changed",
        alias = "base_ref_deleted",
        alias = "base_ref_force_pushed",
        alias = "comment_deleted",
        alias = "connected",
        alias = "convert_to_draft",
        alias = "demilestoned",
        alias = "head_ref_deleted",
        alias = "head_ref_force_pushed",
        alias = "head_ref_restored",
        alias = "issue_type_added",
        alias = "marked_as_duplicate",
        alias = "mentioned",
        alias = "merged",
        alias = "milestoned",
        alias = "pinned",
        alias = "ready_for_review",
        alias = "referenced",
        alias = "renamed",
        alias = "review_dismissed",
        alias = "review_request_removed",
        alias = "review_requested",
        alias = "subscribed",
        alias = "transferred",
        alias = "unassigned",
        alias = "unpinned",
        alias = "unsubscribed",
    )]
    Other,
}

#[derive(Deserialize, Serialize)]
struct Label {
    name: String,
}

#[derive(clap::Parser)]
struct Args {
    #[clap(default_value = "OoTRandomizer/OoT-Randomizer")]
    repos: Vec<Repo>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] HeaderToStr(#[from] reqwest::header::ToStrError),
    #[error(transparent)] InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[wheel::main]
async fn main(Args { repos }: Args) -> Result<(), Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(concat!("token ", env!("GITHUB_TOKEN")))?);
    let http_client = reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), " (", env!("CARGO_PKG_REPOSITORY"), ")"))
        .default_headers(headers)
        .timeout(Duration::from_secs(600))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build()?;
    for Repo { org, repo } in repos {
        let mut all_issues = Vec::default();
        //println!("{} Checking {org}/{repo}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let mut response = http_client.get(&format!("https://api.github.com/repos/{org}/{repo}/issues"))
            .send_github(true).await?;
        loop {
            if_chain! {
                if let Some(links) = response.headers().get(reqwest::header::LINK);
                if let Ok((_, next)) = links.to_str()?
                    .split(", ")
                    .filter_map(|link| regex_captures!("^<(.+)>; rel=\"next\"$", link))
                    .exactly_one();
                then {
                    let next = next.to_owned();
                    all_issues.extend(response.json_with_text_in_error::<Vec<Issue>>().await?);
                    //println!("{} Checking next {org}/{repo} page: {next}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
                    response = http_client.get(next)
                        .send_github(true).await?;
                } else {
                    all_issues.extend(response.json_with_text_in_error::<Vec<Issue>>().await?);
                    break
                }
            }
        }
        let mut rng = thread_rng();
        while !all_issues.is_empty() {
            let Issue { html_url, title, .. } = all_issues.remove(rng.gen_range(0..all_issues.len()));
            wheel::input!("random issue/PR: {html_url} {title}")?;
        }
    }
    Ok(())
}
