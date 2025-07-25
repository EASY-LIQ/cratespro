use crate::config::EvaluationContext;
use crate::pass::activity::Activity;
use crate::pass::maintainability::Maintainability;
use crate::pass::maturity::Maturity;
use crate::pass::openness::Openness;
use crate::pass::popularity::Popularity;
use crate::pass::AnyEvaluationPass;
use database::storage::Context;
use entity::metadata;
use futures::future::join_all;
use futures::TryStreamExt;
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub struct EvaluationManager {
    passes: Vec<Arc<dyn AnyEvaluationPass>>,
}

impl EvaluationManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    // 针对每种 Context 实现默认 Pass 的添加
    pub fn add_default_passes(&mut self) {
        self.add_pass(Arc::new(Popularity));
        self.add_pass(Arc::new(Activity));
        self.add_pass(Arc::new(Maintainability));
        self.add_pass(Arc::new(Maturity));
        self.add_pass(Arc::new(Openness));
    }

    pub fn add_pass(&mut self, pass: Arc<dyn AnyEvaluationPass>) {
        self.passes.push(pass);
    }

    pub async fn run(
        &self,
        evaluation_context: &EvaluationContext,
        db_context: &Context,
    ) -> anyhow::Result<()> {
        let stg = db_context.evaluate_database_stg();
        let url_stream = stg.get_metadata_stream().await?;

        url_stream
            .try_for_each_concurrent(8, |model| {
                let stg = stg.clone();
                let evaluation_context = evaluation_context.clone();
                let passes = self.passes.clone();
                async move {
                    // 并发执行所有 pass
                    let pass_futures = passes.iter().map(|pass| {
                        let data = pass.required_data(&model);
                        let ctx = evaluation_context.clone();
                        let name = pass.name();
                        async move {
                            let score = pass.apply(&ctx, data.as_ref());
                            (name, score)
                        }
                    });

                    let scores: Vec<(&str, f64)> = join_all(pass_futures).await;
                    let score_map: HashMap<&str, f64> = scores.into_iter().collect();
                    let evaluated_score: f64 = score_map.values().sum();

                    // 更新所有分数
                    let active_model = metadata::ActiveModel {
                        id: Set(model.id),
                        evaluated_score: Set(evaluated_score),
                        popularity_score: Set(*score_map.get("popularity_score").unwrap_or(&0.0)),
                        activity_score: Set(*score_map.get("activity_score").unwrap_or(&0.0)),
                        maintainability_score: Set(*score_map.get("maintainability_score").unwrap_or(&0.0)),
                        ..Default::default()
                    };
                    active_model.update(stg.get_connection()).await?;

                    info!(
                        "Repository {} evaluation completed - Scores: total= {}, popularity= {}, activity= {}, maintainability= {}",
                        model.repo_id,
                        evaluated_score,
                        score_map.get("popularity_score").unwrap_or(&0.0),
                        score_map.get("activity_score").unwrap_or(&0.0),
                        score_map.get("maintainability_score").unwrap_or(&0.0),
                    );
                    Ok(())
                }
            })
            .await?;

        Ok(())
    }
}
