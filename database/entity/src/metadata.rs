use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel,Serialize, Deserialize)]
#[sea_orm(table_name = "metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub repo_id: String,
    pub evaluated_score: f64,
    pub popularity_score: f64,
    pub activity_score: f64,
    pub maintainability_score: f64,
    pub maturity_score: f64,
    pub openness_score: f64,
    pub growth_score: f64,
    pub compliance_score: f64,
    pub structure_score: f64,
    pub is_archived: bool,
    pub license_name: Option<String>,
    pub disk_usage: i32,
    pub stargazer_count: i32,
    pub fork_count: i32,
    pub watcher_count: i32,
    pub mentionable_user_count: i32,
    pub open_issues: i32,
    pub closed_issues: i32,
    pub open_pull_requests: i32,
    pub closed_pull_requests: i32,
    pub merged_pull_requests: i32,
    pub commit_count: i32,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
    pub pushed_at: Option<DateTime>,
    pub primary_language: Option<String>,
    pub release_count: i32,
    pub owner_type: String,
    pub language_total_count: i32,
    pub language_total_size: i32,
    pub languages_json: Option<String>, // 复杂结构序列化为json字符串
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
