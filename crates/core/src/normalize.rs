//! Turning "the same command typed two different ways" into one comparable
//! string. Used for duplicate detection and for the content hash that will
//! later mark cached AI explanations as stale.

use sha2::{Digest, Sha256};

/// Strips shell prompt noise and collapses whitespace without touching the
/// meaningful parts of a command (quoting, indentation inside scripts).
pub fn normalize_command(content: &str) -> String {
    let unified = content.replace("\r\n", "\n").replace('\r', "\n");

    let mut lines: Vec<String> = Vec::new();
    let mut blank_run = false;

    for raw_line in unified.lines() {
        let line = strip_prompt_prefix(raw_line.trim_end());
        if line.trim().is_empty() {
            // Collapse runs of blank lines down to a single separator.
            if !blank_run && !lines.is_empty() {
                lines.push(String::new());
                blank_run = true;
            }
            continue;
        }
        blank_run = false;
        lines.push(line.to_string());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n").trim_end().to_string()
}

/// Removes a leading `$ `, `> ` or `PS> ` prompt, keeping indentation of script
/// bodies intact when no prompt is present.
///
/// A leading `# ` is deliberately left alone: telling a root prompt apart from
/// an ordinary shell comment is guesswork, and silently eating the `#` off a
/// comment is the worse mistake of the two.
fn strip_prompt_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();

    for prefix in ["PS> ", "PS>", "$ ", "> "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest;
        }
    }

    // Lines that are exactly "$" or ">" carry no content worth keeping.
    if trimmed == "$" || trimmed == ">" {
        return "";
    }

    line
}

/// Stable fingerprint of the normalized command, used for duplicate lookups.
pub fn content_hash(content: &str) -> String {
    let normalized = normalize_command(content);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Pulls `{{placeholder}}` names out of a command, in first-seen order.
pub fn extract_variables(content: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let bytes: Vec<char> = content.chars().collect();
    let mut index = 0;

    while index + 1 < bytes.len() {
        if bytes[index] == '{' && bytes[index + 1] == '{' {
            if let Some(close) = find_closing(&bytes, index + 2) {
                let name: String = bytes[index + 2..close].iter().collect();
                let name = name.trim().to_string();
                let valid = !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
                if valid && !found.contains(&name) {
                    found.push(name);
                }
                index = close + 2;
                continue;
            }
        }
        index += 1;
    }

    found
}

fn find_closing(chars: &[char], from: usize) -> Option<usize> {
    let mut index = from;
    while index + 1 < chars.len() {
        if chars[index] == '}' && chars[index + 1] == '}' {
            return Some(index);
        }
        if chars[index] == '\n' {
            return None;
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_dollar_prompt() {
        assert_eq!(
            normalize_command("$ git reset --soft HEAD~1"),
            "git reset --soft HEAD~1"
        );
    }

    #[test]
    fn strips_powershell_prompt() {
        assert_eq!(normalize_command("PS> Get-Process"), "Get-Process");
    }

    #[test]
    fn keeps_comments_intact() {
        assert_eq!(
            normalize_command("#!/usr/bin/env bash\n# explain things"),
            "#!/usr/bin/env bash\n# explain things"
        );
        assert_eq!(normalize_command("# pacman -Syu"), "# pacman -Syu");
    }

    #[test]
    fn drops_empty_prompt_lines() {
        assert_eq!(normalize_command("$\ngit status\n>"), "git status");
    }

    #[test]
    fn collapses_blank_line_runs_and_normalizes_line_endings() {
        let input = "npm test\r\n\r\n\r\n\r\nnpm run build\r\n\r\n";
        assert_eq!(normalize_command(input), "npm test\n\nnpm run build");
    }

    #[test]
    fn preserves_script_indentation_and_quoting() {
        let script = "for f in *.txt; do\n  echo \"$f  spaced\"\ndone";
        assert_eq!(normalize_command(script), script);
    }

    #[test]
    fn hash_ignores_prompt_and_trailing_noise() {
        assert_eq!(
            content_hash("$ docker ps  \n\n"),
            content_hash("docker ps")
        );
        assert_ne!(content_hash("docker ps"), content_hash("docker ps -a"));
    }

    #[test]
    fn extracts_variables_in_order_without_duplicates() {
        let vars = extract_variables("ssh {{user}}@{{host}} -p {{port}} # retry {{user}}");
        assert_eq!(vars, vec!["user", "host", "port"]);
    }

    #[test]
    fn ignores_malformed_placeholders() {
        assert!(extract_variables("echo {{ not closed").is_empty());
        assert!(extract_variables("awk '{{print $1}}'").is_empty());
    }
}
