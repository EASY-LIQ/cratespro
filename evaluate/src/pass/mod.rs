pub mod activity;
pub mod maintainability;
pub mod maturity;
pub mod openness;
pub mod popularity;

use crate::config::EvaluationContext;
use entity::metadata::Model as MetadataModel;
use std::any::Any;

#[derive(Debug, Clone, Copy)]
pub enum EvaluationGrade {
    A, // 90-100
    B, // 80-89
    C, // 60-79
    D, // 0-59
}

/// 将分数转换为等级
#[allow(dead_code)]
pub fn score_to_grade(score: f64) -> EvaluationGrade {
    match score {
        s if s >= 90.0 => EvaluationGrade::A,
        s if s >= 80.0 => EvaluationGrade::B,
        s if s >= 60.0 => EvaluationGrade::C,
        _ => EvaluationGrade::D,
    }
}

pub trait PassData: Any + Send + Sync {}

impl<T: Any + Send + Sync> PassData for T {}

pub trait AnyEvaluationPass: Send + Sync {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64;
    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData>;
    fn name(&self) -> &'static str;
}
