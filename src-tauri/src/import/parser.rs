//! Line-based scanner that pulls code blocks out of Markdown, README files,
//! and plain notes. Runs entirely offline; AI never sees the document.

/// A block of code found in the source, with whatever context surrounded it.
#[derive(Debug, Clone, PartialEq)]
pub struct RawBlock {
    pub content: String,
    /// Info string from a fence, when there was one.
    pub language: Option<String>,
    /// Headings above this block, outermost first.
    pub heading_path: Vec<String>,
    /// The paragraph immediately before the block, if it read like prose.
    pub context: Option<String>,
    /// 1-based line the block starts on, for showing the user where it came from.
    pub line: usize,
    /// True when the block was a shell session and only the prompt lines were kept.
    pub from_prompt_session: bool,
    /// How many lines were discarded as terminal output during that extraction.
    pub dropped_output_lines: usize,
}

/// Everything the parser learned about a document.
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    pub blocks: Vec<RawBlock>,
    /// First level-one heading, used to suggest a collection name.
    pub title: Option<String>,
}

const PROMPTS: [&str; 3] = ["$ ", "PS> ", "PS>"];

pub fn parse(source: &str) -> ParsedDocument {
    let mut document = ParsedDocument::default();
    // Borrowed from this local; every value the parser returns is owned.
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();

    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();

        // Fenced block ------------------------------------------------------
        if let Some(fence) = fence_marker(trimmed) {
            let info = trimmed[fence.len()..].trim().to_string();
            let start = index + 1;
            let mut body: Vec<&str> = Vec::new();
            index += 1;

            while index < lines.len() {
                let candidate = lines[index].trim_start();
                if candidate.starts_with(&fence) && candidate.trim_end().chars().all(|c| c == fence.chars().next().unwrap_or('`')) {
                    break;
                }
                body.push(lines[index]);
                index += 1;
            }
            index += 1; // step over the closing fence

            push_block(
                &mut document,
                body.join("\n"),
                language_from_info(&info),
                &headings,
                take_paragraph(&mut paragraph),
                start,
            );
            continue;
        }

        // Heading -----------------------------------------------------------
        if let Some((level, text)) = heading(trimmed) {
            headings.retain(|(existing, _)| *existing < level);
            headings.push((level, text.clone()));
            if level == 1 && document.title.is_none() {
                document.title = Some(text);
            }
            paragraph.clear();
            index += 1;
            continue;
        }

        // Setext heading (underlined with === or ---) ------------------------
        if !trimmed.is_empty() && index + 1 < lines.len() {
            let next = lines[index + 1].trim();
            let underline = next.len() >= 3
                && (next.chars().all(|c| c == '=') || next.chars().all(|c| c == '-'));
            if underline && !paragraph.is_empty() {
                let level = if next.starts_with('=') { 1 } else { 2 };
                let text = clean_heading(trimmed);
                headings.retain(|(existing, _)| *existing < level);
                headings.push((level, text.clone()));
                if level == 1 && document.title.is_none() {
                    document.title = Some(text);
                }
                paragraph.clear();
                index += 2;
                continue;
            }
        }

        // Prompt run --------------------------------------------------------
        if starts_with_prompt(trimmed) {
            let start = index + 1;
            let mut body: Vec<&str> = Vec::new();
            while index < lines.len() {
                let candidate = lines[index].trim_start();
                if starts_with_prompt(candidate) {
                    body.push(lines[index]);
                    index += 1;
                } else if body
                    .last()
                    .is_some_and(|previous| previous.trim_end().ends_with('\\'))
                {
                    // Continuation of the command above.
                    body.push(lines[index]);
                    index += 1;
                } else {
                    break;
                }
            }
            push_block(
                &mut document,
                body.join("\n"),
                None,
                &headings,
                take_paragraph(&mut paragraph),
                start,
            );
            continue;
        }

        // Indented block ----------------------------------------------------
        if is_indented_code(line) && previous_line_is_blank(&lines, index) {
            let start = index + 1;
            let mut body: Vec<&str> = Vec::new();
            while index < lines.len() && (is_indented_code(lines[index]) || lines[index].trim().is_empty()) {
                body.push(lines[index]);
                index += 1;
            }
            while body.last().is_some_and(|last| last.trim().is_empty()) {
                body.pop();
            }
            let dedented: Vec<String> = body.iter().map(|line| dedent(line)).collect();
            push_block(
                &mut document,
                dedented.join("\n"),
                None,
                &headings,
                take_paragraph(&mut paragraph),
                start,
            );
            continue;
        }

        // Standalone inline code line ----------------------------------------
        if let Some(inline) = standalone_inline_code(trimmed) {
            push_block(
                &mut document,
                inline,
                None,
                &headings,
                take_paragraph(&mut paragraph),
                index + 1,
            );
            index += 1;
            continue;
        }

        // Prose --------------------------------------------------------------
        if trimmed.is_empty() {
            // A blank line ends the current paragraph but keeps it available as
            // context for whatever block comes next.
        } else if !is_list_marker(trimmed) {
            paragraph.push(trimmed.to_string());
        } else {
            paragraph.clear();
        }
        index += 1;
    }

    document
}

fn push_block(
    document: &mut ParsedDocument,
    body: String,
    language: Option<String>,
    headings: &[(usize, String)],
    context: Option<String>,
    line: usize,
) {
    if body.trim().is_empty() {
        return;
    }

    let session = extract_session(&body);

    document.blocks.push(RawBlock {
        content: session.content,
        language,
        heading_path: headings.iter().map(|(_, text)| text.clone()).collect(),
        context,
        line,
        from_prompt_session: session.from_prompt_session,
        dropped_output_lines: session.dropped,
    });
}

struct Session {
    content: String,
    from_prompt_session: bool,
    dropped: usize,
}

/// A pasted shell session mixes commands with their output. When prompts are
/// present, only the prompted lines (and their `\` continuations) are commands.
fn extract_session(body: &str) -> Session {
    let lines: Vec<&str> = body.lines().collect();
    let prompted = lines.iter().filter(|line| starts_with_prompt(line.trim_start())).count();

    if prompted == 0 {
        return Session {
            content: body.trim_end().to_string(),
            from_prompt_session: false,
            dropped: 0,
        };
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut dropped = 0;
    let mut continuing = false;

    for line in &lines {
        let trimmed = line.trim_start();
        // A prompted line is a command; so is the line after one that ended in
        // a backslash, because the command carried on.
        if starts_with_prompt(trimmed) || continuing {
            kept.push(line);
            continuing = line.trim_end().ends_with('\\');
        } else if !trimmed.is_empty() {
            dropped += 1;
        }
    }

    Session {
        content: kept.join("\n").trim_end().to_string(),
        from_prompt_session: true,
        dropped,
    }
}

fn fence_marker(trimmed: &str) -> Option<String> {
    for marker in ["```", "~~~"] {
        if trimmed.starts_with(marker) {
            let count = trimmed.chars().take_while(|c| *c == marker.chars().next().unwrap()).count();
            return Some(marker.chars().next().unwrap().to_string().repeat(count));
        }
    }
    None
}

fn language_from_info(info: &str) -> Option<String> {
    let first = info
        .trim_start_matches(['{', '.'])
        .split([' ', ',', ':', '}'])
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

fn heading(trimmed: &str) -> Option<(usize, String)> {
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((level, clean_heading(rest)))
}

fn clean_heading(text: &str) -> String {
    text.trim()
        .trim_end_matches('#')
        .trim()
        .replace(['`', '*', '_'], "")
        .trim()
        .to_string()
}

fn starts_with_prompt(trimmed: &str) -> bool {
    PROMPTS.iter().any(|prompt| trimmed.starts_with(prompt))
}

fn is_indented_code(line: &str) -> bool {
    if line.trim().is_empty() {
        return false;
    }
    let indented = line.starts_with("    ") || line.starts_with('\t');
    indented && !is_list_marker(line.trim_start())
}

fn previous_line_is_blank(lines: &[&str], index: usize) -> bool {
    index == 0 || lines[index - 1].trim().is_empty()
}

fn is_list_marker(trimmed: &str) -> bool {
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || trimmed
            .split_once(". ")
            .is_some_and(|(head, _)| !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()))
}

fn dedent(line: &str) -> String {
    line.strip_prefix("    ")
        .or_else(|| line.strip_prefix('\t'))
        .unwrap_or(line)
        .to_string()
}

/// A line that is nothing but a single inline code span, which cheat sheets use
/// constantly.
fn standalone_inline_code(trimmed: &str) -> Option<String> {
    let inner = trimmed.strip_prefix('`')?.strip_suffix('`')?;
    if inner.is_empty() || inner.contains('`') {
        return None;
    }
    // A short prose-looking span is more likely emphasis than a command.
    if !inner.contains(' ') && !inner.contains('-') && inner.len() < 4 {
        return None;
    }
    Some(inner.to_string())
}

fn take_paragraph(paragraph: &mut Vec<String>) -> Option<String> {
    if paragraph.is_empty() {
        return None;
    }
    let text = paragraph.join(" ").trim().to_string();
    paragraph.clear();

    let cleaned = text
        .replace('`', "")
        .replace("**", "")
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_string();

    if cleaned.len() < 3 || !cleaned.contains(' ') {
        return None;
    }
    Some(cleaned)
}
