use crate::{
    formatters::trivia_util, shape::Shape, BlockNewlineGaps, CallParenType,
    CollapseSimpleStatement, Config, IndentType, LineEndings, Range as FormatRange,
    SpaceAfterFunctionNames,
};
use full_moon::{
    node::Node,
    tokenizer::{Token, TokenType},
};

#[derive(Debug, PartialEq, Eq)]
pub enum FormatNode {
    /// The formatting is completely blocked via an ignore comment, so this node should be skipped
    Skip,
    /// This node is outside the range, but we should still look to format internally to find items within the range
    NotInRange,
    /// There is no restriction, the node should be formatted normally
    Normal,
}

#[derive(Debug, Clone)]
pub struct Context {
    /// The configuration passed to the formatter
    config: std::sync::Arc<Config>,
    /// An optional range of values to format within the file.
    range: Option<FormatRange>,
    /// Whether the formatting has currently been disabled. This should occur when we see the relevant comment.
    formatting_disabled: bool,
    fusion_stack: Vec<bool>,
    fusion_calls: std::sync::Arc<std::collections::HashSet<usize>>,
    excluded_fusion_names: std::collections::HashSet<String>,
}

impl Context {
    /// Creates a new Context, with the given configuration
    pub fn new(config: Config, range: Option<FormatRange>) -> Self {
        Self {
            config: std::sync::Arc::new(config),
            range,
            formatting_disabled: false,
            fusion_stack: Vec::new(),
            fusion_calls: std::sync::Arc::new(std::collections::HashSet::new()),
            excluded_fusion_names: std::collections::HashSet::new(),
        }
    }

    pub fn with_fusion_calls(mut self, calls: std::collections::HashSet<usize>) -> Self {
        self.fusion_calls = std::sync::Arc::new(calls);
        self
    }

    pub fn is_fusion_receiver(&self, node: &impl Node, name: &str) -> bool {
        !self.excluded_fusion_names.contains(name)
            && (self
                .config
                .fusion_scope_names
                .iter()
                .any(|candidate| candidate == name)
                || node
                    .start_position()
                    .is_some_and(|position| self.fusion_calls.contains(&position.bytes())))
    }

    /// Get the configuration for this context
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Determines whether we need to toggle whether formatting is enabled or disabled.
    /// Formatting is toggled on/off whenever we see a `-- stylua: ignore start` or `-- stylua: ignore end` comment respectively.
    // To preserve immutability of Context, we return a new Context with the `formatting_disabled` field toggled or left the same
    // where necessary. Context is cheap so this is reasonable to do.
    pub fn check_toggle_formatting(&self, node: &impl Node) -> Self {
        // Load all the leading comments from the token
        let leading_trivia = node.surrounding_trivia().0;
        let comment_lines = leading_trivia
            .iter()
            .filter_map(|trivia| {
                match trivia.token_type() {
                    TokenType::SingleLineComment { comment } => Some(comment),
                    TokenType::MultiLineComment { comment, .. } => Some(comment),
                    _ => None,
                }
                .map(|comment| comment.lines().map(|line| line.trim()))
            })
            .flatten();

        // Load the current formatting disabled state
        let mut formatting_disabled = self.formatting_disabled;

        // Work through all the lines and update the state as necessary
        for line in comment_lines {
            if matches!(line, "styluau: ignore start" | "stylua: ignore start") {
                formatting_disabled = true;
            } else if matches!(line, "styluau: ignore end" | "stylua: ignore end") {
                formatting_disabled = false;
            }
        }

        Self {
            formatting_disabled,
            ..self.clone()
        }
    }

    /// Applies Fusion directives and returns the context for this node.
    pub fn apply_fusion_directives(&mut self, node: &impl Node) -> Self {
        let mut next_fusion = None;
        for trivia in node.surrounding_trivia().0 {
            let comment = match trivia.token_type() {
                TokenType::SingleLineComment { comment }
                | TokenType::MultiLineComment { comment, .. } => comment,
                _ => continue,
            };
            for line in comment.lines().map(str::trim) {
                let Some(directive) = line.strip_prefix("styluau: fusion ") else {
                    continue;
                };
                match directive {
                    "on" => next_fusion = Some(true),
                    "off" => next_fusion = Some(false),
                    "on start" | "off start" => {
                        self.fusion_stack.push(self.config.fusion);
                        std::sync::Arc::make_mut(&mut self.config).fusion = directive == "on start";
                    }
                    "end" => {
                        if let Some(previous) = self.fusion_stack.pop() {
                            std::sync::Arc::make_mut(&mut self.config).fusion = previous;
                        }
                    }
                    _ => {
                        if let Some(name) = directive.strip_prefix("scope ") {
                            self.excluded_fusion_names.remove(name);
                            let names =
                                &mut std::sync::Arc::make_mut(&mut self.config).fusion_scope_names;
                            if !names.iter().any(|existing| existing == name) {
                                names.push(name.to_owned());
                            }
                        } else if let Some(name) = directive.strip_prefix("not-scope ") {
                            self.excluded_fusion_names.insert(name.to_owned());
                            std::sync::Arc::make_mut(&mut self.config)
                                .fusion_scope_names
                                .retain(|existing| existing != name);
                        }
                    }
                }
            }
        }
        let mut node_context = self.clone();
        if let Some(fusion) = next_fusion {
            std::sync::Arc::make_mut(&mut node_context.config).fusion = fusion;
        }
        node_context
    }

    /// Checks whether we should format the given node.
    /// Firstly determine if formatting is disabled (due to the relevant comment)
    /// If not, determine whether the node has an ignore comment present.
    /// If not, checks whether the provided node is outside the formatting range.
    /// If not, the node should be formatted.
    pub fn should_format_node(&self, node: &impl Node) -> FormatNode {
        // If formatting is disabled we should immediately bailed out.
        if self.formatting_disabled {
            return FormatNode::Skip;
        }

        // Check comments
        let leading_trivia = node.surrounding_trivia().0;
        for trivia in leading_trivia {
            let comment_lines = match trivia.token_type() {
                TokenType::SingleLineComment { comment } => comment,
                TokenType::MultiLineComment { comment, .. } => comment,
                _ => continue,
            }
            .lines()
            .map(|line| line.trim());

            for line in comment_lines {
                if matches!(line, "styluau: ignore" | "stylua: ignore") {
                    return FormatNode::Skip;
                }
            }
        }

        if let Some(range) = self.range {
            match (range.start, node.start_position()) {
                (Some(start_bound), Some(node_start)) if node_start.bytes() < start_bound => {
                    return FormatNode::NotInRange
                }
                _ => (),
            };

            match (range.end, node.end_position()) {
                (Some(end_bound), Some(node_end)) if node_end.bytes() > end_bound => {
                    return FormatNode::NotInRange
                }
                _ => (),
            }
        }

        FormatNode::Normal
    }

    #[allow(deprecated)]
    pub fn should_omit_string_parens(&self) -> bool {
        self.config().no_call_parentheses
            || self.config().call_parentheses == CallParenType::None
            || self.config().call_parentheses == CallParenType::NoSingleString
    }

    #[allow(deprecated)]
    pub fn should_omit_table_parens(&self) -> bool {
        self.config().no_call_parentheses
            || self.config().call_parentheses == CallParenType::None
            || self.config().call_parentheses == CallParenType::NoSingleTable
    }

    /// Whether a function with a simple body should be collapsed onto a single line
    pub fn should_collapse_simple_function(&self, node: &impl Node) -> bool {
        matches!(
            self.config().collapse_simple_statement,
            CollapseSimpleStatement::FunctionOnly | CollapseSimpleStatement::Always
        ) || (self.should_preserve_input_simple_statements()
            && trivia_util::node_spans_single_line(node))
    }

    /// Whether a function with an empty body should be collapsed onto a single line
    pub fn should_collapse_empty_function(&self, node: &impl Node) -> bool {
        !self.should_preserve_input_simple_statements() || trivia_util::node_spans_single_line(node)
    }

    /// Whether an `if` guard with a simple body should be collapsed onto a single line
    pub fn should_collapse_simple_conditional(&self, node: &impl Node) -> bool {
        matches!(
            self.config().collapse_simple_statement,
            CollapseSimpleStatement::ConditionalOnly | CollapseSimpleStatement::Always
        ) || (self.should_preserve_input_simple_statements()
            && trivia_util::node_spans_single_line(node))
    }

    fn should_preserve_input_simple_statements(&self) -> bool {
        matches!(
            self.config().collapse_simple_statement,
            CollapseSimpleStatement::Input
        )
    }

    pub fn should_preserve_leading_block_newline_gaps(&self) -> bool {
        matches!(self.config().block_newline_gaps, BlockNewlineGaps::Preserve)
    }

    pub fn should_preserve_trailing_block_newline_gaps(&self) -> bool {
        matches!(self.config().block_newline_gaps, BlockNewlineGaps::Preserve)
    }
}

/// Returns the relevant line ending string from the [`LineEndings`] enum
pub fn line_ending_character(line_endings: LineEndings) -> String {
    match line_endings {
        LineEndings::Unix => String::from("\n"),
        LineEndings::Windows => String::from("\r\n"),
    }
}

/// Creates a new Token containing whitespace for indents, used for trivia
pub fn create_indent_trivia(ctx: &Context, shape: Shape) -> Token {
    let indent_level = shape.indent().block_indent() + shape.indent().additional_indent();
    create_plain_indent_trivia(ctx, indent_level)
}

/// Creates indent trivia without including `ctx.indent_level()`.
/// You should pass the exact amount of indent you require to this function
pub fn create_plain_indent_trivia(ctx: &Context, indent_level: usize) -> Token {
    match ctx.config().indent_type {
        IndentType::Tabs => Token::new(TokenType::tabs(indent_level)),
        IndentType::Spaces => {
            Token::new(TokenType::spaces(indent_level * ctx.config().indent_width))
        }
    }
}

/// Creates a new Token containing whitespace used after function declarations
pub fn create_function_definition_trivia(ctx: &Context) -> Token {
    match ctx.config().space_after_function_names {
        SpaceAfterFunctionNames::Always | SpaceAfterFunctionNames::Definitions => {
            Token::new(TokenType::spaces(1))
        }
        SpaceAfterFunctionNames::Never | SpaceAfterFunctionNames::Calls => {
            Token::new(TokenType::spaces(0))
        }
    }
}

/// Creates a new Token containing whitespace used after function calls
pub fn create_function_call_trivia(ctx: &Context) -> Token {
    match ctx.config().space_after_function_names {
        SpaceAfterFunctionNames::Always | SpaceAfterFunctionNames::Calls => {
            Token::new(TokenType::spaces(1))
        }
        SpaceAfterFunctionNames::Never | SpaceAfterFunctionNames::Definitions => {
            Token::new(TokenType::spaces(0))
        }
    }
}

/// Creates a new Token containing new line whitespace, used for trivia
pub fn create_newline_trivia(ctx: &Context) -> Token {
    Token::new(TokenType::Whitespace {
        characters: line_ending_character(ctx.config().line_endings).into(),
    })
}
