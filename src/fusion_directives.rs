use crate::Error;
use full_moon::{
    ast::{Ast, Block, TableConstructor},
    tokenizer::{Symbol, Token, TokenReference, TokenType},
    visitors::Visitor,
};

#[derive(Default)]
struct Validator {
    scopes: Vec<usize>,
    next_scope: usize,
    blocks: usize,
    regions: Vec<(Vec<usize>, usize)>,
    pending_block_scope: Option<Vec<usize>>,
    next_closer: Option<usize>,
    table_openers: std::collections::HashSet<usize>,
    table_closers: std::collections::HashSet<usize>,
    can_enable: bool,
    error: Option<String>,
}

impl Validator {
    fn enter_scope(&mut self) {
        self.next_scope += 1;
        self.scopes.push(self.next_scope);
    }

    fn track_braces(&mut self, (open, close): (&TokenReference, &TokenReference)) {
        self.table_openers
            .insert(open.token().start_position().bytes());
        self.table_closers
            .insert(close.token().start_position().bytes());
    }

    fn boundary_comment(&mut self, token: &Token) {
        let parent = self.scopes.clone();
        if self
            .next_closer
            .is_some_and(|position| token.start_position().bytes() < position)
        {
            if let Some(scope) = &self.pending_block_scope {
                self.scopes = scope.clone();
            }
        }
        self.comment(token);
        self.scopes = parent;
    }

    fn comment(&mut self, token: &Token) {
        let comment = match token.token_type() {
            TokenType::SingleLineComment { comment }
            | TokenType::MultiLineComment { comment, .. } => comment,
            _ => return,
        };
        for (offset, line) in comment.lines().enumerate() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("styluau: fusion") else {
                continue;
            };
            if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let number = token.start_position().line() + offset;
            let words: Vec<_> = rest.split_whitespace().collect();
            let problem = if rest != format!(" {}", words.join(" ")) {
                Some("malformed command".to_owned())
            } else {
                match words.as_slice() {
                    ["on"] => {
                        self.can_enable = true;
                        None
                    }
                    ["off"] => None,
                    ["on", "start"] | ["off", "start"] => {
                        self.can_enable |= words[0] == "on";
                        self.regions.push((self.scopes.clone(), number));
                        None
                    }
                    ["end"] => match self.regions.last() {
                        None => Some("unmatched end".to_owned()),
                        Some((scope, _)) if *scope != self.scopes => {
                            Some("end crosses a block or table boundary".to_owned())
                        }
                        Some(_) => {
                            self.regions.pop();
                            None
                        }
                    },
                    ["scope", name] | ["not-scope", name] if valid_identifier(name) => None,
                    ["scope", _] | ["not-scope", _] => Some("invalid Luau identifier".to_owned()),
                    _ => Some("malformed command".to_owned()),
                }
            };
            if let Some(problem) = problem {
                if self.error.is_none() {
                    self.error = Some(format!("{problem} on line {number}: `{line}`"));
                }
            }
        }
    }
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        && !matches!(
            name,
            "and"
                | "break"
                | "do"
                | "else"
                | "elseif"
                | "end"
                | "false"
                | "for"
                | "function"
                | "if"
                | "in"
                | "local"
                | "nil"
                | "not"
                | "or"
                | "repeat"
                | "return"
                | "then"
                | "true"
                | "until"
                | "while"
        )
}

impl Visitor for Validator {
    fn visit_block(&mut self, _: &Block) {
        if self.blocks > 0 {
            self.enter_scope();
        }
        self.blocks += 1;
    }

    fn visit_block_end(&mut self, _: &Block) {
        self.blocks -= 1;
        if self.blocks > 0 {
            self.pending_block_scope = Some(self.scopes.clone());
            self.scopes.pop();
        }
    }

    fn visit_table_constructor(&mut self, table: &TableConstructor) {
        self.track_braces(table.braces().tokens());
    }

    #[cfg(feature = "luau")]
    fn visit_type_info(&mut self, type_info: &full_moon::ast::luau::TypeInfo) {
        // Table-type fields consume Fusion regions the same way value-table fields do, so
        // their braces must delimit regions too
        if let full_moon::ast::luau::TypeInfo::Table { braces, .. } = type_info {
            self.track_braces(braces.tokens());
        }
    }

    fn visit_token(&mut self, token: &Token) {
        match token.token_type() {
            TokenType::Symbol {
                symbol: Symbol::End | Symbol::Else | Symbol::ElseIf | Symbol::Until,
            } => self.next_closer = Some(token.start_position().bytes()),
            TokenType::SingleLineComment { .. }
            | TokenType::MultiLineComment { .. }
            | TokenType::Whitespace { .. } => (),
            _ => {
                self.pending_block_scope = None;
                self.next_closer = None;
            }
        }
    }

    fn visit_symbol(&mut self, token: &Token) {
        let position = token.start_position().bytes();
        if self.table_openers.remove(&position) {
            self.enter_scope();
        } else if self.table_closers.remove(&position) {
            self.scopes.pop();
        }
        self.pending_block_scope = None;
        self.next_closer = None;
    }

    fn visit_single_line_comment(&mut self, token: &Token) {
        self.boundary_comment(token);
    }

    fn visit_multi_line_comment(&mut self, token: &Token) {
        self.boundary_comment(token);
    }
}

pub(crate) fn validate(ast: &Ast) -> Result<bool, Error> {
    let mut validator = Validator::default();
    validator.visit_ast(ast);
    if let Some(error) = validator.error {
        return Err(Error::FusionDirectiveError(error));
    }
    if let Some((_, line)) = validator.regions.last() {
        return Err(Error::FusionDirectiveError(format!(
            "unclosed region starting on line {line}"
        )));
    }
    Ok(validator.can_enable)
}
