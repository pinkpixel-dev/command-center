//! Local, offline risk rules. These always run and always win over anything an
//! AI layer suggests later, because obvious destruction should never depend on
//! a network call.

use std::sync::OnceLock;

use regex::Regex;

use crate::models::RiskLevel;

struct Rule {
    pattern: Regex,
    level: RiskLevel,
    reason: &'static str,
}

fn rules() -> &'static Vec<Rule> {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(build_rules)
}

fn rule(pattern: &str, level: RiskLevel, reason: &'static str) -> Rule {
    Rule {
        pattern: Regex::new(pattern).expect("risk rule pattern should compile"),
        level,
        reason,
    }
}

fn build_rules() -> Vec<Rule> {
    use RiskLevel::{Caution, Destructive};

    vec![
        // Destructive: data loss, unrecoverable writes, remote code execution.
        rule(
            r"(?i)\brm\s+(-[a-z]*\s+)*-[a-z]*r[a-z]*f|(?i)\brm\s+(-[a-z]*\s+)*-[a-z]*f[a-z]*r",
            Destructive,
            "Recursive force delete, no recycle bin and no confirmation",
        ),
        rule(r"(?i)\bmkfs(\.[a-z0-9]+)?\b", Destructive, "Formats a filesystem"),
        rule(r"(?i)\bdd\s+.*\bof=", Destructive, "Writes raw blocks straight to a device or file"),
        rule(r"(?i)\bshred\b", Destructive, "Overwrites files so they cannot be recovered"),
        rule(r"(?i)\bgit\s+reset\s+--hard\b", Destructive, "Throws away uncommitted work"),
        rule(
            r"(?i)\bgit\s+push\s+.*(--force\b|(^|\s)-f(\s|$))",
            Destructive,
            "Force push rewrites history other people may have pulled",
        ),
        rule(r"(?i)\bgit\s+clean\s+-[a-z]*[dfx]", Destructive, "Deletes untracked files"),
        rule(
            r"(?i)\b(drop\s+(table|database|schema)|truncate\s+table)\b",
            Destructive,
            "Removes database objects or rows permanently",
        ),
        rule(
            r"(?i)\bfind\b[^|]*\s-delete\b",
            Destructive,
            "Deletes every file the search matches",
        ),
        rule(
            r"(?i)\bcurl\b[^|]*\|\s*(sudo\s+)?(ba|z|k)?sh\b|(?i)\bwget\b[^|]*\|\s*(sudo\s+)?(ba|z|k)?sh\b",
            Destructive,
            "Downloads a remote script and executes it immediately",
        ),
        rule(r":\(\)\s*\{\s*:\|:&\s*\}\s*;\s*:", Destructive, "Fork bomb"),
        rule(r">\s*/dev/(sd|nvme|hd)", Destructive, "Writes directly to a disk device"),
        // Caution: elevated, wide-reaching, or history-rewriting but recoverable.
        rule(r"(?m)^\s*sudo\b|\s\|\s*sudo\b|&&\s*sudo\b", Caution, "Runs with elevated privileges"),
        rule(r"(?i)\bchmod\b", Caution, "Changes file permissions"),
        rule(r"(?i)\bchown\b", Caution, "Changes file ownership"),
        rule(r"(?i)\b(rm|rmdir)\b", Caution, "Deletes files or directories"),
        rule(r"(?i)\b(kill|pkill|killall)\s+-9\b", Caution, "Force kills processes without cleanup"),
        rule(
            r"(?i)\b(pacman\s+-S|apt(-get)?\s+(install|remove|purge)|dnf\s+(install|remove)|yay\s+-S|brew\s+(install|uninstall))\b",
            Caution,
            "Installs or removes system packages",
        ),
        rule(
            r"(?i)\b(npm|pnpm|yarn|bun)\s+(install|add|remove)\s+.*(-g|--global)\b",
            Caution,
            "Changes globally installed packages",
        ),
        rule(
            r"(?i)\bdocker\s+(system\s+prune|image\s+prune|volume\s+prune|container\s+prune|rmi|rm)\b",
            Caution,
            "Removes Docker resources",
        ),
        rule(r"(?i)\bsystemctl\s+(stop|disable|mask|restart)\b", Caution, "Changes system services"),
        rule(r"(?i)\biptables\b|(?i)\bufw\s+(deny|delete|disable)\b", Caution, "Changes firewall rules"),
        rule(
            r"(?i)\bgit\s+(rebase|commit\s+--amend|filter-branch)\b",
            Caution,
            "Rewrites local git history",
        ),
        rule(r"(?i)\bmv\b.*\s-f\b", Caution, "Overwrites the destination without asking"),
        // Truncating redirect (`cmd > file`). Append (`>>`), fd redirects
        // (`2>&1`) and arrows (`->`, `=>`) are deliberately excluded.
        rule(r"(?:^|\s)>\s*[^\s>&|]", Caution, "Redirect overwrites the target file"),
    ]
}

/// Classifies a command and explains why. The highest matching level wins and
/// every matching reason is reported so the UI can show more than one warning.
pub fn assess(content: &str) -> (RiskLevel, Vec<String>) {
    let mut level = RiskLevel::Safe;
    let mut reasons: Vec<String> = Vec::new();

    for rule in rules() {
        if rule.pattern.is_match(content) {
            if rule.level > level {
                level = rule.level;
            }
            let reason = rule.reason.to_string();
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
        }
    }

    // A destructive verdict makes the milder notes noise; keep the sharp ones.
    if level == RiskLevel::Destructive {
        let destructive: Vec<String> = rules()
            .iter()
            .filter(|rule| rule.level == RiskLevel::Destructive && rule.pattern.is_match(content))
            .map(|rule| rule.reason.to_string())
            .collect();
        return (level, destructive);
    }

    (level, reasons)
}

/// Re-runs the rules for a stored command whose reasons were not persisted.
pub fn reasons_for(content: &str, stored: RiskLevel) -> Vec<String> {
    let (detected, reasons) = assess(content);
    if detected == stored {
        reasons
    } else {
        // The user overrode the label, so only surface notes at or below theirs.
        rules()
            .iter()
            .filter(|rule| rule.level <= stored && rule.pattern.is_match(content))
            .map(|rule| rule.reason.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level_of(command: &str) -> RiskLevel {
        assess(command).0
    }

    #[test]
    fn plain_read_only_commands_are_safe() {
        for command in ["ls", "pwd", "git status", "docker ps", "cat README.md"] {
            assert_eq!(level_of(command), RiskLevel::Safe, "{command}");
        }
    }

    #[test]
    fn recursive_force_delete_is_destructive() {
        for command in ["rm -rf /tmp/build", "rm -fr ./dist", "sudo rm -rf node_modules"] {
            assert_eq!(level_of(command), RiskLevel::Destructive, "{command}");
        }
    }

    #[test]
    fn disk_and_database_destruction_is_flagged() {
        assert_eq!(level_of("mkfs.ext4 /dev/sdb1"), RiskLevel::Destructive);
        assert_eq!(
            level_of("dd if=image.iso of=/dev/sdb bs=4M"),
            RiskLevel::Destructive
        );
        assert_eq!(level_of("DROP TABLE users;"), RiskLevel::Destructive);
        assert_eq!(level_of("TRUNCATE TABLE sessions;"), RiskLevel::Destructive);
    }

    #[test]
    fn history_rewriting_git_is_split_by_severity() {
        assert_eq!(level_of("git reset --hard HEAD~1"), RiskLevel::Destructive);
        assert_eq!(level_of("git push --force origin main"), RiskLevel::Destructive);
        assert_eq!(level_of("git rebase -i HEAD~3"), RiskLevel::Caution);
        assert_eq!(level_of("git reset --soft HEAD~1"), RiskLevel::Safe);
    }

    #[test]
    fn piping_a_download_into_a_shell_is_destructive() {
        assert_eq!(
            level_of("curl -fsSL https://example.com/install.sh | sh"),
            RiskLevel::Destructive
        );
        assert_eq!(
            level_of("wget -qO- https://example.com/i.sh | sudo bash"),
            RiskLevel::Destructive
        );
    }

    #[test]
    fn everyday_admin_work_is_caution() {
        assert_eq!(level_of("sudo pacman -Syu"), RiskLevel::Caution);
        assert_eq!(level_of("chmod +x script.sh"), RiskLevel::Caution);
        assert_eq!(level_of("docker image prune"), RiskLevel::Caution);
        assert_eq!(level_of("npm install -g typescript"), RiskLevel::Caution);
    }

    #[test]
    fn find_delete_is_destructive_but_find_print_is_not() {
        assert_eq!(
            level_of("find . -type f -name \"*.log\" -mtime +7 -delete"),
            RiskLevel::Destructive
        );
        assert_eq!(
            level_of("find . -type f -name \"*.log\" -mtime +7 -print"),
            RiskLevel::Safe
        );
    }

    #[test]
    fn destructive_verdicts_report_only_destructive_reasons() {
        let (level, reasons) = assess("sudo rm -rf /var/cache/pacman/pkg");
        assert_eq!(level, RiskLevel::Destructive);
        assert!(!reasons.is_empty());
        assert!(reasons.iter().all(|reason| !reason.contains("elevated")));
    }

    #[test]
    fn reasons_respect_a_user_override() {
        let overridden = reasons_for("sudo pacman -Syu", RiskLevel::Safe);
        assert!(overridden.is_empty());
        let honest = reasons_for("sudo pacman -Syu", RiskLevel::Caution);
        assert!(!honest.is_empty());
    }
}
