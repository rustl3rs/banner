use crate::grammar::Rule;
use pest::Span;

fn span_into_str(span: Span) -> &str {
    span.as_str()
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::identifier))]
pub struct Identifier {
    #[pest_ast(outer(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::image_identifier))]
pub struct ImageIdentifier {
    #[pest_ast(outer(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub image: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::string_content))]
pub struct StringContent {
    #[pest_ast(outer(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub content: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::raw_string_interior))]
pub struct RawString {
    #[pest_ast(outer(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub content: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::string_literal))]
pub enum StringLiteral {
    Raw(RawString),
    String(StringContent),
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::task_definition))]
pub struct Task {
    pub name: Identifier,
    pub image_identifier: ImageIdentifier,
    pub execute_command: StringLiteral,
    pub script: RawString,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::pipeline_definition))]
pub struct Pipeline {
    pub tasks: Vec<Task>,
    eoi: EOI,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct EOI;
