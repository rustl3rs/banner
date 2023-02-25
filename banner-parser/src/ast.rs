use crate::grammar::Rule;
use pest::Span;
use tracing::debug;

fn span_into_str(span: Span) -> &str {
    debug!("SPAN: {}", span.as_str());
    span.as_str()
}

#[derive(Debug)]
pub enum StringLiteral {
    RawString(String),
    StringLiteral(String),
}

impl<'a> ::from_pest::FromPest<'a> for StringLiteral {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() == Rule::string_literal {
            let this = Err(::from_pest::ConversionError::NoMatch)
                .or_else(|_: ::from_pest::ConversionError<::from_pest::Void>| {
                    let mut inner = pair.clone().into_inner();
                    let inner = &mut inner;
                    let this = StringLiteral::RawString(Result::unwrap(str::parse(span_into_str(
                        inner
                            .next()
                            .ok_or(::from_pest::ConversionError::NoMatch)?
                            .as_span(),
                    ))));
                    if inner.clone().next().is_some() {
                        {
                            panic!(
                                "when converting StringLiteral::Raw, found extraneous {0:?}",
                                inner
                            )
                        }
                    }
                    Ok(this)
                })
                .or_else(|_: ::from_pest::ConversionError<::from_pest::Void>| {
                    let mut inner = pair.clone().into_inner();
                    let inner = &mut inner;
                    let this =
                        StringLiteral::StringLiteral(Result::unwrap(str::parse(span_into_str(
                            inner
                                .next()
                                .ok_or(::from_pest::ConversionError::NoMatch)?
                                .as_span(),
                        ))));
                    if inner.clone().next().is_some() {
                        {
                            panic!(
                                "when converting StringLiteral::String, found extraneous {0:?}",
                                inner
                            )
                        }
                    }
                    Ok(this)
                })?;
            *pest = clone;
            Ok(this)
        } else {
            Err(::from_pest::ConversionError::NoMatch)
        }
    }
}

impl StringLiteral {
    pub fn as_str(&self) -> &str {
        match self {
            StringLiteral::RawString(inner) => inner,
            StringLiteral::StringLiteral(inner) => inner,
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::task_definition))]
pub struct Task {
    pub tags: Vec<Tag>,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub image: String,
    pub command: StringLiteral,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub script: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tag))]
pub struct Tag {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub key: String,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub value: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::import_declaration))]
pub struct Import {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub uri: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::pipeline_definition))]
pub struct Pipeline {
    pub imports: Vec<Import>,
    pub tasks: Vec<Task>,
    eoi: EOI,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct EOI;
