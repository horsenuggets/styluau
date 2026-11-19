use crate::{
    comment_markdown::reflow_markdown,
    context::{create_indent_trivia, create_newline_trivia, line_ending_character, Context},
    shape::Shape,
    Error,
};
use full_moon::{
    ast::Ast,
    tokenizer::{Token, TokenType},
    visitors::Visitor,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CommentState {
    group: usize,
    enabled: bool,
}

#[derive(Debug, Default)]
pub(crate) struct CommentControls {
    comments: HashMap<usize, CommentState>,
}

#[derive(Default)]
struct Tokens(Vec<Token>);

impl Visitor for Tokens {
    fn visit_token(&mut self, token: &Token) {
        if !matches!(
            token.token_type(),
            TokenType::Whitespace { .. } | TokenType::Eof
        ) {
            self.0.push(token.clone());
        }
    }
}

fn comment_text(token: &Token) -> Option<&str> {
    match token.token_type() {
        TokenType::SingleLineComment { comment } | TokenType::MultiLineComment { comment, .. } => {
            Some(comment)
        }
        _ => None,
    }
}

fn protected(text: &str) -> bool {
    // Rewrapping prose must not turn an inline directive mention into an active command.
    text.contains("[styluau]")
        || text.contains("stylua:")
        || text.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with('!') || line.starts_with('@') || line.starts_with("-@")
        })
}

impl CommentControls {
    pub(crate) fn analyze(
        ast: &Ast,
        enabled: bool,
        scopes: &HashMap<usize, Vec<usize>>,
    ) -> Result<Self, Error> {
        let mut tokens = Tokens::default();
        tokens.visit_ast(ast);
        tokens.0.sort_by_key(|token| token.start_position().bytes());
        let tokens = tokens.0;
        let mut controls = Self::default();
        let mut enabled = enabled;
        let mut stack = Vec::new();
        let mut pending = None;
        let mut ignored_scopes = HashSet::new();
        let mut previous_code_line = 0;
        let mut previous_group: Option<(usize, usize, bool)> = None;
        for token in &tokens {
            let Some(text) = comment_text(token) else {
                previous_code_line = token.end_position().line();
                previous_group = None;
                continue;
            };
            let scope = &scopes[&token.start_position().bytes()];
            for (offset, line) in text.lines().enumerate() {
                let line = line.trim();
                if matches!(line, "[styluau] ignore start" | "stylua: ignore start") {
                    ignored_scopes.insert(scope.clone());
                } else if matches!(line, "[styluau] ignore end" | "stylua: ignore end") {
                    ignored_scopes.remove(scope);
                }
                let Some(command) = line.strip_prefix("[styluau] comments") else {
                    continue;
                };
                if !command.is_empty() && !command.starts_with(char::is_whitespace) {
                    continue;
                }
                let number = token.start_position().line() + offset;
                let error = match command {
                    " on" => {
                        pending = Some(true);
                        None
                    }
                    " off" => {
                        pending = Some(false);
                        None
                    }
                    " on start" | " off start" => {
                        stack.push((enabled, number));
                        enabled = command == " on start";
                        None
                    }
                    " end" => match stack.pop() {
                        Some((previous, _)) => {
                            enabled = previous;
                            None
                        }
                        None => Some("unmatched end"),
                    },
                    _ => Some("malformed command"),
                };
                if let Some(error) = error {
                    return Err(Error::CommentDirectiveError(format!(
                        "{} on line {}. `{}`",
                        error, number, line
                    )));
                }
            }
            let start = token.start_position();
            let end = token.end_position();
            // Leading block comments are placed on their own line even when code follows them.
            let standalone = previous_code_line < start.line();
            if !standalone || protected(text) {
                previous_group = None;
                continue;
            }
            let single_line = matches!(token.token_type(), TokenType::SingleLineComment { .. });
            let (group, state) = match previous_group {
                Some((group, last_line, state)) if single_line && last_line + 1 == start.line() => {
                    (group, state)
                }
                _ => (start.bytes(), pending.take().unwrap_or(enabled)),
            };
            let ignored = (0..=scope.len()).any(|length| ignored_scopes.contains(&scope[..length]));
            controls.comments.insert(
                start.bytes(),
                CommentState {
                    group,
                    enabled: state && !ignored,
                },
            );
            previous_group = if single_line {
                Some((group, end.line(), state))
            } else {
                None
            };
        }
        if let Some((_, line)) = stack.last() {
            return Err(Error::CommentDirectiveError(format!(
                "unclosed region starting on line {}",
                line
            )));
        }
        Ok(controls)
    }

    fn state(&self, token: &Token) -> Option<CommentState> {
        self.comments.get(&token.start_position().bytes()).copied()
    }
}

/// Returns fully rendered leading trivia and the number of original trivia tokens consumed.
pub(crate) fn format_group(
    ctx: &Context,
    trivia: &[&Token],
    shape: Shape,
) -> Option<(Vec<Token>, usize)> {
    let first = *trivia.first()?;
    let state = ctx.comment_controls().state(first)?;
    if !state.enabled {
        return None;
    }
    let width = ctx
        .config()
        .comment_width
        .saturating_sub(shape.indent().indent_width())
        .max(1);
    let (output, mut consumed) = match first.token_type() {
        TokenType::SingleLineComment { .. } => {
            let mut lines = Vec::new();
            let mut consumed = 0;
            loop {
                let token = *trivia.get(consumed)?;
                let TokenType::SingleLineComment { comment } = token.token_type() else {
                    break;
                };
                if ctx.comment_controls().state(token).map(|s| s.group) != Some(state.group) {
                    break;
                }
                // Strip only the conventional separator, not Markdown indentation or hard breaks.
                lines.push(
                    comment
                        .strip_prefix(' ')
                        .unwrap_or(comment)
                        .trim_end_matches('\r'),
                );
                consumed += 1;
                let mut next_index = consumed;
                let mut newlines = 0;
                while let Some(whitespace) = trivia.get(next_index) {
                    let TokenType::Whitespace { characters } = whitespace.token_type() else {
                        break;
                    };
                    newlines += characters.matches('\n').count();
                    next_index += 1;
                }
                let Some(next) = trivia.get(next_index) else {
                    break;
                };
                if newlines != 1
                    || ctx.comment_controls().state(next).map(|s| s.group) != Some(state.group)
                {
                    break;
                }
                consumed = next_index;
            }
            let markdown = reflow_markdown(&lines.join("\n"), width.saturating_sub(3).max(1));
            let mut output = Vec::new();
            for line in markdown.split('\n') {
                output.push(create_indent_trivia(ctx, shape));
                output.push(Token::new(TokenType::SingleLineComment {
                    comment: if line.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", line)
                    }
                    .into(),
                }));
                output.push(create_newline_trivia(ctx));
            }
            (output, consumed)
        }
        TokenType::MultiLineComment { blocks, comment } => {
            let normalized = comment.replace("\r\n", "\n");
            let original_indent = first.start_position().character().saturating_sub(1);
            let lines: Vec<_> = normalized.split('\n').collect();
            let strip_indent = |line: &str| -> String {
                let mut removed = 0;
                line.chars()
                    .skip_while(|c| {
                        if removed < original_indent && matches!(c, ' ' | '\t') {
                            removed += 1;
                            true
                        } else {
                            false
                        }
                    })
                    .collect()
            };
            let markdown = lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        (*line).to_owned()
                    } else {
                        strip_indent(line)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let closing_marker = markdown.ends_with("\n--");
            let body = if closing_marker {
                &markdown[..markdown.len() - 2]
            } else {
                &markdown
            };
            let compact = !body.contains('\n');
            let mut markdown = if compact {
                reflow_markdown(body, width.saturating_sub(6 + 2 * blocks).max(1))
            } else {
                String::new()
            };
            if !compact || markdown.contains('\n') {
                // Once a block needs wrapping, put its delimiters on their own lines. This
                // keeps the prose budget independent of its input's previous wrapping.
                let mut body = body.to_owned();
                if !body.starts_with('\n') {
                    body.insert(0, '\n');
                }
                if !body.ends_with('\n') {
                    body.push('\n');
                }
                markdown = reflow_markdown(&body, width);
            }
            if closing_marker {
                markdown.push_str("--");
            }
            let indent = create_indent_trivia(ctx, shape).to_string();
            let last_line = markdown.split('\n').count() - 1;
            let comment = markdown
                .split('\n')
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 || (line.is_empty() && i != last_line) {
                        line.to_owned()
                    } else {
                        format!("{}{}", indent, line)
                    }
                })
                .collect::<Vec<_>>()
                .join(&line_ending_character(ctx.config().line_endings));
            (
                vec![
                    create_indent_trivia(ctx, shape),
                    Token::new(TokenType::MultiLineComment {
                        blocks: *blocks,
                        comment: comment.into(),
                    }),
                    create_newline_trivia(ctx),
                ],
                1,
            )
        }
        _ => return None,
    };
    // Each rendered comment already terminates its line. Keep a physical blank-line gap.
    if let Some(next) = trivia.get(consumed) {
        if let TokenType::Whitespace { characters } = next.token_type() {
            if characters.matches('\n').count() == 1 {
                consumed += 1;
            }
        }
    }
    Some((output, consumed))
}
