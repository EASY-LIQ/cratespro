use crate::config::EvaluationContext;
use crate::pass::{AnyEvaluationPass, PassData};
use entity::metadata::Model as MetadataModel;
use std::any::Any;
#[derive(Debug, Clone)]
pub struct PopularityData {
    pub stargazer_count: i32,
    pub fork_count: i32,
    pub watcher_count: i32,
}
impl From<&MetadataModel> for PopularityData {
    fn from(model: &MetadataModel) -> Self {
        Self {
            stargazer_count: model.stargazer_count,
            fork_count: model.fork_count,
            watcher_count: model.watcher_count,
        }
    }
}
pub struct Popularity;

impl AnyEvaluationPass for Popularity {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64 {
        let popularity_data = (data as &dyn Any).downcast_ref::<PopularityData>().unwrap();
        ctx.pass.popularity_weight
            * (popularity_data.stargazer_count as f64 * ctx.popularity.star
                + popularity_data.fork_count as f64 * ctx.popularity.fork
                + popularity_data.watcher_count as f64 * ctx.popularity.watch)
    }
    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData> {
        Box::new(PopularityData::from(model))
    }
    fn name(&self) -> &'static str {
        "popularity_score"
    }
}
