use std::ops;

use crate::grammar::Rule;
use pest::{iterators::Pairs, Span};
use tracing::debug;

fn span_into_str(span: Span) -> &str {
    span.as_str()
}

fn inner_into_vec_of_string(span: Span) -> Vec<String> {
    println!("span_into_vec_of_string: {:#?}", span);
    let result = span
        .as_str()
        .split_whitespace()
        .map(|s| s.trim_end_matches(",").to_string())
        .collect();
    println!("{result:?}");
    result
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
            let mut literal = pair.clone().into_inner();
            let pair = literal
                .next()
                .ok_or(::from_pest::ConversionError::NoMatch)?;
            match pair.as_rule() {
                Rule::raw_string => {
                    let mut inner = pair.clone().into_inner();
                    let inner = &mut inner;
                    let this = StringLiteral::RawString(
                        span_into_str(
                            inner
                                .next()
                                .ok_or(::from_pest::ConversionError::NoMatch)?
                                .as_span(),
                        )
                        .to_string(),
                    );
                    tracing::trace!("raw_string = {this:?}");
                    tracing::trace!("clone: {clone:?}");
                    *pest = clone;
                    Ok(this)
                }
                Rule::standard_string => {
                    let mut inner = pair.clone().into_inner();
                    let inner = &mut inner;
                    let this = StringLiteral::StringLiteral(
                        span_into_str(
                            inner
                                .next()
                                .ok_or(::from_pest::ConversionError::NoMatch)?
                                .as_span(),
                        )
                        .to_string(),
                    );
                    tracing::trace!("string_literal = {this:?}");
                    tracing::trace!("clone: {clone:?}");
                    *pest = clone;
                    Ok(this)
                }
                _ => {
                    tracing::trace!("StringLiteral NoMatch");
                    Err(::from_pest::ConversionError::NoMatch)
                }
            }
        } else if pair.as_rule() == Rule::raw_string {
            let mut clone = pest.clone();
            let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
            let mut inner = pair.clone().into_inner();
            let inner = &mut inner;
            let this = StringLiteral::RawString(
                span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                )
                .to_string(),
            );
            tracing::trace!("raw_string = {this:?}");
            tracing::trace!("clone: {clone:?}");
            *pest = clone;
            Ok(this)
        } else {
            tracing::trace!("StringLiteral NoMatch");
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

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::task_definition))]
pub struct Task {
    pub tags: Vec<Tag>,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub image: String,
    pub command: StringLiteral,
    pub script: StringLiteral,
}

// impl<'a> ::from_pest::FromPest<'a> for Task {
//     type Rule = Rule;
//     type FatalError = ::from_pest::Void;
//     fn from_pest(
//         pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
//     ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
//         let mut clone = pest.clone();
//         let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
//         println!("=====> TASK_SPEC: {pair:?}");
//         if pair.as_rule() == Rule::task_definition {
//             let mut inner = pair.into_inner();
//             let inner = &mut inner;
//             let this = Task { tags: ::from_pest::FromPest(), name: (), image: (), command: (), script: () } {
//                 name: Result::unwrap(str::parse(span_into_str(
//                     inner
//                         .next()
//                         .ok_or(::from_pest::ConversionError::NoMatch)?
//                         .as_span(),
//                 ))),
//                 tasks: inner
//                     .into_iter()
//                     .map(|p| {
//                         // we need to strip out the trailing ","
//                         let inner = p.into_inner().into_iter().next().unwrap();
//                         let span = inner.as_span();
//                         debug!("THE JOB SPAN: {:?}", span);
//                         span.as_str().to_string()
//                     })
//                     .collect(),
//             };
//             if inner.clone().next().is_some() {
//                 {
//                     panic!(
//                         "when converting JobSpecification, found extraneous {0:?}",
//                         inner
//                     )
//                 }
//             }
//             println!("=====> {clone:?}");
//             // let clone = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?; // need to be rid of the NEWLINE
//             *pest = clone; // need to be rid of the NEWLINE
//             Ok(this)
//         } else {
//             tracing::trace!("JobSpec NoMatch: {pair:?}");
//             Err(::from_pest::ConversionError::NoMatch)
//         }
//     }
// }

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

#[derive(Debug, Clone)]
pub enum MountSource {
    EngineSupplied(String),
    Identifier(String),
    StringLiteral(String),
}
impl<'a> ::from_pest::FromPest<'a> for MountSource {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        match pair.as_rule() {
            Rule::variable => {
                let mut inner = pair.clone().into_inner();
                let inner = &mut inner;
                let this = MountSource::EngineSupplied(Result::unwrap(str::parse(span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                ))));
                tracing::trace!("variable = {this:?}");
                *pest = clone;
                Ok(this)
            }
            Rule::pipe_job_task_identifier => {
                let this = MountSource::Identifier(pair.clone().as_span().as_str().to_string());
                *pest = clone;
                Ok(this)
            }
            Rule::string_literal => {
                let mut inner = pair.clone().into_inner();
                let inner = &mut inner;
                let this = MountSource::StringLiteral(Result::unwrap(str::parse(span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                ))));
                if inner.clone().next().is_some() {
                    {
                        panic!(
                            "when converting MountSource::StringLiteral, found extraneous {0:?}",
                            inner
                        )
                    }
                }
                *pest = clone;
                Ok(this)
            }
            _ => {
                tracing::trace!("MountSource NoMatch");
                Err(::from_pest::ConversionError::NoMatch)
            }
        }
    }
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::mount))]
pub struct Mount {
    pub source: MountSource,
    pub destination: StringLiteral,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::env_var))]
pub struct EnvironmentVariable {
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
    pub envs: Vec<EnvironmentVariable>,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::let_statement))]
pub struct Images {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    pub image: Image,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::identifier))]
pub struct Identifier {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
}

impl Identifier {
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl From<String> for Identifier {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::job_specification))]
pub struct JobSpecification {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    #[pest_ast(inner(with(inner_into_vec_of_string)))]
    pub tasks: Vec<String>,
}

// impl<'a> ::from_pest::FromPest<'a> for JobSpecification {
//     type Rule = Rule;
//     type FatalError = ::from_pest::Void;
//     fn from_pest(
//         pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
//     ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
//         let mut clone = pest.clone();
//         let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
//         println!("=====> JOB_SPEC: {pair:?}");
//         if pair.as_rule() == Rule::job_specification {
//             let mut inner = pair.into_inner();
//             let inner = &mut inner;
//             let this = JobSpecification {
//                 name: Result::unwrap(str::parse(span_into_str(
//                     inner
//                         .next()
//                         .ok_or(::from_pest::ConversionError::NoMatch)?
//                         .as_span(),
//                 ))),
//                 tasks: inner
//                     .into_iter()
//                     .map(|p| {
//                         // we need to strip out the trailing ","
//                         let inner = p.into_inner().into_iter().next().unwrap();
//                         let span = inner.as_span();
//                         debug!("THE JOB SPAN: {:?}", span);
//                         span.as_str().to_string()
//                     })
//                     .collect(),
//             };
//             if inner.clone().next().is_some() {
//                 {
//                     panic!(
//                         "when converting JobSpecification, found extraneous {0:?}",
//                         inner
//                     )
//                 }
//             }
//             println!("=====> {clone:?}");
//             // let clone = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?; // need to be rid of the NEWLINE
//             *pest = clone; // need to be rid of the NEWLINE
//             Ok(this)
//         } else {
//             tracing::trace!("JobSpec NoMatch: {pair:?}");
//             Err(::from_pest::ConversionError::NoMatch)
//         }
//     }
// }

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::pipeline_specification))]
pub struct PipelineSpecification {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub name: String,
    #[pest_ast(inner(with(inner_into_vec_of_string)))]
    pub jobs: Vec<String>,
}

// impl<'a> ::from_pest::FromPest<'a> for PipelineSpecification {
//     type Rule = Rule;
//     type FatalError = ::from_pest::Void;
//     fn from_pest(
//         pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
//     ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
//         let mut clone = pest.clone();
//         let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
//         if pair.as_rule() == Rule::pipeline_specification {
//             let mut inner = pair.into_inner();
//             let inner = &mut inner;
//             let this = PipelineSpecification {
//                 name: Result::unwrap(str::parse(span_into_str(
//                     inner
//                         .next()
//                         .ok_or(::from_pest::ConversionError::NoMatch)?
//                         .as_span(),
//                 ))),
//                 jobs: inner
//                     .into_iter()
//                     .map(|p| {
//                         // we need to strip out the trailing ","
//                         let inner = p.into_inner().into_iter().next().unwrap();
//                         let span = inner.as_span();
//                         debug!("THE PIPELINE SPAN: {:?}", span);
//                         span.as_str().to_string()
//                     })
//                     .collect(),
//             };
//             if inner.clone().next().is_some() {
//                 {
//                     panic!(
//                         "when converting JobSpecification, found extraneous {0:?}",
//                         inner
//                     )
//                 }
//             }
//             *pest = clone;
//             Ok(this)
//         } else {
//             tracing::trace!("PipelineSpec NoMatch: {pair:?}");
//             Err(::from_pest::ConversionError::NoMatch)
//         }
//     }
// }

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
