use crate::config::EvaluationContext;
use crate::pass::{AnyEvaluationPass, PassData};
use chrono::Utc;
use entity::metadata::Model as MetadataModel;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct MaintainabilityData {
    pub pushed_at_days: i64, // 距今天数
    pub is_archived: i64,
    pub commit_count: i32,
    pub releases_count: i32,
}

impl From<&MetadataModel> for MaintainabilityData {
    fn from(model: &MetadataModel) -> Self {
        let pushed_at_days = model
            .pushed_at
            .map(|dt| {
                let now = Utc::now().naive_utc();
                let duration = now - dt;
                duration.num_days()
            })
            .unwrap_or(0);
        Self {
            pushed_at_days,
            is_archived: model.is_archived as i64,
            commit_count: model.commit_count,
            releases_count: model.release_count,
        }
    }
}

pub struct Maintainability;

impl AnyEvaluationPass for Maintainability {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64 {
        let maintainability_data = (data as &dyn Any)
            .downcast_ref::<MaintainabilityData>()
            .unwrap();
        ctx.pass.maintainability_weight
            * (maintainability_data.pushed_at_days as f64 * ctx.maintainability.pushed_at
                + maintainability_data.is_archived as f64 * ctx.maintainability.is_archived
                + maintainability_data.commit_count as f64 * ctx.maintainability.commit_totalcount
                + maintainability_data.releases_count as f64
                    * ctx.maintainability.releases_totalcount)
    }
    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData> {
        Box::new(MaintainabilityData::from(model))
    }
    fn name(&self) -> &'static str {
        "maintainability_score"
    }
}
