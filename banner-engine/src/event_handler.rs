use std::error::Error;
use std::sync::Arc;

use std::fmt::{Debug, Formatter};

use rune::{
    termcolor::{BufferWriter, ColorChoice},
    ContextError, Diagnostics, Module, Source, Sources, Vm,
};
use tokio::sync::broadcast::Sender;

use crate::ListenForEvents;
use crate::{
    rune_engine::EventHandlerEngine, Engine, Event, EventType, Metadata, SystemEventResult,
    SystemEventScope, SystemEventType, Tag,
};

#[derive(Clone)]
pub struct EventHandler {
    tags: Vec<Tag>,
    listen_for_events: ListenForEvents,
    script: String,
}

impl Debug for EventHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, r#"{{"#)?;

        writeln!(f, "    listen_for_events:")?;
        for event in &self.listen_for_events {
            writeln!(f, "        {event:?},")?;
        }

        writeln!(f, r#"    tags:"#)?;
        for tag in &self.tags {
            writeln!(f, "        {tag:?},")?;
        }

        writeln!(f, r#####"    script: ###""#####)?;
        writeln!(f, "        {}", self.script)?;
        writeln!(f, r#####"    "###"#####)?;

        write!(f, r#"}}"#)?;
        Ok(())
    }
}

impl EventHandler {
    pub fn new(tags: Vec<Tag>, listen_for_events: ListenForEvents, script: String) -> Self {
        Self {
            tags,
            listen_for_events,
            script,
        }
    }

    pub(crate) fn execute(
        &self,
        engine: Arc<dyn Engine + Sync + Send>,
        tx: &Sender<Event>,
        trigger: Event,
    ) {
        let script: &str = &self.script;
        let result = execute_event_script(engine, trigger, tx, script);

        // as long as everything is good, exit, otherwise raise an error event.
        match result {
            Ok(()) => (),
            Err(e) => {
                Event::new_builder(EventType::System(SystemEventType::Done(
                    SystemEventScope::EventHandler,
                    SystemEventResult::Errored,
                )))
                .with_metadata(Metadata::new_banner_error(&format!(
                    "arghhh. Rune script failed.... :(:\n{e}"
                )))
                .send_from(tx);
            }
        }
    }

    pub fn is_listening_for(&self, event: &Event) -> bool {
        self.listen_for_events
            .iter()
            .filter(|lfe| {
                let is_listening = lfe.matches_event(event);
                tracing::debug!(is_listening, "is listening for event: {:?}", lfe);
                is_listening
            })
            .count()
            > 0
    }

    pub fn tags(&self) -> &[Metadata] {
        self.tags.as_ref()
    }
}

fn execute_event_script(
    engine: Arc<dyn Engine + Sync + Send>,
    event: Event,
    tx: &Sender<Event>,
    script: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(&m)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    let source = Source::new("event", script);
    sources.insert(source);

    let mut diagnostics = Diagnostics::new();

    let unit = match rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
    {
        Ok(result) => result,
        Err(e) => {
            if !diagnostics.is_empty() {
                let writer = BufferWriter::stderr(ColorChoice::Never);
                let mut buffer = writer.buffer();
                diagnostics.emit(&mut buffer, &sources)?;
                let bufvec = buffer.into_inner();
                let message = std::str::from_utf8(&bufvec);
                log::error!(target: "task_log", "{}", message.unwrap());
                Event::new_builder(EventType::Log)
                    .with_log_message(message.unwrap())
                    .send_from(tx);
            }
            return Err(e.into());
        }
    };

    if !diagnostics.is_empty() {
        let writer = BufferWriter::stderr(ColorChoice::Never);
        let mut buffer = writer.buffer();
        diagnostics.emit(&mut buffer, &sources)?;
        let bufvec = buffer.into_inner();
        let message = std::str::from_utf8(&bufvec);
        log::error!(target: "task_log", "{}", message.unwrap());
        Event::new_builder(EventType::Log)
            .with_log_message(message.unwrap())
            .send_from(tx);
    }

    let wrapper = EventHandlerEngine {
        engine,
        tx: tx.clone(),
    };

    let vm = match Vm::new(runtime, Arc::new(unit)).send_execute(["main"], (wrapper, event)) {
        Ok(vm) => vm,
        Err(e) => {
            if !diagnostics.is_empty() {
                let writer = BufferWriter::stderr(ColorChoice::Never);
                let mut buffer = writer.buffer();
                diagnostics.emit(&mut buffer, &sources)?;
                let bufvec = buffer.into_inner();
                let message = std::str::from_utf8(&bufvec);
                log::error!(target: "task_log", "{}", message.unwrap());
                Event::new_builder(EventType::Log)
                    .with_log_message(message.unwrap())
                    .send_from(tx);
            }
            return Err(e.into());
        }
    };

    // spawn this off into it's own thread so we can continue to process events. This is only required because
    // we give users the capability to define their own event handlers. User defined event handlers could potentially
    // be long running and we don't want to block the event loop.
    tokio::spawn(async move {
        match vm.async_complete().await {
            Ok(out) => {
                log::trace!("Rune script completed successfully: {:?}", out);
            }
            Err(e) => {
                println!("ERROR: {e:?}");
                let writer = BufferWriter::stderr(ColorChoice::Never);
                let mut buffer = writer.buffer();
                e.emit(&mut buffer, &sources).unwrap();
                let bufvec = buffer.into_inner();
                let message = std::str::from_utf8(&bufvec);
                log::error!(target: "task_log", "{}", message.unwrap());
            }
        }
    });

    Ok(())
}

// create a module for rune to use the rune_engine::RuneEngineWrapper and everything required by Event::new including rune_engine::RuneEngineWrapper::trigger_job(pipeline, job)
fn module() -> Result<Module, ContextError> {
    let mut module = Module::default();
    module.ty::<EventHandlerEngine>()?;
    module.async_inst_fn("trigger_pipeline", EventHandlerEngine::trigger_pipeline)?;
    module.async_inst_fn("trigger_job", EventHandlerEngine::trigger_job)?;
    module.async_inst_fn("trigger_task", EventHandlerEngine::trigger_task)?;
    module.inst_fn("log_message", EventHandlerEngine::log_message)?;
    module.async_inst_fn("pipeline_complete", EventHandlerEngine::pipeline_complete)?;
    module.async_inst_fn("job_complete", EventHandlerEngine::job_complete)?;
    module.async_inst_fn(
        "execute_task_name_in_scope",
        EventHandlerEngine::execute_task_name_in_scope,
    )?;
    module.async_inst_fn("get_from_state", EventHandlerEngine::get_from_state)?;
    module.async_inst_fn("set_state_for_task", EventHandlerEngine::set_state_for_task)?;
    module.inst_fn(
        "get_pipeline_metadata_from_event",
        EventHandlerEngine::get_pipeline_metadata_from_event,
    )?;
    module.ty::<Event>()?;
    module.inst_fn("get_type", Event::get_type)?;
    module.ty::<EventType>()?;
    module.ty::<SystemEventType>()?;
    module.ty::<SystemEventScope>()?;
    module.ty::<SystemEventResult>()?;
    module.ty::<Metadata>()?;
    Ok(module)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error};

    use async_trait::async_trait;
    use banner_parser::ast::PipelineSpecification;
    use tokio::sync::broadcast;

    use crate::{ExecutionResult, Pipeline, TaskDefinition};

    use super::*;

    struct MockEngine {}

    #[async_trait]
    impl Engine for MockEngine {
        async fn confirm_requirements(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            todo!()
        }

        // Now's your chance to load state from wherever
        // it's stored.  The engine won't be considered ready until this returns
        // successfully; possibly with retries.
        async fn initialise(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            todo!()
        }

        // Not sure about the return type at all.
        async fn execute(
            &self,
            _task: &TaskDefinition,
        ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
            // self.call_count += 1;
            Ok(ExecutionResult::Success(vec![]))
        }

        async fn execute_task_name_in_scope(
            &self,
            _scope_name: &str,
            _pipeline_name: &str,
            _job_name: &str,
            _task_name: &str,
        ) -> Result<ExecutionResult, Box<dyn Error + Send + Sync>> {
            todo!()
        }

        fn get_pipelines(&self) -> Vec<&Pipeline> {
            todo!()
        }

        fn get_pipeline_specification(&self) -> &Vec<PipelineSpecification> {
            todo!()
        }

        async fn get_state_for_id(&self, _key: &str) -> Option<String> {
            todo!()
        }

        async fn set_state_for_id(
            &self,
            _key: &str,
            _value: String,
        ) -> Result<(), Box<dyn Error + Send + Sync>> {
            Ok(())
        }
    }

    // #[tokio::test]
    // async fn can_trigger_task() {
    //     let e = MockEngine { };
    //     let script = "";
    //     let result = execute_event_script(&e, script).await;
    //     assert!(result.is_ok());
    // }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn rune_script_setup() {
        let mut hm = HashMap::new();
        hm.insert("", "");

        let script = r#"
        pub async fn main (engine, event) {{
            engine.trigger_pipeline("pipeline_1").await;
            engine.trigger_job("pipeline_1", "job_1").await;
            engine.trigger_task("pipeline_1", "job_1", "task_1").await;
        }}
        "#;

        let (tx, mut rx) = broadcast::channel::<Event>(100);
        let eng = Arc::new(MockEngine {});

        let result = execute_event_script(
            eng,
            Event::new_builder(EventType::UserDefined).build(),
            &tx,
            script,
        );
        assert!(result.is_ok());
        log::debug!("{result:?}");
        let message = rx.recv().await;
        log::debug!("{message:?}");
        assert_eq!(
            message.unwrap(),
            Event::new_builder(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Pipeline
            )))
            .with_metadata(Metadata::new_banner_pipeline("pipeline_1"))
            .build()
        );
        let message = rx.recv().await;
        assert_eq!(
            message.unwrap(),
            Event::new_builder(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Job
            )))
            .with_metadata(Metadata::new_banner_pipeline("pipeline_1"))
            .with_metadata(Metadata::new_banner_job("job_1"))
            .build()
        );
        let message = rx.recv().await;
        assert_eq!(
            message.unwrap(),
            Event::new_builder(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Task
            )))
            .with_metadata(Metadata::new_banner_pipeline("pipeline_1"))
            .with_metadata(Metadata::new_banner_job("job_1"))
            .with_metadata(Metadata::new_banner_task("task_1"))
            .build()
        );
    }
}

#[cfg(test)]
mod event_handler_tests {
    use crate::ListenForEvent;
    use crate::ListenForEventType;
    use crate::ListenForSystemEventResult;
    use crate::ListenForSystemEventScope;
    use crate::ListenForSystemEventType;
    use crate::Select::*;

    use super::*;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_is_listening_for() {
        let tags = vec![];
        let listen_for_events = vec![ListenForEvent::new_builder(ListenForEventType::System(
            Only(ListenForSystemEventType::Done(
                Only(ListenForSystemEventScope::Task),
                Only(ListenForSystemEventResult::Success),
            )),
        ))
        .build()];
        let script = String::new();
        let eh = EventHandler::new(tags, listen_for_events, script);
        let listen_for = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .build();
        assert!(eh.is_listening_for(&listen_for));

        let listen_for = Event::new_builder(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Failed,
        )))
        .build();
        assert!(!eh.is_listening_for(&listen_for));
    }
}
