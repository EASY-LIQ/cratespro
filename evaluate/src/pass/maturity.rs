use crate::config::EvaluationContext;
use crate::pass::{AnyEvaluationPass, PassData};
use entity::metadata::Model as MetadataModel;
use serde_json::Value;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct MaturityData {
    pub rust_ratio: f64,
    pub releases_count: bool,
}
//#region 1
impl MaturityData {
    fn calculate_rust_ratio(languages_json: &Option<String>, total_size: i32) -> f64 {
        if total_size <= 0 {
            return 0.0;
        }

        if let Some(json_str) = languages_json {
            if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                let rust_size: i64 = json
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .find(|item| item["node"]["name"].as_str() == Some("Rust"))
                            .and_then(|item| item["size"].as_i64())
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                return rust_size as f64 / total_size as f64;
            }
        }
        0.0
    }
}
//#endregion

impl From<&MetadataModel> for MaturityData {
    fn from(model: &MetadataModel) -> Self {
        Self {
            rust_ratio: Self::calculate_rust_ratio(
                &model.languages_json,
                model.language_total_size,
            ),
            releases_count: model.release_count > 0,
        }
    }
}

pub struct Maturity;

impl AnyEvaluationPass for Maturity {
    fn apply(&self, ctx: &EvaluationContext, data: &dyn PassData) -> f64 {
        let maturity_data = (data as &dyn Any).downcast_ref::<MaturityData>().unwrap();
        ctx.pass.maturity_weight
            * (maturity_data.rust_ratio * ctx.maturity.languages
                + if maturity_data.releases_count {
                    ctx.maturity.push_releases
                } else {
                    0.0
                })
    }

    fn required_data(&self, model: &MetadataModel) -> Box<dyn PassData> {
        Box::new(MaturityData::from(model))
    }

    fn name(&self) -> &'static str {
        "maturity_score"
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_valid_json_with_rust() {
        let json = r#"
        [
            {"node": {"name": "Rust"}, "size": 1000},
            {"node": {"name": "Python"}, "size": 2000}
        ]
        "#
        .to_string();
        let result = MaturityData::calculate_rust_ratio(&Some(json), 3000);
        println!("result: {}", result);
        assert_eq!(result, 1000.0 / 3000.0);
    }

    #[test]
    fn test_valid_json_without_rust() {
        let json = r#"
        [
            {"node": {"name": "Python"}, "size": 2000},
            {"node": {"name": "Java"}, "size": 1000}
        ]"#
        .to_string();
        let result = MaturityData::calculate_rust_ratio(&Some(json), 3000);
        println!("result: {}", result);
        assert_eq!(result, 0.0);
    }

    
}