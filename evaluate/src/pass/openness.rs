use crate::config::EvaluationContext;
use crate::pass::{AnyEvaluationPass, PassData};
use entity::metadata::Model as MetadataModel;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct OpennessData {
    pub has_license: bool,
}

impl From<&MetadataModel> for OpennessData {
    fn from(model: &MetadataModel) -> Self {
        Self {
            has_license: model
                .license_name
                .as_ref()
                .map_or(false, |name| !name.is_empty()),
        }
    }
}

pub struct Openness;

impl AnyEvaluationPass for Openness {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64 {
        let openness_data = (data as &dyn Any).downcast_ref::<OpennessData>().unwrap();
        ctx.pass.openness_weight
            * (if openness_data.has_license {
                ctx.openness.license_info
            } else {
                0.0
            })
    }

    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData> {
        Box::new(OpennessData::from(model))
    }

    fn name(&self) -> &'static str {
        "openness_score"
    }
}
