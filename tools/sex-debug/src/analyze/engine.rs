use crate::analyze::{rules, AnalysisResult};
use crate::model::TraceEvent;

pub fn analyze(events: &[TraceEvent]) -> AnalysisResult {
    if events.is_empty() {
        return AnalysisResult {
            root_cause: "Empty trace".to_string(),
            first_bad_index: 0,
            confidence: 0.0,
            function: None,
        };
    }

    for (i, window) in events.windows(2).enumerate() {
        if let Some(result) = rules::detect_violation(i, window) {
            return result;
        }
    }

    AnalysisResult {
        root_cause: "No violations found".to_string(),
        first_bad_index: events.len().saturating_sub(1),
        confidence: 1.0,
        function: None,
    }
}
