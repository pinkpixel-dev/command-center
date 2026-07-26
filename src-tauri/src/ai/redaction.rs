//! Local detection and replacement of likely credentials before future
//! user-content workflows serialize an OpenAI request.

use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactionResult {
    pub redacted_text: String,
    pub findings: Vec<RedactionFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactionFinding {
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
    pub placeholder: &'static str,
}

struct Rule {
    kind: &'static str,
    pattern: Regex,
    secret_group: usize,
    placeholder: &'static str,
}

pub fn redact(input: &str) -> RedactionResult {
    let mut findings = rules()
        .iter()
        .flat_map(|rule| {
            rule.pattern.captures_iter(input).filter_map(|captures| {
                let matched = captures.get(rule.secret_group)?;
                Some(RedactionFinding {
                    kind: rule.kind,
                    start: matched.start(),
                    end: matched.end(),
                    placeholder: rule.placeholder,
                })
            })
        })
        .collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    findings = remove_overlaps(findings);

    let mut redacted_text = String::with_capacity(input.len());
    let mut cursor = 0;
    for finding in &findings {
        redacted_text.push_str(&input[cursor..finding.start]);
        redacted_text.push_str(finding.placeholder);
        cursor = finding.end;
    }
    redacted_text.push_str(&input[cursor..]);

    RedactionResult {
        redacted_text,
        findings,
    }
}

fn remove_overlaps(findings: Vec<RedactionFinding>) -> Vec<RedactionFinding> {
    let mut accepted = Vec::with_capacity(findings.len());
    let mut covered_until = 0;
    for finding in findings {
        if finding.start < covered_until {
            continue;
        }
        covered_until = finding.end;
        accepted.push(finding);
    }
    accepted
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            rule(
                "private_key",
                r"(?s)-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----.*?-----END (?:[A-Z0-9 ]+ )?PRIVATE KEY-----",
                0,
                "{{PRIVATE_KEY}}",
            ),
            rule(
                "embedded_credentials",
                r"(?i)[a-z][a-z0-9+.-]*://([^/\s:@]+:[^@\s/]+)@",
                1,
                "{{USERNAME}}:{{PASSWORD}}",
            ),
            rule(
                "openai_api_key",
                r"\b(sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{16,})\b",
                1,
                "{{OPENAI_API_KEY}}",
            ),
            rule(
                "github_token",
                r"\b((?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}))\b",
                1,
                "{{GITHUB_TOKEN}}",
            ),
            rule(
                "aws_access_key_id",
                r"\b((?:AKIA|ASIA)[A-Z0-9]{16})\b",
                1,
                "{{AWS_ACCESS_KEY_ID}}",
            ),
            rule(
                "bearer_token",
                r"(?i)\bbearer\s+([A-Za-z0-9._~+/=-]{16,})",
                1,
                "{{BEARER_TOKEN}}",
            ),
            rule(
                "password",
                r#"(?i)\b(?:password|passwd|pwd)\s*[=:]\s*["']?([^\s"';&|]{4,})"#,
                1,
                "{{PASSWORD}}",
            ),
            rule(
                "api_key",
                r#"(?i)\b(?:[A-Z0-9_]*api[_-]?key|access[_-]?key)\s*[=:]\s*["']?([^\s"';&|]{8,})"#,
                1,
                "{{API_KEY}}",
            ),
            rule(
                "token",
                r#"(?i)\b(?:[A-Z0-9_]*token|secret)\s*[=:]\s*["']?([^\s"';&|]{8,})"#,
                1,
                "{{TOKEN}}",
            ),
        ]
    })
}

fn rule(kind: &'static str, pattern: &str, secret_group: usize, placeholder: &'static str) -> Rule {
    Rule {
        kind,
        pattern: Regex::new(pattern).expect("valid secret-detection pattern"),
        secret_group,
        placeholder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_secret_shapes_are_replaced_without_retaining_values() {
        let openai = ["sk-", "abcdefghijklmnopqrstuvwxyz123456"].concat();
        let bearer = ["eyJhbGciOiJIUzI1NiJ9", ".payload.signature"].concat();
        let aws = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
        let github = ["github_pat_", "abcdefghijklmnopqrstuvwxyz123456"].concat();
        let source = format!(
            concat!(
                "OPENAI_API_KEY={}\n",
                "Authorization: Bearer {}\n",
                "password=hunter2\n",
                "remote=https://alice:correct-horse@example.com/database\n",
                "aws={}\n",
                "token={}\n",
                "-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----\n",
            ),
            openai, bearer, aws, github
        );

        let result = redact(&source);
        let serialized = serde_json::to_string(&result).unwrap();

        for raw in [
            openai.as_str(),
            bearer.as_str(),
            "hunter2",
            "alice:correct-horse",
            aws.as_str(),
            github.as_str(),
            "abc123",
        ] {
            assert!(!result.redacted_text.contains(raw));
            assert!(!serialized.contains(raw));
        }
        assert!(result
            .redacted_text
            .contains("OPENAI_API_KEY={{OPENAI_API_KEY}}"));
        assert!(result
            .redacted_text
            .contains("https://{{USERNAME}}:{{PASSWORD}}@example.com"));
        assert_eq!(result.findings.len(), 7);
    }

    #[test]
    fn findings_report_locations_and_placeholders_but_not_secret_values() {
        let source = "before password=do-not-keep after";
        let result = redact(source);
        let finding = &result.findings[0];

        assert_eq!(&source[finding.start..finding.end], "do-not-keep");
        assert_eq!(finding.kind, "password");
        assert_eq!(finding.placeholder, "{{PASSWORD}}");
        assert_eq!(result.redacted_text, "before password={{PASSWORD}} after");
    }

    #[test]
    fn normal_commands_are_unchanged() {
        let source = "git status\ncurl https://example.com/api\nexport MODE=production";
        let result = redact(source);

        assert_eq!(result.redacted_text, source);
        assert!(result.findings.is_empty());
    }
}
