#[derive(Clone, Copy, Debug)]
pub enum UiLayout {
    FullScreenLogs,
    FullScreenEvents,
    FullScreenPipelines,
    MultiPanelLayout(FrameSplits),
}

#[derive(Clone, Copy, Debug)]
pub struct FrameSplits {
    pub(crate) log_and_event_frame: u8,
    pub(crate) pipeline_frame: u8,
}

impl UiLayout {
    pub fn set_full_screen_logs(&mut self) {
        *self = UiLayout::FullScreenLogs;
    }

    pub fn set_full_screen_events(&mut self) {
        *self = UiLayout::FullScreenEvents;
    }

    pub fn set_full_screen_pipeline(&mut self) {
        *self = UiLayout::FullScreenPipelines;
    }

    pub fn set_multi_panel(&mut self) {
        *self = UiLayout::MultiPanelLayout(FrameSplits {
            log_and_event_frame: 50,
            pipeline_frame: 40,
        });
    }
}
