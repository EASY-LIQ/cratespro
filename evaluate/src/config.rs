use anyhow::Context;
use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

/// 评价算法权重配置
#[derive(Debug, Deserialize, Clone)]
pub struct PassConfig {
    pub popularity_weight: f64,
    pub activity_weight: f64,
    pub maintainability_weight: f64,
    pub maturity_weight: f64,
    pub openness_weight: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseUrlConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PopularityConfig {
    pub star: f64,
    pub fork: f64,
    pub watch: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActivityConfig {
    pub pr: f64,
    pub contributors: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MaintainabilityConfig {
    pub pushed_at: f64,
    pub is_archived: f64,
    pub commit_totalcount: f64,
    pub releases_totalcount: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MaturityConfig {
    pub languages: f64,
    pub push_releases: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpennessConfig {
    pub license_info: f64,
}

/// 评价上下文结构体
#[derive(Debug, Deserialize, Clone)]
pub struct EvaluationContext {
    pub database_url: DatabaseUrlConfig,
    pub pass: PassConfig,
    pub popularity: PopularityConfig,
    pub activity: ActivityConfig,
    pub maintainability: MaintainabilityConfig,
    pub maturity: MaturityConfig,
    pub openness: OpennessConfig,
}

impl EvaluationContext {
    pub fn load_config(config_path: &str) -> anyhow::Result<Self> {
        Config::builder()
            .add_source(
                File::with_name(config_path)
                    .format(FileFormat::Toml)
                    .required(true),
            )
            .add_source(
                Environment::with_prefix("CRATESPRO")
                    .try_parsing(true)
                    .separator("_"),
            )
            .build()
            .with_context(|| anyhow::anyhow!("Failed to load config"))?
            .try_deserialize()
            .with_context(|| anyhow::anyhow!("Failed to deserialize config"))
    }
}
