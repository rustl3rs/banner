use std::sync::Arc;

use rune::{
    termcolor::{BufferWriter, ColorChoice, StandardStream},
    ContextError, Diagnostics, Module, Source, Sources, Vm,
};
use tokio::sync::mpsc::Sender;

use crate::{
    pipeline, Engine, Event, EventType, Events, Metadata, SystemEventResult, SystemEventScope,
    SystemEventType, Tag, JOB_TAG, PIPELINE_TAG, TASK_TAG,
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
        trigger: Event,
    ) {
        let (pipeline_name, job_name, task_name) = get_target_pipeline_and_task(trigger.metadata());
        Event::new(EventType::System(SystemEventType::Starting(
            SystemEventScope::EventHandler,
        )))
        .with_pipeline_name(pipeline_name)
        .with_job_name(job_name)
        .with_task_name(task_name)
        .with_event(&trigger)
        .send_from(&tx)
        .await;

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
            .filter(|e| *e == event)
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
    let m = module(engine, tx.clone())?;

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

    let mut vm = Vm::new(runtime, Arc::new(unit.unwrap()));
    let output = vm.call(["main"], ())?;
    // let output = Foo::from_value(output)?;
    Ok(())
}

fn module(
    engine: Arc<dyn Engine + Sync + Send>,
    tx: Sender<Event>,
) -> Result<Module, ContextError> {
    let e = rune_engine::RuneEngineWrapper { engine, tx };
    let mut m = Module::with_item(["module"]);
    // m.ty::<Engine>()?;
    // m.inst_fn(
    //     "execute_task_name_in_scope",
    //     Engine::execute_task_name_in_scope,
    // )?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use async_trait::async_trait;
    use tokio::sync::mpsc;

    use crate::{ExecutionResult, Pipeline, TaskDefinition};

    use super::*;

    struct MockEngine {
        pub call_count: u8,
    }

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
    //     let e = MockEngine { call_count: 0 };
    //     let script = "";
    //     let result = execute_event_script(&e, script).await;
    //     assert!(result.is_ok());
    // }

    #[tokio::test]
    async fn rune_script_setup() {
        let script = r###"
        pub fn main () {{
            trigger_job("pipeline_1", "job_1")
        }}
        "###;

        let (tx, rx) = mpsc::channel::<Event>(100);
        let e = rune_engine::RuneEngineWrapper {
            engine: Arc::new(MockEngine { call_count: 0 }),
            tx,
        };

        let result = execute_event_script(&e, tx, script).await;
        assert!(result.is_ok());
    }
}

mod rune_engine {
    use std::sync::Arc;

    use tokio::sync::mpsc::Sender;

    use crate::{Engine, Event, EventType, SystemEventScope, SystemEventType};

    pub struct RuneEngineWrapper {
        pub engine: Arc<dyn Engine + Send + Sync>,
        pub tx: Sender<Event>,
    }

    impl RuneEngineWrapper {
        pub fn trigger_job(&self, pipeline: &str, job: &str) {
            Event::new(EventType::System(SystemEventType::Trigger(
                SystemEventScope::Job,
            )))
            .with_pipeline_name(pipeline)
            .with_job_name(job)
            .blocking_send_from(&self.tx);
        }
    }
}
