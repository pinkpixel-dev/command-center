//! What the user is told before their own text leaves the machine.
//!
//! Import and error analysis both take a block of content the user did not
//! write for a model to read, so both show the same thing first: how much text
//! goes out, which model receives it, and every likely secret that was replaced
//! along the way. Redaction happens here once, and the redacted text this
//! returns is the text that is actually sent. The disclosure and the request
//! can never drift apart, because they come from the same call.

use serde::Serialize;

use crate::ai::redaction::{self, RedactionFinding};

/// The summary shown before a request, paired with the text that would be sent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutboundPlan {
    /// Size of the content as it sits on this machine.
    pub document_bytes: usize,
    /// Size of the redacted text that would actually be sent.
    pub sent_bytes: usize,
    pub line_count: usize,
    pub model: String,
    pub findings: Vec<PlannedRedaction>,
}

/// A likely secret, described by where it is rather than by what it says.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedRedaction {
    pub kind: &'static str,
    pub placeholder: &'static str,
    pub line: usize,
}

/// Redacts once and describes the result. Callers send the returned text and
/// show the returned plan, so what the user sees and what leaves the machine
/// are produced by the same code.
pub fn plan(content: &str, model: &str) -> (String, OutboundPlan) {
    let result = redaction::redact(content);
    let starts = line_starts(content);

    let findings = result
        .findings
        .iter()
        .map(|finding| PlannedRedaction {
            kind: finding.kind,
            placeholder: finding.placeholder,
            line: line_of(&starts, finding),
        })
        .collect();

    let plan = OutboundPlan {
        document_bytes: content.len(),
        sent_bytes: result.redacted_text.len(),
        line_count: content.lines().count(),
        model: model.to_owned(),
        findings,
    };
    (result.redacted_text, plan)
}

fn line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        content
            .match_indices('\n')
            .map(|(index, _)| index.saturating_add(1)),
    );
    starts
}

fn line_of(starts: &[usize], finding: &RedactionFinding) -> usize {
    match starts.binary_search(&finding.start) {
        Ok(index) => index + 1,
        Err(index) => index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_reports_locations_and_never_the_secret_itself() {
        let secret = ["sk-", "abcdefghijklmnopqrstuvwxyz012345"].concat();
        let document =
            format!("# Notes\n\ngit status\nexport OPENAI_API_KEY={secret}\npassword=hunter2\n");

        let (redacted, plan) = plan(&document, "gpt-test");
        let serialized = serde_json::to_string(&plan).unwrap();

        assert_eq!(plan.document_bytes, document.len());
        assert_eq!(plan.sent_bytes, redacted.len());
        assert_eq!(plan.line_count, 5);
        assert_eq!(plan.model, "gpt-test");
        assert_eq!(
            plan.findings,
            vec![
                PlannedRedaction {
                    kind: "openai_api_key",
                    placeholder: "{{OPENAI_API_KEY}}",
                    line: 4,
                },
                PlannedRedaction {
                    kind: "password",
                    placeholder: "{{PASSWORD}}",
                    line: 5,
                },
            ]
        );
        for raw in [secret.as_str(), "hunter2"] {
            assert!(!serialized.contains(raw));
            assert!(!redacted.contains(raw));
        }
    }

    #[test]
    fn a_clean_document_reports_nothing_to_review() {
        let (redacted, plan) = plan("git status\ndocker ps\n", "gpt-test");

        assert!(plan.findings.is_empty());
        assert_eq!(redacted, "git status\ndocker ps\n");
        assert_eq!(plan.sent_bytes, plan.document_bytes);
    }

    #[test]
    fn line_numbers_hold_up_on_the_first_and_last_line() {
        let document = "password=first\ngit status\ntoken=last-value-here";
        let (_, plan) = plan(document, "gpt-test");

        assert_eq!(plan.findings[0].line, 1);
        assert_eq!(plan.findings[1].line, 3);
    }

    /// Terminal output is the other thing this describes, and a stack trace
    /// with a connection string in it is exactly the case worth proving.
    #[test]
    fn pasted_terminal_output_is_described_the_same_way_a_document_is() {
        let output = concat!(
            "Error: connection refused\n",
            "  at connect (postgres://admin:letmein@db.internal:5432/app)\n",
            "  exit status 1\n",
        );

        let (redacted, plan) = plan(output, "gpt-test");

        assert_eq!(plan.line_count, 3);
        assert_eq!(plan.findings.len(), 1);
        assert_eq!(plan.findings[0].line, 2);
        assert!(!redacted.contains("letmein"));
        assert!(redacted.contains("{{USERNAME}}:{{PASSWORD}}"));
    }
}
