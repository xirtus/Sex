use serde::Serialize;

pub mod engine;
pub mod report;
pub mod rules;

#[derive(Serialize)]
pub struct AnalysisResult {
    pub root_cause: String,
    pub first_bad_index: usize,
    pub confidence: f32,
    pub function: Option<String>,
}
