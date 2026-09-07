//! Deciding what a parsed block actually is: a command, a script, a config
//! snippet, or the output somebody pasted along with it.

use std::sync::OnceLock;

use regex::Regex;

use crate::models::CommandKind;

/// Languages that mean "this is configuration or code", not a shell command.
const SNIPPET_LANGUAGES: [&str; 22] = [
    "toml", "json", "yaml", "yml", "ini", "conf", "cfg", "sql", "rust", "python", "py", "js",
    "javascript", "ts", "typescript", "go", "java", "c", "cpp", "xml", "html", "css",
];

/// Languages that mean "shell", including session formats.
const SHELL_LANGUAGES: [&str; 11] = [
    "bash", "sh", "zsh", "fish", "shell", "console", "shell-session", "shellsession", "powershell",
    "pwsh", "ps1",
];

/// Languages that are explicitly not executable.
const OUTPUT_LANGUAGES: [&str; 6] = ["text", "txt", "output", "plain", "log", "diff"];

/// Why a block was judged to be terminal output.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputReason {
    LanguageHint,
    TabularColumns,
    KnownHeader,
    FewCommandLines,
}

impl OutputReason {
    pub fn describe(&self) -> &'static str {
        match self {
            Self::LanguageHint => "The code fence is marked as plain output",
            Self::TabularColumns => "Lines are laid out in columns, like a table of results",
            Self::KnownHeader => "Starts with a familiar output header",
            Self::FewCommandLines => "Very few lines look like commands",
        }
    }
}

fn output_headers() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?i)^\s*(CONTAINER ID|IMAGE\s+COMMAND|REPOSITORY\s+TAG|PID\s+|USER\s+PID|NAME\s+READY|NAMESPACE\s+NAME|Filesystem\s+|total \d+|COMMAND\s+PID|PORT\s+STATE|Proto\s+Recv-Q)",
        )
        .expect("output header pattern should compile")
    })
}

fn permission_line() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN
        .get_or_init(|| Regex::new(r"^[bcdlps-][rwxsStT-]{9}[.+]?\s").expect("mode pattern"))
}

fn log_line() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?i)^\s*(\[\d|\d{4}-\d{2}-\d{2}[ T]\d{2}:|\d{2}:\d{2}:\d{2}\s|(info|warn|warning|error|debug|trace)\b[:\s])")
            .expect("log pattern")
    })
}

/// Rough judgement of whether one line reads like something you would type.
pub fn looks_like_command(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        // Comments live inside scripts, so they are not evidence either way.
        return true;
    }
    if permission_line().is_match(trimmed)
        || log_line().is_match(trimmed)
        || output_headers().is_match(trimmed)
    {
        return false;
    }

    // Shell syntax is a strong signal.
    if trimmed.contains(" | ")
        || trimmed.contains("&&")
        || trimmed.contains("||")
        || trimmed.ends_with('\\')
        || trimmed.starts_with("./")
        || trimmed.starts_with('/')
        || trimmed.starts_with("~/")
        || trimmed.starts_with("sudo ")
    {
        return true;
    }

    let mut words = trimmed.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };

    // An executable name: lowercase letters, digits, dashes, dots, underscores.
    let plausible_name = first.len() <= 32
        && first
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':'))
        && first.chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '.');

    if !plausible_name {
        return false;
    }

    // A bare word with no arguments is ambiguous; accept the short, common ones.
    let rest: Vec<&str> = words.collect();
    if rest.is_empty() {
        return first.len() <= 12 && !first.contains(':');
    }

    // Output tables put wide gaps between columns; commands rarely do.
    if column_gaps(trimmed) >= 2 {
        return false;
    }

    true
}

/// Counts runs of two or more spaces, which is how column layouts look.
fn column_gaps(line: &str) -> usize {
    let mut gaps = 0;
    let mut spaces = 0;
    for character in line.trim().chars() {
        if character == ' ' {
            spaces += 1;
        } else {
            if spaces >= 2 {
                gaps += 1;
            }
            spaces = 0;
        }
    }
    gaps
}

/// Decides whether a block is terminal output rather than something to save.
pub fn detect_output(content: &str, language: Option<&str>, from_prompt_session: bool) -> Option<OutputReason> {
    // A session was already split into its commands; what is left is a command.
    if from_prompt_session {
        return None;
    }

    if let Some(hint) = language {
        if OUTPUT_LANGUAGES.contains(&hint) {
            return Some(OutputReason::LanguageHint);
        }
        if SNIPPET_LANGUAGES.contains(&hint) || SHELL_LANGUAGES.contains(&hint) {
            // An explicit code language means the author meant it as code.
            return None;
        }
    }

    let lines: Vec<&str> = content.lines().filter(|line| !line.trim().is_empty()).collect();
    if lines.is_empty() {
        return None;
    }

    if output_headers().is_match(lines[0]) {
        return Some(OutputReason::KnownHeader);
    }

    if lines.len() == 1 {
        return if looks_like_command(lines[0]) {
            None
        } else if column_gaps(lines[0]) >= 2 {
            Some(OutputReason::TabularColumns)
        } else {
            None
        };
    }

    let tabular = lines.iter().filter(|line| column_gaps(line) >= 2).count();
    if tabular * 2 >= lines.len() {
        return Some(OutputReason::TabularColumns);
    }

    let commandish = lines.iter().filter(|line| looks_like_command(line)).count();
    if commandish * 2 < lines.len() {
        return Some(OutputReason::FewCommandLines);
    }

    None
}

/// Picks the entry kind for a block.
pub fn detect_kind(content: &str, language: Option<&str>) -> CommandKind {
    if content.trim_start().starts_with("#!") {
        return CommandKind::Script;
    }

    if let Some(hint) = language {
        if SNIPPET_LANGUAGES.contains(&hint) {
            return CommandKind::Snippet;
        }
    }

    let lines: Vec<&str> = content.lines().filter(|line| !line.trim().is_empty()).collect();

    // Shell control flow means it is a script, not a list of commands.
    let scripty = lines.iter().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("if ")
            || trimmed.starts_with("case ")
            || trimmed.starts_with("function ")
            || trimmed == "done"
            || trimmed == "fi"
            || trimmed == "esac"
    });
    if scripty {
        return CommandKind::Script;
    }

    // A key/value block with no command-looking lines is configuration.
    if lines.len() > 1 && lines.iter().all(|line| is_assignment(line)) {
        return CommandKind::Snippet;
    }

    let commands = lines.iter().filter(|line| looks_like_command(line)).count();
    if commands > 1 {
        CommandKind::Sequence
    } else {
        CommandKind::Command
    }
}

fn is_assignment(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return true; // TOML/INI section header
    }
    match trimmed.split_once('=') {
        Some((key, _)) => {
            !key.trim().is_empty()
                && key
                    .trim()
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
        }
        None => trimmed
            .split_once(':')
            .is_some_and(|(key, _)| !key.trim().is_empty() && !key.contains(' ')),
    }
}

/// Shell name for a block, when the fence said so.
pub fn detect_shell(language: Option<&str>, content: &str) -> Option<String> {
    if let Some(hint) = language {
        match hint {
            "bash" | "sh" | "zsh" | "fish" => return Some(hint.to_string()),
            "powershell" | "pwsh" | "ps1" => return Some("pwsh".to_string()),
            _ => {}
        }
    }

    let first = content.lines().next().unwrap_or("");
    if let Some(shebang) = first.strip_prefix("#!") {
        for shell in ["bash", "zsh", "fish", "sh"] {
            if shebang.contains(shell) {
                return Some(shell.to_string());
            }
        }
    }
    None
}

/// Language recorded on the entry, for snippets where it is meaningful.
pub fn detect_language(language: Option<&str>, kind: CommandKind) -> Option<String> {
    let hint = language?;
    if kind == CommandKind::Snippet && SNIPPET_LANGUAGES.contains(&hint) {
        Some(hint.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_ordinary_commands() {
        for line in [
            "git status",
            "sudo pacman -Syu",
            "docker ps -a",
            "find . -name \"*.log\" | xargs rm",
            "./scripts/deploy.sh",
            "npm run build",
        ] {
            assert!(looks_like_command(line), "{line}");
        }
    }

    #[test]
    fn rejects_output_lines() {
        for line in [
            "drwxr-xr-x  2 sizzlebop sizzlebop 4096 Jul 25 19:18 storage",
            "CONTAINER ID   IMAGE     COMMAND",
            "2026-07-25 19:18:04 INFO  starting up",
            "total 188",
        ] {
            assert!(!looks_like_command(line), "{line}");
        }
    }

    #[test]
    fn detects_docker_ps_output() {
        let block = "CONTAINER ID   IMAGE     COMMAND       STATUS\n9f2b1c   nginx   \"nginx -g\"   Up 2 hours";
        assert_eq!(detect_output(block, None, false), Some(OutputReason::KnownHeader));
    }

    #[test]
    fn detects_ls_output_by_columns() {
        let block = "-rw-r--r--  1 pink pink   220 Jul 25 10:00 README.md\n-rw-r--r--  1 pink pink  1024 Jul 25 10:00 notes.txt";
        assert!(detect_output(block, None, false).is_some());
    }

    #[test]
    fn respects_an_explicit_output_fence() {
        assert_eq!(
            detect_output("anything at all here", Some("text"), false),
            Some(OutputReason::LanguageHint)
        );
    }

    #[test]
    fn leaves_real_commands_alone() {
        assert_eq!(detect_output("docker ps", Some("bash"), false), None);
        assert_eq!(detect_output("npm test\nnpm run build", None, false), None);
        assert_eq!(detect_output("[profile.release]\nlto = true", Some("toml"), false), None);
    }

    #[test]
    fn a_split_session_is_never_output() {
        assert_eq!(detect_output("git status", None, true), None);
    }

    #[test]
    fn classifies_kinds() {
        assert_eq!(detect_kind("git status", None), CommandKind::Command);
        assert_eq!(detect_kind("npm test\nnpm run build", None), CommandKind::Sequence);
        assert_eq!(
            detect_kind("#!/usr/bin/env bash\necho hi", None),
            CommandKind::Script
        );
        assert_eq!(
            detect_kind("for f in *.txt; do\n  echo \"$f\"\ndone", None),
            CommandKind::Script
        );
        assert_eq!(
            detect_kind("[profile.release]\nlto = true", Some("toml")),
            CommandKind::Snippet
        );
        assert_eq!(
            detect_kind("host = localhost\nport = 5432", None),
            CommandKind::Snippet
        );
    }

    #[test]
    fn reads_shell_and_language_hints() {
        assert_eq!(detect_shell(Some("fish"), ""), Some("fish".into()));
        assert_eq!(detect_shell(Some("ps1"), ""), Some("pwsh".into()));
        assert_eq!(detect_shell(None, "#!/usr/bin/env zsh\n"), Some("zsh".into()));
        assert_eq!(detect_shell(None, "git status"), None);

        assert_eq!(detect_language(Some("toml"), CommandKind::Snippet), Some("toml".into()));
        assert_eq!(detect_language(Some("bash"), CommandKind::Command), None);
    }
}
