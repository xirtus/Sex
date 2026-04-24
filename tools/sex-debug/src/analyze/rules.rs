use crate::analyze::AnalysisResult;
use crate::domain::Domain;
use crate::model::TraceEvent;

pub fn detect_violation(i: usize, window: &[TraceEvent]) -> Option<AnalysisResult> {
    if window.len() < 2 {
        return None;
    }

    let prev = &window[0];
    let curr = &window[1];

    if prev.domain != curr.domain && prev.pkru == curr.pkru {
        return Some(AnalysisResult {
            root_cause: "Domain switch without PKRU update".to_string(),
            first_bad_index: i + 1,
            confidence: 0.95,
            function: None,
        });
    }

    if prev.domain == curr.domain && prev.pkru != curr.pkru {
        return Some(AnalysisResult {
            root_cause: "PKRU changed without domain transition".to_string(),
            first_bad_index: i + 1,
            confidence: 0.85,
            function: None,
        });
    }

    if matches!(curr.domain, Domain::Unknown(_)) {
        return Some(AnalysisResult {
            root_cause: "Unknown PKRU value observed".to_string(),
            first_bad_index: i + 1,
            confidence: 0.80,
            function: None,
        });
    }

    if curr.rip == 0 {
        return Some(AnalysisResult {
            root_cause: "Execution jumped to null (RIP=0)".to_string(),
            first_bad_index: i + 1,
            confidence: 0.99,
            function: None,
        });
    }

    if curr.domain == Domain::Kernel && curr.pkru != 0x0 {
        return Some(AnalysisResult {
            root_cause: "Kernel executing under restricted PKRU".to_string(),
            first_bad_index: i + 1,
            confidence: 0.97,
            function: None,
        });
    }

    None
}
