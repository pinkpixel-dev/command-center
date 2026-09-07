//! Turning document structure into titles, tags, and a collection suggestion.
//! All of this is a starting point the user edits in the review screen.

/// Words that make terrible tags.
const STOPWORDS: [&str; 34] = [
    "the", "and", "for", "with", "that", "this", "from", "your", "you", "are", "was", "how", "why",
    "what", "when", "into", "all", "any", "can", "get", "use", "using", "not", "but", "its", "it",
    "a", "an", "of", "to", "in", "on", "at", "by",
];

/// Best available title for a block.
///
/// A heading that covers exactly one block names that block. Otherwise a short
/// lead-in sentence works, and failing both, the command speaks for itself.
pub fn title_for(
    heading: Option<&str>,
    heading_is_exclusive: bool,
    context: Option<&str>,
    content: &str,
) -> String {
    if let Some(text) = heading {
        if heading_is_exclusive && !text.trim().is_empty() {
            return truncate(text.trim(), 72);
        }
    }

    if let Some(text) = context {
        let sentence = first_sentence(text);
        if sentence.len() <= 64 && sentence.split_whitespace().count() >= 2 {
            return truncate(&sentence, 72);
        }
    }

    from_content(content)
}

/// Falls back to the first meaningful line of the command itself.
pub fn from_content(content: &str) -> String {
    let candidate = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or("Untitled command");

    truncate(candidate, 72)
}

/// Description text, capped so a whole paragraph does not land in the field.
pub fn description_for(context: Option<&str>, title: &str) -> String {
    let Some(text) = context else {
        return String::new();
    };
    let sentence = first_sentence(text);
    if sentence.eq_ignore_ascii_case(title) || sentence.len() < 8 {
        return String::new();
    }
    truncate(&sentence, 240)
}

/// Tags suggested for a block.
///
/// The tool being run is the most useful tag there is, so it comes first. Short
/// headings read like topics ("Docker", "Git Mistakes") and make good tags too;
/// sentence-style headings ("Check what failed at boot") do not, and are
/// skipped rather than shredded into filler words.
pub fn tags_for(heading_path: &[String], shell: Option<&str>, content: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();

    if let Some(tool) = primary_tool(content) {
        tags.push(tool);
    }

    for heading in heading_path.iter().rev().take(2) {
        if heading.split_whitespace().count() > 3 {
            continue;
        }
        for word in heading.split(|c: char| !c.is_alphanumeric()) {
            if tags.len() >= 4 {
                break;
            }
            let word = word.trim().to_lowercase();
            if word.len() < 3 || word.len() > 20 {
                continue;
            }
            if STOPWORDS.contains(&word.as_str()) || word.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            if !tags.contains(&word) {
                tags.push(word);
            }
        }
    }

    if let Some(shell) = shell {
        let shell = shell.to_lowercase();
        if !tags.contains(&shell) && tags.len() < 4 {
            tags.push(shell);
        }
    }

    tags
}

/// The program being run, which is what someone would search for later.
fn primary_tool(content: &str) -> Option<String> {
    let line = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('['))?;

    let mut words = line.split_whitespace();
    let mut first = words.next()?;

    // `sudo` and `env` are not the interesting part.
    if matches!(first, "sudo" | "doas" | "env" | "time") {
        first = words.next()?;
    }

    // Strip a path, and anything after a redirect or pipe.
    let name = first
        .rsplit('/')
        .next()
        .unwrap_or(first)
        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');

    let usable = name.len() >= 2
        && name.len() <= 20
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());

    if !usable {
        return None;
    }

    let lowered = name.to_lowercase();
    if STOPWORDS.contains(&lowered.as_str()) {
        return None;
    }
    Some(lowered)
}

/// Collection name suggested from the document title or file name.
pub fn collection_for(document_title: Option<&str>, source_name: Option<&str>) -> Option<String> {
    if let Some(title) = document_title {
        let cleaned = title.trim();
        if !cleaned.is_empty() && cleaned.len() <= 60 {
            return Some(cleaned.to_string());
        }
    }

    let name = source_name?;
    let stem = name.rsplit('/').next().unwrap_or(name);
    let stem = stem.split('.').next().unwrap_or(stem);
    if stem.is_empty() {
        return None;
    }

    let pretty: String = stem
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");

    if pretty.is_empty() {
        None
    } else {
        Some(pretty)
    }
}

fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    let end = trimmed
        .find(". ")
        .map(|index| index + 1)
        .unwrap_or(trimmed.len());
    trimmed[..end].trim_end_matches('.').trim().to_string()
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut short: String = text.chars().take(limit).collect();
    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exclusive_heading_names_the_block() {
        assert_eq!(
            title_for(Some("Update Arch packages"), true, None, "sudo pacman -Syu"),
            "Update Arch packages"
        );
    }

    #[test]
    fn a_shared_heading_does_not_name_every_block() {
        assert_eq!(
            title_for(Some("Docker"), false, None, "docker ps"),
            "docker ps"
        );
    }

    #[test]
    fn a_short_lead_in_becomes_the_title() {
        assert_eq!(
            title_for(Some("Docker"), false, Some("Remove stopped containers"), "docker container prune"),
            "Remove stopped containers"
        );
    }

    #[test]
    fn a_long_paragraph_does_not_become_a_title() {
        let paragraph = "This command walks the entire filesystem looking for files that have not been touched in a while and then removes them without asking";
        assert_eq!(
            title_for(None, false, Some(paragraph), "find . -delete"),
            "find . -delete"
        );
    }

    #[test]
    fn descriptions_skip_text_already_used_as_the_title() {
        assert_eq!(description_for(Some("Remove stopped containers"), "Remove stopped containers"), "");
        assert_eq!(
            description_for(Some("Frees the disk space dead containers hold"), "Prune"),
            "Frees the disk space dead containers hold"
        );
    }

    #[test]
    fn the_tool_being_run_is_the_first_tag() {
        let path = vec!["Docker Cheat Sheet".to_string(), "Cleanup".to_string()];
        let tags = tags_for(&path, Some("bash"), "docker container prune");

        assert_eq!(tags.first(), Some(&"docker".to_string()));
        assert!(tags.contains(&"cleanup".to_string()));
        assert!(!tags.iter().any(|tag| tag == "the"));
    }

    #[test]
    fn sentence_headings_do_not_become_tags() {
        let path = vec!["Arch Rescue".to_string(), "Check what failed at boot".to_string()];
        let tags = tags_for(&path, None, "systemctl --failed");

        assert!(tags.contains(&"systemctl".to_string()));
        assert!(tags.contains(&"arch".to_string()));
        for junk in ["check", "what", "failed", "boot"] {
            assert!(!tags.contains(&junk.to_string()), "{junk} should not be a tag");
        }
    }

    #[test]
    fn sudo_and_paths_do_not_hide_the_tool() {
        assert_eq!(primary_tool("sudo pacman -Syu"), Some("pacman".into()));
        assert_eq!(primary_tool("/usr/bin/systemctl restart nginx"), Some("systemctl".into()));
        assert_eq!(primary_tool("#!/usr/bin/env bash\ngit status"), Some("git".into()));
        assert_eq!(primary_tool("[profile.release]\nlto = true"), Some("lto".into()));
    }

    #[test]
    fn tag_lists_stay_short() {
        let path = vec!["Docker".to_string(), "Cleanup".to_string()];
        let tags = tags_for(&path, Some("bash"), "docker system prune -a");
        assert!(tags.len() <= 4, "got {tags:?}");
    }

    #[test]
    fn collection_falls_back_to_the_file_name() {
        assert_eq!(
            collection_for(None, Some("/home/pink/docs/arch-rescue.md")),
            Some("Arch Rescue".to_string())
        );
        assert_eq!(
            collection_for(Some("Git Mistakes"), Some("notes.md")),
            Some("Git Mistakes".to_string())
        );
    }

    #[test]
    fn long_titles_are_truncated() {
        let long = "x".repeat(120);
        let title = from_content(&long);
        assert_eq!(title.chars().count(), 73);
    }
}
