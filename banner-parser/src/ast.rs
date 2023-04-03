use std::ops;

use crate::grammar::Rule;
use pest::Span;
use tracing::debug;

fn span_into_str(span: Span) -> &str {
    debug!("SPAN: {}", span.as_str());
    span.as_str()
}

#[derive(Debug, Clone)]
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

#[derive(Debug, FromPest, Clone)]
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

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::tag))]
pub struct Tag {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub key: String,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub value: String,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::import_declaration))]
pub struct Import {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub uri: String,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::mount))]
pub struct Mount {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub source: String,
    pub destination: StringLiteral,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::env_var))]
pub struct EnviromentVariable {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub key: String,
    pub value: StringLiteral,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::image_specification))]
pub struct Image {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    pub mounts: Vec<Mount>,
    pub envs: Vec<EnviromentVariable>,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::let_statement))]
pub struct Images {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    pub image: Image,
}

#[derive(Debug, Clone)]
pub struct JobSpecification {
    pub name: String,
    pub tasks: Vec<String>,
}

impl<'a> ::from_pest::FromPest<'a> for JobSpecification {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() == Rule::job_specification {
            let mut inner = pair.into_inner();
            let inner = &mut inner;
            let this = JobSpecification {
                name: Result::unwrap(str::parse(span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                ))),
                tasks: inner
                    .into_iter()
                    .map(|p| {
                        let span = p.as_span();
                        debug!("THE SPAN: {:?}", span);
                        span.as_str().to_string()
                    })
                    .collect(),
            };
            if inner.clone().next().is_some() {
                {
                    panic!(
                        "when converting JobSpecification, found extraneous {0:?}",
                        inner
                    )
                }
            }
            *pest = clone;
            Ok(this)
        } else {
            Err(::from_pest::ConversionError::NoMatch)
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineSpecification {
    pub name: String,
    pub jobs: Vec<String>,
}

impl<'a> ::from_pest::FromPest<'a> for PipelineSpecification {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() == Rule::pipeline_specification {
            let mut inner = pair.into_inner();
            let inner = &mut inner;
            let this = PipelineSpecification {
                name: Result::unwrap(str::parse(span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                ))),
                jobs: inner
                    .into_iter()
                    .map(|p| {
                        let span = p.as_span();
                        debug!("THE SPAN: {:?}", span);
                        span.as_str().to_string()
                    })
                    .collect(),
            };
            if inner.clone().next().is_some() {
                {
                    panic!(
                        "when converting JobSpecification, found extraneous {0:?}",
                        inner
                    )
                }
            }
            *pest = clone;
            Ok(this)
        } else {
            Err(::from_pest::ConversionError::NoMatch)
        }
    }
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::pipeline_definition))]
pub struct Pipeline {
    pub imports: Vec<Import>,
    pub images: Vec<Images>,
    pub tasks: Vec<Task>,
    pub jobs: Vec<JobSpecification>,
    pub pipelines: Vec<PipelineSpecification>,
    eoi: EOI,
}

impl ops::Add<Pipeline> for Pipeline {
    type Output = Pipeline;

    fn add(self, rhs: Pipeline) -> Pipeline {
        Pipeline {
            imports: self
                .imports
                .into_iter()
                .chain(rhs.imports)
                .collect::<Vec<_>>(),
            images: self
                .images
                .into_iter()
                .chain(rhs.images)
                .collect::<Vec<_>>(),
            tasks: self.tasks.into_iter().chain(rhs.tasks).collect::<Vec<_>>(),
            jobs: self.jobs.into_iter().chain(rhs.jobs).collect::<Vec<_>>(),
            pipelines: self
                .pipelines
                .into_iter()
                .chain(rhs.pipelines)
                .collect::<Vec<_>>(),
            eoi: self.eoi,
        }
    }
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::EOI))]
struct EOI;
