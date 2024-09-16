use {
    std::{
        collections::{
            BTreeMap,
            BTreeSet,
            HashSet,
        },
        path::Path,
        str::FromStr,
        time::Duration,
    },
    chrono::prelude::*,
    if_chain::if_chain,
    itertools::Itertools as _,
    lazy_regex::regex_captures,
    serde::{
        Deserialize,
        Serialize,
    },
    url::Url,
    wheel::{
        fs,
        traits::ReqwestResponseExt as _,
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

#[derive(Serialize)]
struct Report {
    labels: BTreeSet<String>,
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
    created_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    pull_request: Option<serde_json::Value>,
    events_url: Url,
}

#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum IssueEvent {
    Labeled {
        created_at: DateTime<Utc>,
        label: Label,
    },
    Unlabeled {
        created_at: DateTime<Utc>,
        label: Label,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Label {
    name: String,
}

enum Event {
    IssueOpened,
    IssueClosed,
    PullRequestOpened,
    PullRequestClosed,
    IssueLabeled(String),
    IssueUnlabeled(String),
    PullRequestLabeled(String),
    PullRequestUnlabeled(String),
}

#[derive(clap::Parser)]
struct Args {
    repos: Vec<Repo>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] HeaderToStr(#[from] reqwest::header::ToStrError),
    #[error(transparent)] InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[wheel::main]
async fn main(Args { repos }: Args) -> Result<(), Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::AUTHORIZATION, reqwest::header::HeaderValue::from_str(concat!("token ", env!("GITHUB_TOKEN")))?);
    let http_client = reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
        .timeout(Duration::from_secs(600))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build()?;
    for Repo { org, repo } in repos {
        let mut events = BTreeMap::<_, Vec<_>>::default();
        let mut all_issues = Vec::default();
        println!("Checking {org}/{repo}");
        let mut response = http_client.get(&format!("https://api.github.com/repos/{org}/{repo}/issues"))
            .query(&[
                ("state", "all"),
            ])
            .send().await?
            .detailed_error_for_status().await?;
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
                    println!("Checking next {org}/{repo} page: {next}");
                    response = http_client.get(next)
                        .send().await?
                        .detailed_error_for_status().await?;
                } else {
                    all_issues.extend(response.json_with_text_in_error::<Vec<Issue>>().await?);
                    break
                }
            }
        }
        for Issue { created_at, closed_at, pull_request, events_url } in all_issues {
            events.entry(created_at).or_default().push(if pull_request.is_some() {
                Event::PullRequestOpened
            } else {
                Event::IssueOpened
            });
            let mut labels = HashSet::new();
            println!("Checking {org}/{repo} issue: {events_url}");
            let issue_events = http_client.get(events_url)
                .send().await?
                .detailed_error_for_status().await?
                .json_with_text_in_error::<Vec<IssueEvent>>().await?;
            for event in issue_events {
                match event {
                    IssueEvent::Labeled { created_at, label } => {
                        labels.insert(label.name.clone());
                        events.entry(created_at).or_default().push(if pull_request.is_some() {
                            Event::PullRequestLabeled(label.name)
                        } else {
                            Event::IssueLabeled(label.name)
                        });
                    }
                    IssueEvent::Unlabeled { created_at, label } => {
                        labels.remove(&label.name);
                        events.entry(created_at).or_default().push(if pull_request.is_some() {
                            Event::PullRequestUnlabeled(label.name)
                        } else {
                            Event::IssueUnlabeled(label.name)
                        });
                    }
                    IssueEvent::Other => {}
                }
            }
            if let Some(closed_at) = closed_at {
                let entry = events.entry(closed_at).or_default();
                entry.push(if pull_request.is_some() {
                    Event::PullRequestClosed
                } else {
                    Event::IssueClosed
                });
                entry.extend(labels.into_iter().map(if pull_request.is_some() {
                    Event::PullRequestUnlabeled
                } else {
                    Event::IssueUnlabeled
                }));
            }
        }
        let mut timeline = Vec::with_capacity(events.len());
        let mut open_issues = 0;
        let mut open_prs = 0;
        let mut issue_labels = BTreeMap::default();
        let mut pr_labels = BTreeMap::default();
        for (timestamp, events) in events {
            timeline.push(DataPoint {
                day: timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                issue_labels: issue_labels.clone(),
                pr_labels: pr_labels.clone(),
                open_issues, open_prs,
            });
            for event in events {
                match event {
                    Event::IssueOpened => open_issues += 1,
                    Event::IssueClosed => open_issues -= 1,
                    Event::PullRequestOpened => open_prs += 1,
                    Event::PullRequestClosed => open_prs -= 1,
                    Event::IssueLabeled(label) => *issue_labels.entry(label).or_default() += 1,
                    Event::IssueUnlabeled(label) => *issue_labels.entry(label).or_default() -= 1,
                    Event::PullRequestLabeled(label) => *pr_labels.entry(label).or_default() += 1,
                    Event::PullRequestUnlabeled(label) => *pr_labels.entry(label).or_default() -= 1,
                }
            }
            timeline.push(DataPoint {
                day: timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                issue_labels: issue_labels.clone(),
                pr_labels: pr_labels.clone(),
                open_issues, open_prs,
            });
        }
        timeline.push(DataPoint {
            day: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            issue_labels: issue_labels.clone(),
            pr_labels: pr_labels.clone(),
            open_issues, open_prs,
        });
        let dir = Path::new("data").join(org);
        fs::create_dir_all(&dir).await?;
        fs::write(dir.join(format!("{repo}.json")), serde_json::to_vec_pretty(&Report {
            labels: issue_labels.into_keys().chain(pr_labels.into_keys()).collect(),
            timeline,
        })?).await?;
    }
    Ok(())
}
