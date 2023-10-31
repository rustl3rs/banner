use std::error::Error;

use from_pest::FromPest;
use pest::{Parser, Span};
use tracing::trace;

use crate::image_ref;

extern crate pest;

// TODO: I need to think about this more. I'm not certain this isn't the right name
#[allow(clippy::module_name_repetitions)]
#[derive(Parser)]
#[grammar = "grammar/image_ref.pest"]
pub struct ImageRefParser;

/// Parses a docker URI.
///
/// # Panics
///
/// Panics if the parsed input cannot be converted to an AST.
///
/// # Errors
///
/// This function will return an error if the input cannot be parsed.
pub fn parse_docker_uri(input: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    trace!("docker ref = {:#?}", input);
    let mut tree = ImageRefParser::parse(image_ref::Rule::reference, input)?;
    trace!("image_ref_tree: {tree:?}");
    let _parsed = image_ref::ImageRef::from_pest(&mut tree).unwrap();
    Ok(())
}

fn span_into_str(span: Span) -> &str {
    span.as_str()
}

fn span_into_optional_str(span: Span) -> Option<String> {
    let result = span_into_str(span);
    match result {
        "" => None,
        _ => Some(result.to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct ImageRef {
    pub r#ref: String,
    pub tag: Option<String>,
    pub digest: Option<String>,

    // without this, the compiler complains that EOI is never used. But it is used, in the FromPest impl.
    #[allow(dead_code)]
    eoi: EndOfInput,
}

#[derive(Debug, FromPest, Clone)]
#[pest_ast(rule(Rule::EOI))]
struct EndOfInput;

impl<'a> ::from_pest::FromPest<'a> for ImageRef {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() == Rule::reference {
            let mut inner = pair.into_inner();
            let inner = &mut inner;
            let this = ImageRef {
                r#ref: Result::unwrap(str::parse(span_into_str(
                    inner
                        .next()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_span(),
                ))),
                tag: {
                    if inner
                        .peek()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_rule()
                        == Rule::tag
                    {
                        trace!("TAG is: {:?}", inner.peek().unwrap().as_span().as_str());
                        span_into_optional_str(
                            inner
                                .next()
                                .ok_or(::from_pest::ConversionError::NoMatch)?
                                .as_span(),
                        )
                    } else {
                        trace!("TAG is None");
                        None::<String>
                    }
                },
                digest: {
                    if inner
                        .peek()
                        .ok_or(::from_pest::ConversionError::NoMatch)?
                        .as_rule()
                        == Rule::digest
                    {
                        trace!("DIGEST is: {:?}", inner.peek().unwrap().as_span().as_str());
                        span_into_optional_str(
                            inner
                                .next()
                                .ok_or(::from_pest::ConversionError::NoMatch)?
                                .as_span(),
                        )
                    } else {
                        trace!("Digest is None");
                        None::<String>
                    }
                },
                eoi: ::from_pest::FromPest::from_pest(inner)?,
            };
            assert!(
                inner.clone().next().is_none(),
                "when converting ImageRef, found extraneous {inner:?}"
            );
            *pest = clone;
            Ok(this)
        } else {
            Err(::from_pest::ConversionError::NoMatch)
        }
    }
}

#[cfg(test)]
mod docker_uri_parser_tests {
    use std::error::Error;

    use pest::Parser;
    use tracing::trace;
    use tracing_test::traced_test;

    use crate::image_ref;

    use super::*;

    fn parse_docker_domain(input: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        trace!("docker domain_name = {:#?}", input);
        let tree = ImageRefParser::parse(image_ref::Rule::domain, input)?;
        println!("tree: {tree:?}");
        Ok(())
    }

    fn parse_docker_domain_component(input: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        trace!("docker domain_component = {:#?}", input);
        let tree = ImageRefParser::parse(image_ref::Rule::domain_component, input)?;
        println!("tree: {tree:?}");

        Ok(())
    }

    fn check_domain(input: &str) {
        match parse_docker_domain(input) {
            Ok(()) => {}
            Err(e) => {
                panic!("Failed to parse domain: {e:#?}");
            }
        }
    }

    fn check_domain_component(input: &str) {
        match parse_docker_domain_component(input) {
            Ok(()) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    fn check(input: &str) {
        match parse_docker_uri(input) {
            Ok(()) => {}
            Err(e) => {
                panic!("{e:#?}");
            }
        }
    }

    fn check_invalid(input: &str) {
        if let Ok(()) = parse_docker_uri(input) {
            panic!("this should be invalid: {input}");
        }
    }

    #[test]
    #[traced_test]
    fn test_docker_images() {
        let name = "registry";
        check_domain_component(name);

        let name = "registry-test";
        check_domain_component(name);

        let name = "registry-test.eu-west-1.elb.amazonaws.com:5000";
        check_domain(name);

        let name = "x10k.com:80";
        check_domain(name);

        let ref_string = "localhost/test";
        check(ref_string);

        let ref_string = "localhost/test:latest";
        check(ref_string);

        let ref_string = "localhost:80/test:latest";
        check(ref_string);

        let ref_string = "registry-test.eu-west-1.elb.amazonaws.com/rustl3rs/test";
        check(ref_string);

        let ref_string = "registry-test.eu-west-1.elb.amazonaws.com/rustl3rs/test:latest";
        check(ref_string);

        let ref_string = "registry-test.eu-west-1.elb.amazonaws.com:5000/rustl3rs/test:latest";
        check(ref_string);

        let ref_string = "rust:latest";
        check(ref_string);

        let ref_string = "rust";
        check(ref_string);

        let ref_string = "local'host/test:latest";
        check_invalid(ref_string);
    }
}
