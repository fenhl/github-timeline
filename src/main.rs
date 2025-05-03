use {
    std::{
        collections::{
            BTreeSet,
            HashSet,
            btree_map::{
                self,
                BTreeMap,
            },
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
        traits::{
            IoResultExt as _,
            RequestBuilderExt as _,
            ReqwestResponseExt as _,
        },
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
    number: u32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    pull_request: Option<serde_json::Value>,
    events_url: Url,
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

impl Label {
    #[must_use]
    fn map(&self, org: &str, repo: &str) -> &str {
        match (org, repo) {
            ("OoTRandomizer", "OoT-Randomizer") => self.ootr_map(),
            ("midoshouse", "ootr-multiworld") => self.mhmw_map(),
            _ => &self.name,
        }
    }

    #[must_use]
    fn ootr_map(&self) -> &str {
        match &*self.name {
            | "Changes Item Table"
                => "Changes Item Table",
            | "Algorithm Changes"
            | "Component: Algorithm"
                => "Component: Algorithm",
            | "ASM/C Changes"
            | "Component: ASM/C"
                => "Component: ASM/C",
            | "Component: Cosmetics"
                => "Component: Cosmetics",
            | "Component: Documentation"
                => "Component: Documentation",
            | "Component: GUI/Website"
                => "Component: GUI/Website",
            | "Component: Hints"
                => "Component: Hints",
            | "Component: Logic"
            | "Logic Changes"
                => "Component: Logic",
            | "Component: Misc"
                => "Component: Misc",
            | "Component: Patching"
                => "Component: Patching",
            | "Component: Plandomizer"
                => "Component: Plandomizer",
            | "Component: Presets"
                => "Component: Presets",
            | "Component: Randomizer Core"
                => "Component: Randomizer Core",
            | "Component: Setting"
                => "Component: Setting",
            | "Component: Tricks/Glitches"
                => "Component: Tricks/Glitches",
            | "Racing Impact"
                => "Racing Impact",
            | "Status: Blocked"
                => "Status: Blocked",
            | "Status: Duplicate"
            | "duplicate"
                => "Status: Duplicate",
            | "Status: Good First Issue"
            | "good first issue"
                => "Status: Good First Issue",
            | "Status: Help Wanted"
            | "help wanted"
                => "Status: Help Wanted",
            | "Needs Review"
            | "Status: Needs Review"
                => "Status: Needs Review",
            | "Status: Needs Testing"
                => "Status: Needs Testing",
            | "Status: Under Consideration"
                => "Status: Under Consideration",
            | "Status: Waiting for Author"
            | "Waiting for Author"
            | "question"
                => "Status: Waiting for Author",
            | "Status: Waiting for Maintainers"
                => "Status: Waiting for Maintainers",
            | "Status: Waiting for Release"
                => "Status: Waiting for Release",
            | "Status: Won't Fix"
            | "wontfix"
                => "Status: Won't Fix",
            | "Trivial"
            | "trivial"
                => "Trivial",
            | "Type: Bug"
            | "bug"
                => "Type: Bug",
            | "Type: Enhancement"
            | "enhancement"
                => "Type: Enhancement",
            | "Type: Maintenance"
                => "Type: Maintenance",
            _ => &self.name,
        }
    }

    #[must_use]
    fn mhmw_map(&self) -> &str {
        match &*self.name {
            | "component: GUI"
            | "component: gui"
                => "component: GUI",
            | "component: installer"
                => "component: installer",
            | "component: server"
                => "component: server",
            | "component: updater"
                => "component: updater",
            | "bizhawk"
            | "frontend: BizHawk"
            | "platform: BizHawk"
                => "frontend: BizHawk",
            | "frontend: EverDrive"
            | "platform: EverDrive"
                => "frontend: EverDrive",
            | "frontend: Project64"
            | "project64"
                => "frontend: Project64",
            | "frontend: RetroArch"
            | "platform: RetroArch"
                => "frontend: RetroArch",
            | "has workaround"
                => "has workaround",
            | "os: Linux"
                => "os: Linux",
            | "os: macOS"
                => "os: macOS",
            | "os: Windows"
                => "os: Windows",
            | "status: blocked"
                => "status: blocked",
            | "status: duplicate"
                => "status: duplicate",
            | "status: good first issue"
                => "status: good first issue",
            | "help wanted"
            | "status: help wanted"
                => "status: help wanted",
            | "status: in progress"
                => "status: in progress",
            | "status: invalid"
                => "status: invalid",
            | "status: pending release"
                => "status: pending release",
            | "status: question"
                => "status: question",
            | "status: released"
                => "status: released",
            | "status: wontfix"
                => "status: wontfix",
            | "bug"
            | "type: bug"
                => "type: bug",
            | "type: documentation"
                => "type: documentation",
            | "enhancement"
            | "type: enhancement"
                => "type: enhancement",
            | "type: maintenance"
                => "type: maintenance",
            _ => &self.name,
        }
    }
}

enum Event {
    IssueOpened(HashSet<String>),
    IssueClosed(HashSet<String>),
    PullRequestOpened(HashSet<String>),
    PullRequestClosed(HashSet<String>),
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
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("attempted to remove a label that wasn't present")]
    RemovedNonexistentLabel {
        events_url: Url,
        label: String,
    },
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
        let dir = Path::new("data").join(&org);
        let Report { mut last_updated, mut issue_events_cache, .. } = fs::read_json(dir.join(format!("{repo}.json"))).await.missing_ok()?;
        let mut events = BTreeMap::<_, Vec<_>>::default();
        let mut all_issues = Vec::default();
        println!("{} Checking {org}/{repo}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let mut response = http_client.get(&format!("https://api.github.com/repos/{org}/{repo}/issues"))
            .query(&[
                ("state", "all"),
            ])
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
                    println!("{} Checking next {org}/{repo} page: {next}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
                    response = http_client.get(next)
                        .send_github(true).await?;
                } else {
                    all_issues.extend(response.json_with_text_in_error::<Vec<Issue>>().await?);
                    break
                }
            }
        }
        for Issue { number, created_at, updated_at, pull_request, events_url } in all_issues {
            events.entry(created_at).or_default().push(if pull_request.is_some() {
                Event::PullRequestOpened(HashSet::default())
            } else {
                Event::IssueOpened(HashSet::default())
            });
            let mut labels = HashSet::new();
            println!("{} Checking {org}/{repo} issue: {events_url}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let issue_events = match issue_events_cache.entry(number) {
                btree_map::Entry::Occupied(mut entry) => if last_updated.get(&number).is_some_and(|&last_updated| last_updated == updated_at) {
                    entry.into_mut()
                } else {
                    let mut issue_events = http_client.get(events_url.clone())
                        .send_github(true).await?
                        .json_with_text_in_error::<Vec<IssueEvent>>().await?;
                    issue_events.sort_by_key(|IssueEvent { created_at, .. }| *created_at);
                    *entry.get_mut() = issue_events;
                    entry.into_mut()
                },
                btree_map::Entry::Vacant(entry) => {
                    let mut issue_events = http_client.get(events_url.clone())
                        .send_github(true).await?
                        .json_with_text_in_error::<Vec<IssueEvent>>().await?;
                    issue_events.sort_by_key(|IssueEvent { created_at, .. }| *created_at);
                    entry.insert(issue_events)
                }
            };
            let mut open = true;
            for IssueEvent { created_at, kind } in issue_events {
                match kind {
                    IssueEventKind::Labeled { label } => {
                        if labels.insert(label.map(&org, &repo).to_owned()) && open {
                            events.entry(*created_at).or_default().push(if pull_request.is_some() {
                                Event::PullRequestLabeled(label.map(&org, &repo).to_owned())
                            } else {
                                Event::IssueLabeled(label.map(&org, &repo).to_owned())
                            });
                        }
                    }
                    IssueEventKind::Unlabeled { label } => {
                        if !labels.remove(label.map(&org, &repo)) {
                            return Err(Error::RemovedNonexistentLabel { events_url, label: label.map(&org, &repo).to_owned() })
                        }
                        if open {
                            events.entry(*created_at).or_default().push(if pull_request.is_some() {
                                Event::PullRequestUnlabeled(label.map(&org, &repo).to_owned())
                            } else {
                                Event::IssueUnlabeled(label.map(&org, &repo).to_owned())
                            });
                        }
                    }
                    IssueEventKind::Closed => {
                        open = false;
                        events.entry(*created_at).or_default().push(if pull_request.is_some() {
                            Event::PullRequestClosed(labels.clone())
                        } else {
                            Event::IssueClosed(labels.clone())
                        });
                    }
                    IssueEventKind::Reopened => {
                        open = true;
                        events.entry(*created_at).or_default().push(if pull_request.is_some() {
                            Event::PullRequestOpened(labels.clone())
                        } else {
                            Event::IssueOpened(labels.clone())
                        });
                    }
                    IssueEventKind::Other => {}
                }
            }
            last_updated.insert(number, updated_at);
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
                    Event::IssueOpened(labels) => {
                        open_issues += 1;
                        for label in labels {
                            *issue_labels.entry(label).or_default() += 1;
                        }
                    }
                    Event::IssueClosed(labels) => {
                        open_issues -= 1;
                        for label in labels {
                            *issue_labels.entry(label).or_default() -= 1;
                        }
                    }
                    Event::PullRequestOpened(labels) => {
                        open_prs += 1;
                        for label in labels {
                            *pr_labels.entry(label).or_default() += 1;
                        }
                    }
                    Event::PullRequestClosed(labels) => {
                        open_prs -= 1;
                        for label in labels {
                            *pr_labels.entry(label).or_default() -= 1;
                        }
                    }
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
        fs::create_dir_all(&dir).await?;
        fs::write_json(dir.join(format!("{repo}.json")), Report {
            labels: issue_labels.into_keys().chain(pr_labels.into_keys()).collect(),
            last_updated, issue_events_cache, timeline,
        }).await?;
    }
    Ok(())
}
