use chrono::{DateTime, Utc};
use entity::{contributor_location, github_user, metadata, programs};
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// GitHub用户信息结构
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub bio: Option<String>,
    pub public_repos: Option<i32>,
    pub followers: Option<i32>,
    pub following: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "type")]
    pub user_type: String,
}

impl GitHubUser {
    pub fn is_bot(&self) -> bool {
        self.user_type == "Bot"
    }
}

// 转换函数，用于将GitHub API返回的用户转换为数据库模型
impl From<GitHubUser> for github_user::ActiveModel {
    fn from(user: GitHubUser) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: NotSet,
            github_id: Set(user.id),
            login: Set(user.login),
            name: Set(user.name),
            email: Set(user.email),
            avatar_url: Set(user.avatar_url),
            company: Set(user.company),
            location: Set(user.location),
            bio: Set(user.bio),
            public_repos: Set(user.public_repos),
            followers: Set(user.followers),
            following: Set(user.following),
            created_at: Set(user.created_at.naive_utc()),
            updated_at: Set(user.updated_at.naive_utc()),
            inserted_at: Set(now),
            updated_at_local: Set(now),
        }
    }
}

impl From<github_user::Model> for GitHubUser {
    fn from(value: github_user::Model) -> Self {
        Self {
            id: value.github_id,
            login: value.login,
            avatar_url: value.avatar_url,
            name: value.name,
            email: value.email,
            company: value.company,
            location: value.location,
            bio: value.bio,
            public_repos: value.public_repos,
            followers: value.followers,
            following: value.following,
            user_type: "User".to_owned(),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(value.updated_at, Utc),
        }
    }
}

pub struct AnalyzedUser {
    pub user_id: i32,
    pub github_id: i64,
    pub login: String,
    pub profile_email: Option<String>,
    pub commit_email: Option<String>,
}

impl From<github_user::Model> for AnalyzedUser {
    fn from(value: github_user::Model) -> Self {
        Self {
            user_id: value.id,
            github_id: value.github_id,
            login: value.login,
            profile_email: value.email,
            commit_email: None,
        }
    }
}
// 贡献者信息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contributor {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
    pub contributions: i32,
    pub email: Option<String>,
}

// 贡献者分析结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContributorAnalysis {
    pub has_china_timezone: bool,
    pub common_timezone: String,
}

// 转换函数，将分析结果转换为数据库模型
impl From<&ContributorAnalysis> for contributor_location::ActiveModel {
    fn from(analysis: &ContributorAnalysis) -> Self {
        let now = chrono::Utc::now().naive_utc();

        Self {
            id: NotSet,
            is_from_china: Set(analysis.common_timezone == "+08:00"),
            common_timezone: Set(Some(analysis.common_timezone.clone())),
            analyzed_at: Set(now),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub url: String,
    pub created_at: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLResponse {
    pub data: Option<SearchData>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchData {
    pub search: SearchResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub edges: Vec<Edge>,
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub node: Repository,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

// 解析提交数据
#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub login: String,
    pub id: i64,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitInfo {
    pub _author: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitDetail {
    pub author: Option<CommitInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CommitData {
    pub author: Option<CommitAuthor>,
    pub commit: CommitDetail,
}

#[derive(Debug, Deserialize)]
pub struct RestfulRepository {
    pub id: i32,
    pub node_id: String,
    pub name: String,
    pub html_url: String,
    pub created_at: String,
}

impl From<RestfulRepository> for programs::ActiveModel {
    fn from(item: RestfulRepository) -> Self {
        Self {
            id: Set(Uuid::new_v4()),
            github_url: Set(item.html_url),
            name: Set(item.name),
            description: Set("".to_owned()),
            namespace: Set("".to_owned()),
            max_version: Set("".to_owned()),
            mega_url: Set("".to_owned()),
            doc_url: Set("".to_owned()),
            program_type: Set("".to_owned()),
            downloads: Set(0),
            cratesio: Set("".to_owned()),
            repo_created_at: Set(Some(
                item.created_at
                    .parse::<DateTime<Utc>>()
                    .unwrap()
                    .naive_utc(),
            )),
            github_analyzed: Set(false),
            in_cratesio: Set(false),
            github_node_id: Set(item.node_id),
            updated_at: Set(Some(chrono::Utc::now().naive_utc())),
            metadata_update_state: Set(false),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubMetadataResponse {
    pub data: GithubMetadataData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubMetadataData {
    pub node: Option<GithubMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubMetadata {
    pub id: String,
    pub is_archived: bool,
    pub license_info: Option<LicenseInfo>,
    pub disk_usage: i32,
    pub stargazer_count: i32,
    pub fork_count: i32,
    pub watchers: Count,
    pub mentionable_users: Count,
    pub open_issues: Count,
    pub closed_issues: Count,
    pub open_pull_requests: Count,
    pub closed_pull_requests: Count,
    pub merged_pull_requests: Count,
    pub default_branch_ref: Option<DefaultBranchRef>,
    pub created_at: String,
    pub pushed_at: String,
    pub primary_language: Option<Language>,
    pub releases: Count,
    pub owner: Owner,
    pub languages: Languages,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    pub total_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultBranchRef {
    pub target: CommitTarget,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitTarget {
    pub history: History,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub total_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    #[serde(rename = "__typename")]
    pub typename: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Languages {
    pub total_count: i32,
    pub total_size: i32,
    pub edges: Vec<LanguageEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageEdge {
    pub size: i32,
    pub node: Language,
}

impl From<GithubMetadata> for metadata::ActiveModel {
    fn from(repo: GithubMetadata) -> Self {
        Self {
            repo_id: Set(repo.id),
            updated_at: Set(Some(Utc::now().naive_utc())),
            is_archived: Set(repo.is_archived),
            license_name: Set(repo.license_info.map(|l| l.name)),
            disk_usage: Set(repo.disk_usage),
            stargazer_count: Set(repo.stargazer_count),
            fork_count: Set(repo.fork_count),
            watcher_count: Set(repo.watchers.total_count),
            mentionable_user_count: Set(repo.mentionable_users.total_count),
            open_issues: Set(repo.open_issues.total_count),
            closed_issues: Set(repo.closed_issues.total_count),
            open_pull_requests: Set(repo.open_pull_requests.total_count),
            closed_pull_requests: Set(repo.closed_pull_requests.total_count),
            merged_pull_requests: Set(repo.merged_pull_requests.total_count),
            commit_count: Set(repo
                .default_branch_ref
                .map_or(0, |ref_branch| ref_branch.target.history.total_count)),
            created_at: Set(Some(
                repo.created_at
                    .parse::<DateTime<Utc>>()
                    .unwrap()
                    .naive_utc(),
            )),
            pushed_at: Set(Some(
                repo.pushed_at.parse::<DateTime<Utc>>().unwrap().naive_utc(),
            )),
            primary_language: Set(repo.primary_language.map(|l| l.name)),
            release_count: Set(repo.releases.total_count),
            owner_type: Set(repo.owner.typename),
            language_total_count: Set(repo.languages.total_count),
            language_total_size: Set(repo.languages.total_size),
            languages_json: Set(Some(serde_json::to_string(&repo.languages.edges).unwrap())),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubErrorResponse {
    pub errors: Option<Vec<GitHubError>>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubError {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub type_: String,
}
