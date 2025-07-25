use crate::config::EvaluationContext;
use crate::pass::{AnyEvaluationPass, PassData};
use entity::metadata::Model as MetadataModel;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct ActivityData {
    pub pr_count: i32,
    pub contributor_count: i32,
}

impl From<&MetadataModel> for ActivityData {
    fn from(model: &MetadataModel) -> Self {
        Self {
            pr_count: model.open_pull_requests
                + model.closed_pull_requests
                + model.merged_pull_requests,
            contributor_count: model.mentionable_user_count,
        }
    }
}

pub struct Activity;

impl AnyEvaluationPass for Activity {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64 {
        let activity_data = (data as &dyn Any).downcast_ref::<ActivityData>().unwrap();
        ctx.pass.activity_weight
            * (activity_data.pr_count as f64 * ctx.activity.pr
                + activity_data.contributor_count as f64 * ctx.activity.contributors)
    }
    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData> {
        Box::new(ActivityData::from(model))
    }
    fn name(&self) -> &'static str {
        "activity_score"
    }
}
