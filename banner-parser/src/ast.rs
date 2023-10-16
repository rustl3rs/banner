use std::{fmt::Display, ops};

use crate::{from_pest::FromPest, grammar::Rule};

use pest::iterators::{Pair, Pairs};

#[derive(Debug, Clone, PartialEq)]
pub enum StringLiteral {
    RawString(u8, String),
    StringLiteral(String),
}

impl Default for StringLiteral {
    fn default() -> Self {
        Self::StringLiteral("".into())
    }
}

impl<'a> ::from_pest::FromPest<'a> for StringLiteral {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        tracing::trace!("STRING_LITERAL: pair: {pair:?}");
        match pair.as_rule() {
            Rule::raw_string => {
                let this = StringLiteral::from_raw_string(pair);
                tracing::trace!("raw_string = {this:?}");
                *pest = clone;
                Ok(this)
            }
            Rule::standard_string => {
                let this = StringLiteral::StringLiteral(pair.into_inner().as_str().to_string());
                tracing::trace!("string_literal = {this:?}");
                *pest = clone;
                Ok(this)
            }
            _ => {
                tracing::trace!("StringLiteral NoMatch");
                Err(::from_pest::ConversionError::NoMatch)
            }
        }
    }
}

impl StringLiteral {
    pub fn as_str(&self) -> &str {
        match self {
            StringLiteral::RawString(_, inner) => inner,
            StringLiteral::StringLiteral(inner) => inner,
        }
    }

    pub fn from_raw_string(raw: Pair<Rule>) -> StringLiteral {
        if raw.as_rule() != Rule::raw_string {
            panic!("Expected raw_string, got {:?}", raw);
        };
        // subtract 1 for the starting `r` char
        // since we are zero based, we don't need to subtract an extra 1 for the ending `"`
        let count = raw.as_str().find('\"').unwrap() as u8 - 1;
        let raw = raw.into_inner().next().unwrap().as_str().to_string();
        StringLiteral::RawString(count, raw)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pragma {
    pub context: String,
    pub src: String,
}

impl<'pest> FromPest<'pest> for Pragma {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("PRAGMA: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let mut pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("PRAGMA: pair = {:#?}", pair);

        if pair.as_rule() != Rule::identifier {
            tracing::trace!("PRAGMA: Pragma.context NoMatch");
            return Err(from_pest::ConversionError::NoMatch);
        }

        let context = pair.as_str().to_string();
        tracing::trace!("PRAGMA: context = {:#?}", context);

        pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("PRAGMA: pair = {:#?}", pair);

        let src = match pair.as_rule() {
            Rule::pragma_body | Rule::pragma_identifier => pair.as_str().to_string(),
            _ => {
                tracing::trace!("PRAGMA: Pragma.value NoMatch");
                return Err(from_pest::ConversionError::NoMatch);
            }
        };

        let pragma = Pragma { context, src };

        *pest = clone;
        Ok(pragma)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskSpecification {
    pub tags: Vec<Tag>,
    pub name: String,
    pub image: String,
    pub command: StringLiteral,
    pub script: StringLiteral,
}

impl<'pest> FromPest<'pest> for TaskSpecification {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("TASK: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let mut pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("TASK: pair = {:#?}", pair);

        let mut task_def = TaskSpecification {
            ..Default::default()
        };
        tracing::trace!("TASK: default task_def = {:#?}", task_def);

        while pair.as_rule() == Rule::tag {
            tracing::trace!("TAG pair = {:#?}", pair);
            let tag = Tag::from_pest(&mut pair.into_inner())?;
            task_def.tags.push(tag);
            pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        }
        tracing::trace!("TASK: task_def with tags = {:#?}", task_def);

        tracing::trace!("TASK: pair = {:#?}", pair);
        if pair.as_rule() != Rule::identifier {
            tracing::trace!("TASK: Task.Name NoMatch");
            return Err(from_pest::ConversionError::NoMatch);
        }
        task_def.name = pair.as_str().to_string();
        tracing::trace!("TASK: task_def with name = {:#?}", task_def);

        pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("TASK: pair = {:#?}", pair);
        if pair.as_rule() != Rule::image_identifier {
            tracing::trace!("TASK: Task.Image NoMatch");
            return Err(from_pest::ConversionError::NoMatch);
        }
        task_def.image = pair.as_str().to_string();
        tracing::trace!("TASK: task_def with image = {:#?}", task_def);

        pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("TASK: pair = {:#?}", pair);
        if pair.as_rule() != Rule::string_literal {
            tracing::trace!("TASK: Task.Command NoMatch");
            return Err(from_pest::ConversionError::NoMatch);
        }
        task_def.command = StringLiteral::from_pest(&mut pair.into_inner())?;
        tracing::trace!("TASK: task_def with command = {:#?}", task_def);

        pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("TASK: pair = {:#?}", pair);
        if pair.as_rule() != Rule::raw_string {
            tracing::trace!("TTASK: ask.Script NoMatch");
            return Err(from_pest::ConversionError::NoMatch);
        }
        task_def.script = StringLiteral::from_raw_string(pair);
        tracing::trace!("TASK: task_def with script = {:#?}", task_def);

        *pest = clone;
        Ok(task_def)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl<'pest> FromPest<'pest> for Tag {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("TAG: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("TAG: pair = {:#?}", pair);

        // because the parser has already returned a valid set of pairs, we can
        // safely assume that the first pair is a key and the second pair is a value
        // because to get here, we had to validate that we had a TAG rule
        let key = pair.as_str().to_string();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        let value = pair.as_str().to_string();

        let tag = Tag { key, value };
        *pest = clone;
        Ok(tag)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Import {
    pub uri: String,
}

impl<'pest> FromPest<'pest> for Import {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("IMPORT: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("IMPORT: pair = {:#?}", pair);

        let uri = pair.as_str().to_string();

        let import = Import { uri };
        *pest = clone;
        Ok(import)
    }
}

#[derive(Debug, Clone)]
pub enum MountSource {
    EngineSupplied(String),
    Identifier(String),
    StringLiteral(String),
}

impl Default for MountSource {
    fn default() -> Self {
        Self::StringLiteral("".into())
    }
}

impl<'a> ::from_pest::FromPest<'a> for MountSource {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;
    fn from_pest(
        pest: &mut ::from_pest::pest::iterators::Pairs<'a, Rule>,
    ) -> ::std::result::Result<Self, ::from_pest::ConversionError<::from_pest::Void>> {
        tracing::trace!("MOUNTSOURCE: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(::from_pest::ConversionError::NoMatch)?;
        match pair.as_rule() {
            Rule::variable => {
                let this = MountSource::EngineSupplied(pair.into_inner().as_str().to_string());
                tracing::trace!("variable = {this:?}");
                *pest = clone;
                Ok(this)
            }
            Rule::pipe_job_task_identifier => {
                let this = MountSource::Identifier(pair.as_span().as_str().to_string());
                *pest = clone;
                Ok(this)
            }
            Rule::string_literal => {
                let mut inner = pair.clone().into_inner();
                let inner = &mut inner;
                let this = MountSource::StringLiteral(inner.as_str().to_string());
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

#[derive(Debug, Clone, Default)]
pub struct Mount {
    pub source: MountSource,
    pub destination: StringLiteral,
}

impl<'pest> FromPest<'pest> for Mount {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("MOUNT: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("MOUNT: pair = {:#?}", pair);

        let source = match pair.as_rule() {
            Rule::variable => MountSource::EngineSupplied(pair.into_inner().as_str().to_string()),
            Rule::pipe_job_task_identifier => MountSource::Identifier(pair.as_str().to_string()),
            Rule::string_literal => MountSource::StringLiteral(pair.as_str().to_string()),
            _ => return Err(from_pest::ConversionError::NoMatch),
        };
        tracing::trace!("MOUNT: source = {:#?}", source);

        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("MOUNT: pair = {:#?}", pair);
        let destination = match pair.as_rule() {
            Rule::string_literal => StringLiteral::from_pest(&mut pair.into_inner())?,
            _ => return Err(from_pest::ConversionError::NoMatch),
        };
        tracing::trace!("MOUNT: destination = {:#?}", destination);

        let mount = Mount {
            source,
            destination,
        };
        *pest = clone;
        Ok(mount)
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: StringLiteral,
}

impl<'pest> FromPest<'pest> for EnvironmentVariable {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("ENV_VAR: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("ENV_VAR: pair = {:#?}", pair);

        let key = pair.as_str().to_string();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        let value = StringLiteral::from_pest(&mut pair.into_inner())?;

        let env_var = EnvironmentVariable { key, value };
        *pest = clone;
        Ok(env_var)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Image {
    pub name: String,
    pub mounts: Vec<Mount>,
    pub envs: Vec<EnvironmentVariable>,
}

impl<'pest> FromPest<'pest> for Image {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("IMAGE: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("IMAGE: pair = {:#?}", pair);

        let mut image = Image {
            name: pair.as_str().to_string(),
            ..Default::default()
        };

        let mut pair = clone.next();
        while pair.is_some() {
            let possible_mount = pair.clone().unwrap();
            if possible_mount.as_rule() != Rule::mount {
                break;
            }
            let mount = Mount::from_pest(&mut possible_mount.into_inner())?;
            image.mounts.push(mount);
            tracing::trace!("IMAGE: move next");
            pair = clone.next();
            tracing::trace!("IMAGE: move next done!");
        }

        while pair.is_some() {
            let possible_var = pair.clone().unwrap();
            if possible_var.as_rule() != Rule::env_var {
                break;
            }
            let env_var = EnvironmentVariable::from_pest(&mut possible_var.into_inner())?;
            image.envs.push(env_var);
            pair = clone.next();
        }

        if pair.is_some() {
            panic!(
                "when converting MountSource::StringLiteral, found extraneous {0:?}",
                pair.unwrap().as_str()
            )
        }

        *pest = clone;
        Ok(image)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageDefinition {
    pub name: String,
    pub image: Image,
}

impl<'pest> FromPest<'pest> for ImageDefinition {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        tracing::trace!("IMAGE_DEFINITION: pest = {:#?}", pest);
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("IMAGE_DEFINITION: pair = {:#?}", pair);

        let name = pair.as_str().to_string();
        tracing::trace!("IMAGE_DEFINITION: name = {:#?}", name);
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        tracing::trace!("IMAGE_DEFINITION: pair = {:#?}", pair);
        let image = Image::from_pest(&mut pair.into_inner())?;

        let image_def = ImageDefinition { name, image };
        *pest = clone;
        Ok(image_def)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdentifierMarker {
    JobMacro,
    Optional,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdentifierListItem {
    Identifier(String, Vec<IdentifierMarker>),
    SequentialList(Vec<IdentifierListItem>),
    ParallelList(Vec<IdentifierListItem>),
}

pub struct IdentifierListItemIter<'a> {
    stack: Vec<&'a IdentifierListItem>,
}

impl IdentifierListItem {
    pub fn iter(&self) -> IdentifierListItemIter {
        IdentifierListItemIter { stack: vec![self] }
    }
}

impl<'a> Iterator for IdentifierListItemIter<'a> {
    type Item = &'a IdentifierListItem;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.stack.pop() {
            match item {
                IdentifierListItem::Identifier(_, _) => {
                    return Some(item);
                }
                IdentifierListItem::SequentialList(list)
                | IdentifierListItem::ParallelList(list) => {
                    self.stack.extend(list);
                }
            }
        }

        None
    }
}

impl Default for IdentifierListItem {
    fn default() -> Self {
        Self::Identifier("".to_string(), vec![])
    }
}

fn from_ident_list(pest: &mut Pairs<Rule>) -> Vec<IdentifierListItem> {
    let mut tasks = Vec::<IdentifierListItem>::new();
    let mut clone = pest.clone();
    if clone.peek().is_none() {
        return tasks;
    };

    loop {
        let pair = clone.next();
        if pair.is_none() {
            break;
        }

        let pair = pair.unwrap();

        match pair.as_rule() {
            Rule::identifier => {
                let task = IdentifierListItem::Identifier(pair.as_str().to_string(), vec![]);
                if let Some(next) = clone.peek() {
                    match next.as_rule() {
                        Rule::job_macro_marker => {
                            let mut markers = Vec::<IdentifierMarker>::new();
                            loop {
                                if clone.peek().is_none() || clone.peek().unwrap().as_rule() != Rule::job_macro_marker {
                                    break;
                                }
                                let pair = clone.next().unwrap();
                                if pair.as_rule() != Rule::job_macro_marker {
                                    break;
                                }
                                let marker = match pair.as_str() {
                                    // "?" => IdentifierMarker::Optional,
                                    "!" => IdentifierMarker::JobMacro,
                                    _ => panic!("unexpected job macro marker: {}", pair.as_str()),
                                };
                                markers.push(marker);
                            }

                            tasks.push(IdentifierListItem::Identifier(
                                task.to_string(),
                                markers,
                            ));
                        }
                        Rule::identifier => {
                            tasks.push(task);
                        }
                        Rule::sequential_identifier_list  |
                        Rule::parallel_identifier_list => {
                            tasks.push(task);
                        }
                        _ => panic!(
                            "expected identifier or job_macro_marker, found {:#?}",
                            next.as_rule()
                        ),
                    }
                } else {
                    tasks.push(task);
                }
            }
            Rule::sequential_identifier_list => {
                let sequential_tasks = from_ident_list(&mut pair.into_inner());
                tasks.push(IdentifierListItem::SequentialList(sequential_tasks));
            }
            Rule::parallel_identifier_list => {
                let parallel_tasks = from_ident_list(&mut pair.into_inner());
                tasks.push(IdentifierListItem::ParallelList(parallel_tasks));
            }
            _ => panic!(
                "expected identifier, sequential_identifier_list or parallel_identifier_list, found {:#?}",
                pair.as_rule()
            ),
        }
    }

    tasks
}

impl Display for IdentifierListItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierListItem::SequentialList(tasks) => {
                for task in tasks.iter() {
                    write!(f, "{},", task)?
                }
            }
            IdentifierListItem::ParallelList(tasks) => {
                for task in tasks.iter() {
                    write!(f, "{},", task)?
                }
            }
            IdentifierListItem::Identifier(task_name, markers) => {
                write!(f, "{}", task_name)?;
                for marker in markers.iter() {
                    match marker {
                        IdentifierMarker::JobMacro => write!(f, "!")?,
                        IdentifierMarker::Optional => write!(f, "?")?,
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobSpecification {
    pub name: String,
    pub tasks: Vec<IdentifierListItem>,
}

impl JobSpecification {
    pub fn all_tasks(&self) -> Vec<String> {
        let mut tasks = Vec::<String>::new();
        for task in self.tasks.iter() {
            match task {
                IdentifierListItem::Identifier(task_name, _) => {
                    tasks.push(task_name.to_string());
                }
                IdentifierListItem::SequentialList(task_list) => {
                    for task in task_list.iter() {
                        match task {
                            IdentifierListItem::Identifier(task_name, _) => {
                                tasks.push(task_name.to_string());
                            }
                            _ => panic!("expected IdentifierListItem::Identifier"),
                        }
                    }
                }
                IdentifierListItem::ParallelList(task_list) => {
                    for task in task_list.iter() {
                        match task {
                            IdentifierListItem::Identifier(task_name, _) => {
                                tasks.push(task_name.to_string());
                            }
                            _ => panic!("expected IdentifierListItem::Identifier"),
                        }
                    }
                }
            }
        }
        tasks
    }
}

impl<'pest> FromPest<'pest> for JobSpecification {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() != Rule::identifier {
            return Err(from_pest::ConversionError::NoMatch);
        }

        let mut ident_list = clone.next().unwrap().into_inner();

        let job_spec = JobSpecification {
            name: pair.as_str().to_string(),
            tasks: from_ident_list(&mut ident_list),
        };

        *pest = clone;
        Ok(job_spec)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PipelineSpecification {
    pub name: String,
    pub jobs: Vec<IdentifierListItem>,
}

impl PipelineSpecification {
    pub fn iter_jobs(&self) -> impl Iterator<Item = &IdentifierListItem> {
        let jobs: Vec<&IdentifierListItem> = self.jobs.iter().collect();
        let iterator = IdentifierListItemIter { stack: jobs };
        iterator
    }
}

impl<'pest> FromPest<'pest> for PipelineSpecification {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() != Rule::identifier {
            return Err(from_pest::ConversionError::NoMatch);
        }

        let mut ident_list = clone.next().unwrap().into_inner();

        let pipeline_spec = PipelineSpecification {
            name: pair.as_str().to_string(),
            jobs: from_ident_list(&mut ident_list),
        };

        *pest = clone;
        Ok(pipeline_spec)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Pipeline {
    pub pragmas: Vec<Pragma>,
    pub imports: Vec<Import>,
    pub images: Vec<ImageDefinition>,
    pub tasks: Vec<TaskSpecification>,
    pub jobs: Vec<JobSpecification>,
    pub pipelines: Vec<PipelineSpecification>,
    eoi: EndOfInput,
}

impl<'pest> FromPest<'pest> for Pipeline {
    type Rule = Rule;
    type FatalError = ::from_pest::Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let mut clone = pest.clone();
        let pair = clone.next().ok_or(from_pest::ConversionError::NoMatch)?;
        if pair.as_rule() != Rule::pipeline_definition {
            return Err(from_pest::ConversionError::NoMatch);
        }

        let mut pipeline = Pipeline {
            ..Default::default()
        };
        for pair in pair.into_inner().clone() {
            tracing::trace!("pair = {pair:?}");
            let mut inner = pair.clone().into_inner();
            // let inner = &mut inner;
            match pair.as_rule() {
                Rule::pipeline_specification => {
                    tracing::trace!("pipeline_specification");
                    let this = PipelineSpecification::from_pest(&mut inner)?;
                    pipeline.pipelines.push(this);
                }
                Rule::job_specification => {
                    tracing::trace!("job_specification");
                    let this = JobSpecification::from_pest(&mut inner)?;
                    pipeline.jobs.push(this);
                }
                Rule::import_declaration => {
                    tracing::trace!("import_declaration");
                    let this = Import::from_pest(&mut inner)?;
                    pipeline.imports.push(this);
                }
                Rule::let_statement => {
                    tracing::trace!("let_statement");
                    let this = ImageDefinition::from_pest(&mut inner)?;
                    pipeline.images.push(this);
                }
                Rule::task_definition => {
                    tracing::trace!("task_definition");
                    let this = TaskSpecification::from_pest(&mut inner)?;
                    pipeline.tasks.push(this);
                }
                Rule::pragma => {
                    tracing::trace!("pragma");
                    let this = Pragma::from_pest(&mut inner)?;
                    pipeline.pragmas.push(this);
                }
                Rule::EOI => {}
                _ => panic!("unexpected pair: {pair:?}"),
            }
        }
        Ok(pipeline)
    }
}

impl ops::Add<Pipeline> for Pipeline {
    type Output = Pipeline;

    fn add(self, rhs: Pipeline) -> Pipeline {
        Pipeline {
            pragmas: self
                .pragmas
                .into_iter()
                .chain(rhs.pragmas)
                .collect::<Vec<_>>(),
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

#[derive(Debug, Clone, FromPest, Default)]
#[pest_ast(rule(Rule::EOI))]
struct EndOfInput;
