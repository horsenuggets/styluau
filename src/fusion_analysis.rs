use full_moon::{
    ast::{
        Assignment, Ast, Block, Call, Expression, FunctionArgs, FunctionCall, Index,
        LocalAssignment, Parameter, Prefix, Suffix, Var,
    },
    node::Node,
    tokenizer::{TokenReference, TokenType},
    visitors::Visitor,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq)]
enum Binding {
    Module,
    Scoped,
    Scope,
}

fn name(token: &TokenReference) -> String {
    token.token().to_string()
}

#[derive(Default)]
struct Writes {
    names: HashSet<String>,
    declared: HashSet<String>,
    member_write: bool,
}

impl Writes {
    fn declaration(&mut self, token: &TokenReference) {
        let identifier = name(token);
        if identifier == "require" || !self.declared.insert(identifier.clone()) {
            self.names.insert(identifier);
        }
    }
}

impl Visitor for Writes {
    fn visit_local_assignment(&mut self, assignment: &LocalAssignment) {
        for token in assignment.names() {
            self.declaration(token);
        }
    }
    #[cfg(feature = "luau")]
    fn visit_const_assignment(&mut self, assignment: &full_moon::ast::luau::ConstAssignment) {
        for token in assignment.names() {
            self.declaration(token);
        }
    }
    #[cfg(feature = "luau")]
    fn visit_const_function(&mut self, function: &full_moon::ast::luau::ConstFunction) {
        self.names.insert(name(function.name()));
    }
    #[cfg(feature = "luau")]
    fn visit_if_condition_binding(&mut self, binding: &full_moon::ast::luau::IfConditionBinding) {
        self.names.insert(name(binding.name()));
    }
    fn visit_local_function(&mut self, function: &full_moon::ast::LocalFunction) {
        self.names.insert(name(function.name()));
    }
    fn visit_function_declaration(&mut self, function: &full_moon::ast::FunctionDeclaration) {
        if function.name().method_name().is_some() {
            self.names.insert("self".to_owned());
        }
        if function.name().method_name().is_some() || function.name().names().len() > 1 {
            self.member_write = true;
        }
        if let Some(token) = function.name().names().iter().next() {
            self.names.insert(name(token));
        }
    }
    fn visit_numeric_for(&mut self, statement: &full_moon::ast::NumericFor) {
        self.names.insert(name(statement.index_variable()));
    }
    fn visit_generic_for(&mut self, statement: &full_moon::ast::GenericFor) {
        for token in statement.names() {
            self.names.insert(name(token));
        }
    }
    #[cfg(any(feature = "luau", feature = "cfxlua"))]
    fn visit_compound_assignment(&mut self, assignment: &full_moon::ast::CompoundAssignment) {
        match assignment.lhs() {
            Var::Name(token) => {
                self.names.insert(name(token));
            }
            _ => self.member_write = true,
        }
    }
    fn visit_assignment(&mut self, assignment: &Assignment) {
        for variable in assignment.variables() {
            match variable {
                Var::Name(token) => {
                    self.names.insert(name(token));
                }
                Var::Expression(expression) => {
                    self.member_write = true;
                    if let Prefix::Name(token) = expression.prefix() {
                        self.names.insert(name(token));
                    }
                }
                _ => (),
            }
        }
    }
    fn visit_parameter(&mut self, parameter: &Parameter) {
        if let Parameter::Name(token) = parameter {
            self.names.insert(name(token));
        }
    }
}

struct Analyzer {
    scopes: Vec<HashMap<String, Option<Binding>>>,
    writes: HashSet<String>,
    calls: HashSet<usize>,
}

impl Analyzer {
    fn lookup(&self, identifier: &str) -> Option<Binding> {
        if self.writes.contains(identifier) {
            return None;
        }
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(identifier))
            .copied()
            .flatten()
    }

    fn expression(&self, expression: &Expression) -> Option<Binding> {
        match expression {
            Expression::Parentheses { expression, .. } => self.expression(expression),
            Expression::Var(Var::Name(token)) => self.lookup(&name(token)),
            Expression::Var(Var::Expression(expression)) => {
                let Prefix::Name(prefix) = expression.prefix() else {
                    return None;
                };
                let suffixes: Vec<_> = expression.suffixes().collect();
                match suffixes.as_slice() {
                    [Suffix::Index(Index::Dot { name: member, .. })]
                        if self.lookup(&name(prefix)) == Some(Binding::Module)
                            && name(member) == "scoped" =>
                    {
                        Some(Binding::Scoped)
                    }
                    _ => None,
                }
            }
            Expression::FunctionCall(call) => self.scope_call(call),
            _ => None,
        }
    }

    fn scope_call(&self, call: &FunctionCall) -> Option<Binding> {
        let Prefix::Name(prefix) = call.prefix() else {
            return None;
        };
        let identifier = name(prefix);
        let suffixes: Vec<_> = call.suffixes().collect();
        match suffixes.as_slice() {
            [Suffix::Call(Call::AnonymousCall(args))]
                if identifier == "require" && !self.writes.contains("require") =>
            {
                let literal = match args.as_ref() {
                    FunctionArgs::String(token) => Some(token),
                    FunctionArgs::Parentheses { arguments, .. } if arguments.len() == 1 => {
                        match arguments.iter().next()? {
                            Expression::String(token) => Some(token),
                            _ => None,
                        }
                    }
                    _ => None,
                }?;
                if let TokenType::StringLiteral { literal, .. } = literal.token_type() {
                    if literal.rsplit('/').next() == Some("Fusion") {
                        return Some(Binding::Module);
                    }
                }
                None
            }
            [Suffix::Call(Call::MethodCall(method))] => {
                match (self.lookup(&identifier), name(method.name()).as_str()) {
                    (Some(Binding::Module), "scoped")
                    | (Some(Binding::Scope), "innerScope" | "deriveScope") => Some(Binding::Scope),
                    _ => None,
                }
            }
            [Suffix::Index(Index::Dot { name: member, .. }), Suffix::Call(Call::AnonymousCall(_))]
                if self.lookup(&identifier) == Some(Binding::Module)
                    && name(member) == "scoped" =>
            {
                Some(Binding::Scope)
            }
            [Suffix::Call(Call::AnonymousCall(_))]
                if self.lookup(&identifier) == Some(Binding::Scoped) =>
            {
                Some(Binding::Scope)
            }
            _ => None,
        }
    }
}

impl Visitor for Analyzer {
    fn visit_block(&mut self, _: &Block) {
        self.scopes.push(HashMap::new());
    }
    fn visit_block_end(&mut self, _: &Block) {
        self.scopes.pop();
    }
    fn visit_local_assignment_end(&mut self, assignment: &LocalAssignment) {
        let bindings: Vec<_> = assignment
            .names()
            .iter()
            .enumerate()
            .map(|(index, token)| {
                (
                    name(token),
                    assignment
                        .expressions()
                        .iter()
                        .nth(index)
                        .and_then(|expression| self.expression(expression)),
                )
            })
            .collect();
        self.scopes.last_mut().unwrap().extend(bindings);
    }
    #[cfg(feature = "luau")]
    fn visit_const_assignment_end(&mut self, assignment: &full_moon::ast::luau::ConstAssignment) {
        let bindings: Vec<_> = assignment
            .names()
            .iter()
            .enumerate()
            .map(|(index, token)| {
                (
                    name(token),
                    assignment
                        .expressions()
                        .iter()
                        .nth(index)
                        .and_then(|expression| self.expression(expression)),
                )
            })
            .collect();
        self.scopes.last_mut().unwrap().extend(bindings);
    }
    fn visit_function_call(&mut self, call: &FunctionCall) {
        if let Prefix::Name(prefix) = call.prefix() {
            if matches!(
                self.lookup(&name(prefix)),
                Some(Binding::Module | Binding::Scope)
            ) {
                if let Some(position) = call.start_position() {
                    self.calls.insert(position.bytes());
                }
            }
        }
    }
}

pub(crate) fn analyze(ast: &Ast) -> HashSet<usize> {
    let mut writes = Writes::default();
    writes.visit_ast(ast);
    // Aliases can share a mutable module or scope. Without object identity analysis,
    // member writes make import-based recognition uncertain across the file.
    if writes.member_write {
        return HashSet::new();
    }
    let mut analyzer = Analyzer {
        scopes: Vec::new(),
        writes: writes.names,
        calls: HashSet::new(),
    };
    analyzer.visit_ast(ast);
    analyzer.calls
}
