use crate::analyze::AnalysisResult;

pub struct FixSuggestion {
    pub message: String,
}

pub fn suggest_fix(result: &AnalysisResult) -> FixSuggestion {
    let message = match result.root_cause.as_str() {
        "Domain switch without PKRU update" => {
            "Insert wrpkru(<linen_pkru>) before entering linen domain"
        }
        "PKRU changed without domain transition" => {
            "Remove or guard stray wrpkru call"
        }
        "Unknown PKRU value observed" => {
            "Verify PKRU mask and ensure correct pkey bit layout"
        }
        "Execution jumped to null (RIP=0)" => {
            "Check function pointer initialization or ELF mapping"
        }
        "Kernel executing under restricted PKRU" => {
            "Restore PKRU to 0x0 before returning to kernel context"
        }
        _ => "Further investigation required",
    };

    FixSuggestion {
        message: message.to_string(),
    }
}
