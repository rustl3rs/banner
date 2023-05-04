use std::sync::Arc;

use rune::{
    termcolor::{BufferWriter, ColorChoice},
    ContextError, Diagnostics, Module, Source, Sources, Vm,
};
use tokio::sync::mpsc::Sender;

use crate::{
    rune_engine::RuneEngineWrapper, Engine, Event, EventType, Events, Metadata, SystemEventResult,
    SystemEventScope, SystemEventType, Tag, JOB_TAG, PIPELINE_TAG, TASK_TAG,
};

#[derive(Debug, Clone)]
pub struct EventHandler {
    tags: Vec<Tag>,
    listen_for_events: Events,
    script: String,
}

impl EventHandler {
    pub fn new(tags: Vec<Tag>, listen_for_events: Events, script: String) -> Self {
        Self {
            tags,
            listen_for_events,
            script,
        }
    }

    pub(crate) async fn execute(
        &self,
        engine: Arc<dyn Engine + Sync + Send>,
        tx: Sender<Event>,
        _trigger: Event,
    ) {
        let script: &str = &self.script;
        let result = execute_event_script(engine, tx.clone(), script).await;

        // as long as everything is good, exit, otherwise raise an error event.
        match result {
            Ok(_) => (),
            Err(e) => {
                Event::new(EventType::System(SystemEventType::Done(
                    SystemEventScope::EventHandler,
                    SystemEventResult::Errored,
                )))
                .with_metadata(Metadata::new_banner_error(&format!(
                    "arghhh. Rune script failed.... :(:\n{e}"
                )))
                .send_from(&tx)
                .await;
            }
        }
    }

    pub fn is_listening_for(&self, event: &Event) -> bool {
        self.listen_for_events
            .iter()
            .filter(|e| {
                let is_listening = *e == event;
                tracing::debug!(is_listening, "is listening for event: {:?}", e);
                is_listening
            })
            .count()
            > 0
    }

    pub fn tags(&self) -> &[Metadata] {
        self.tags.as_ref()
    }
}

fn get_target_pipeline_and_task(tags: &[Metadata]) -> (&str, &str, &str) {
    let pipeline_name = tags.iter().find(|tag| tag.key() == PIPELINE_TAG);
    let job_name = tags.iter().find(|tag| tag.key() == JOB_TAG);
    let task_name = tags.iter().find(|tag| tag.key() == TASK_TAG);
    let pipeline_name = match pipeline_name {
        Some(pt) => pt.value(),
        None => "_",
    };
    let job_name = match job_name {
        Some(jt) => jt.value(),
        None => "_",
    };
    let task_name = match task_name {
        Some(tt) => tt.value(),
        None => "_",
    };
    (pipeline_name, job_name, task_name)
}

async fn execute_event_script(
    engine: Arc<dyn Engine + Sync + Send>,
    tx: Sender<Event>,
    script: &str,
) -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(&m)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new("event", script));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let writer = BufferWriter::stderr(ColorChoice::Never);
        let mut buffer = writer.buffer();
        diagnostics.emit(&mut buffer, &sources)?;
        let bufvec = buffer.into_inner();
        let message = std::str::from_utf8(&bufvec);
        Event::new(EventType::Log)
            .with_log_message(&message.unwrap())
            .send_from(&tx)
            .await;
    }

    let wrapper = RuneEngineWrapper { engine, tx };
    let vm = Vm::new(runtime, Arc::new(unit.unwrap())).send_execute(&["main"], (wrapper,))?;

    // spawn this off into it's own thread so we can continue to process events. This is only required because
    // we give users the capability to define their own event handlers. User defined event handlers could potentially
    // be long running and we don't want to block the event loop.
    let scr = script.to_owned();
    tokio::spawn(async move {
        match vm.async_complete().await {
            Ok(_) => {}
            Err(e) => {
                log::error!(target: "task_log", "There was an error running rune script: {e}\nfor: {scr}")
            }
        }
    });

    Ok(())
}

// create a module for rune to use the rune_engine::RuneEngineWrapper and everything required by Event::new including rune_engine::RuneEngineWrapper::trigger_job(pipeline, job)
fn module() -> Result<Module, ContextError> {
    let mut module = Module::default();
    module.ty::<RuneEngineWrapper>()?;
    module.async_inst_fn("trigger_pipeline", RuneEngineWrapper::trigger_pipeline)?;
    module.async_inst_fn("trigger_job", RuneEngineWrapper::trigger_job)?;
    module.async_inst_fn("trigger_task", RuneEngineWrapper::trigger_task)?;
    module.async_inst_fn("log_message", RuneEngineWrapper::log_message)?;
    module.async_inst_fn("job_success", RuneEngineWrapper::job_success)?;
    module.async_inst_fn("job_fail", RuneEngineWrapper::job_fail)?;
    module.async_inst_fn("task_success", RuneEngineWrapper::task_success)?;
    module.async_inst_fn("task_fail", RuneEngineWrapper::task_fail)?;
    module.async_inst_fn("pipeline_success", RuneEngineWrapper::pipeline_success)?;
    module.async_inst_fn("pipeline_fail", RuneEngineWrapper::pipeline_fail)?;
    module.async_inst_fn(
        "execute_task_name_in_scope",
        RuneEngineWrapper::execute_task_name_in_scope,
    )?;
    Ok(module)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use async_trait::async_trait;
    use tokio::sync::mpsc;

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
    }

    // #[tokio::test]
    // async fn can_trigger_task() {
    //     let e = MockEngine { };
    //     let script = "";
    //     let result = execute_event_script(&e, script).await;
    //     assert!(result.is_ok());
    // }

    #[tokio::test]
    async fn rune_script_setup() {
        let script = r###"
        pub async fn main (engine) {{
            engine.trigger_pipeline("pipeline_1").await;
            engine.trigger_job("pipeline_1", "job_1").await;
            engine.trigger_task("pipeline_1", "job_1", "task_1").await;
        }}
        "###;

        let (tx, mut rx) = mpsc::channel::<Event>(100);
        let eng = Arc::new(MockEngine {});

        let result = execute_event_script(eng, tx, script).await;
        assert!(result.is_ok());
        let message = rx.recv().await;
        assert_eq!(
            message.unwrap(),
            Event::new(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Pipeline
            )))
            .with_metadata(Metadata::new_banner_pipeline("pipeline_1"))
            .build()
        );
        let message = rx.recv().await;
        assert_eq!(
            message.unwrap(),
            Event::new(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Job
            )))
            .with_metadata(Metadata::new_banner_pipeline("pipeline_1"))
            .with_metadata(Metadata::new_banner_job("job_1"))
            .build()
        );
        let message = rx.recv().await;
        assert_eq!(
            message.unwrap(),
            Event::new(EventType::System(SystemEventType::Trigger(
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
    use super::*;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_is_listening_for() {
        let tags = vec![];
        let listen_for_events = vec![Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .build()];
        let script = String::from("");
        let eh = EventHandler::new(tags, listen_for_events, script);
        let listen_for = Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Success,
        )))
        .build();
        assert!(eh.is_listening_for(&listen_for));

        let listen_for = Event::new(EventType::System(SystemEventType::Done(
            SystemEventScope::Task,
            SystemEventResult::Failed,
        )))
        .build();
        assert!(!eh.is_listening_for(&listen_for));
    }
}
