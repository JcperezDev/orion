use tree_sitter::Parser;

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub full_text: String,
}

pub fn parse_commands(input: &str) -> Vec<ParsedCommand> {
    let mut parser = Parser::new();
    if parser.set_language(&tree_sitter_bash::LANGUAGE.into()).is_err() {
        return fallback_parse(input);
    }
    let tree = match parser.parse(input, None) {
        Some(t) => t,
        None => return fallback_parse(input),
    };
    let root = tree.root_node();
    let mut results = Vec::new();
    extract_commands(root, input, &mut results);
    if results.is_empty() {
        return fallback_parse(input);
    }
    results
}

fn extract_commands(node: tree_sitter::Node, source: &str, out: &mut Vec<ParsedCommand>) {
    match node.kind() {
        "command" | "cURL_command" => {
            // Skip commands that are nested inside command_substitution
            // (e.g., the `whoami` inside `$(whoami)`).
            let skip = node.parent().map_or(false, |p| {
                matches!(p.kind(), "command_substitution" | "string_expansion" | "brace_expansion")
            });
            if !skip {
                if let Some(cmd) = extract_single_command(node, source) {
                    out.push(cmd);
                }
            }
            // Don't recurse into children — command children are sub-parts, not separate commands.
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                extract_commands(child, source, out);
            }
        }
    }
}

fn extract_single_command(node: tree_sitter::Node, source: &str) -> Option<ParsedCommand> {
    let full_text = node.utf8_text(source.as_bytes()).ok()?.to_string();
    let mut cmd_name = String::new();
    let mut args = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "command_name" => {
                cmd_name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            "command_substitution" | "string" | "word" | "raw_string" | "concatenation"
            | "expansion" | "file_redirect" | "file_descriptor" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    args.push(text.to_string());
                }
            }
            _ => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    args.push(text.to_string());
                }
            }
        }
    }
    if cmd_name.is_empty() {
        cmd_name = full_text.split_whitespace().next()?.to_string();
    }
    Some(ParsedCommand {
        command: cmd_name,
        args,
        full_text,
    })
}

fn fallback_parse(input: &str) -> Vec<ParsedCommand> {
    let mut results = Vec::new();
    for segment in split_into_segments(input) {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first_word = trimmed.split_whitespace().next().unwrap_or("").to_string();
        let args: Vec<String> = trimmed
            .split_whitespace()
            .skip(1)
            .map(|s| s.to_string())
            .collect();
        results.push(ParsedCommand {
            command: first_word,
            args,
            full_text: trimmed.to_string(),
        });
    }
    results
}

fn split_into_segments(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '&' if !in_single_quote && !in_double_quote && i + 1 < chars.len() && chars[i + 1] == '&' => {
                flush_segment(&mut current, &mut segments);
                i += 2;
                continue;
            }
            '|' if !in_single_quote && !in_double_quote => {
                // single pipe is pipeline, double pipe is logical or
                flush_segment(&mut current, &mut segments);
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            ';' if !in_single_quote && !in_double_quote => {
                flush_segment(&mut current, &mut segments);
                i += 1;
                continue;
            }
            _ => {}
        }
        current.push(ch);
        i += 1;
    }
    flush_segment(&mut current, &mut segments);
    segments
}

fn flush_segment(current: &mut String, segments: &mut Vec<String>) {
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }
    current.clear();
}

pub fn command_matches_glob(cmd: &ParsedCommand, glob: &globset::GlobMatcher) -> bool {
    glob.is_match(&cmd.full_text) || glob.is_match(&cmd.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmds = parse_commands("echo hello world");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "echo");
        assert_eq!(cmds[0].full_text, "echo hello world");
    }

    #[test]
    fn parse_compound_commands() {
        let cmds = parse_commands("git status && rm -rf node_modules");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].command, "git");
        assert_eq!(cmds[1].command, "rm");
    }

    #[test]
    fn parse_pipeline() {
        let cmds = parse_commands("cat file.txt | grep hello");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].command, "cat");
        assert_eq!(cmds[1].command, "grep");
    }

    #[test]
    fn parse_with_semicolon() {
        let cmds = parse_commands("cd src; npm test");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_empty_string() {
        let cmds = parse_commands("");
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let cmds = parse_commands("   ");
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_command_with_redirect() {
        let cmds = parse_commands("echo data > file.txt");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "echo");
    }

    #[test]
    fn parse_command_with_complex_args() {
        let cmds = parse_commands(r#"npm install --save-dev "typescript" 'prettier'"#);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "npm");
    }

    #[test]
    fn parse_heredoc_passes_through() {
        let cmds = parse_commands("cat <<EOF\nhello\nEOF");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "cat");
    }

    #[test]
    fn parse_subshell_in_string() {
        let cmds = parse_commands("echo $(whoami)");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "echo");
    }

    #[test]
    fn parse_backtick_subshell() {
        let cmds = parse_commands("echo `whoami`");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn parse_var_assignment() {
        let cmds = parse_commands("FOO=bar echo baz");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn parse_chained_pipeline_and_list() {
        let cmds = parse_commands("echo a | grep x && echo b; echo c");
        assert_eq!(cmds.len(), 4);
        assert_eq!(cmds[0].command, "echo");
        assert_eq!(cmds[1].command, "grep");
        assert_eq!(cmds[2].command, "echo");
        assert_eq!(cmds[3].command, "echo");
    }

    #[test]
    fn parse_quoted_semicolons_not_splits() {
        let cmds = parse_commands(r#"echo "hello; world""#);
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn parse_logical_or() {
        let cmds = parse_commands("false || echo fallback");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[1].command, "echo");
    }
}
