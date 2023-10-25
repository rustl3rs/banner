// Need to ensure that events, and listen_for_events are re-exported and in sync.
// I don't like specifically having two different ways of representing the same thing, but I feel in this case it's
// the best thing, since listen_for_events are a more for searching and filtering, while events are the things being
// searched against.  Events can never have a Select::Any, while listen_for_events can.

use banner_engine::{ListenForSystemEventResult, SystemEventResult};
use strum::IntoEnumIterator;

#[test]
pub fn check_system_result() {
    let ser = SystemEventResult::iter().collect::<Vec<_>>();
    let lf_ser = ListenForSystemEventResult::iter().collect::<Vec<_>>();
    for lf_event in lf_ser {
        let lf_event = match lf_event {
            ListenForSystemEventResult::Success => SystemEventResult::Success,
            ListenForSystemEventResult::Failed => SystemEventResult::Failed,
            ListenForSystemEventResult::Aborted => SystemEventResult::Aborted,
            ListenForSystemEventResult::Errored => SystemEventResult::Errored,
        };
        assert!(ser.iter().any(|event| *event == lf_event));
    }
}

pub fn check_listen_for_system_result() {
    let ser = SystemEventResult::iter().collect::<Vec<_>>();
    let lf_ser = ListenForSystemEventResult::iter().collect::<Vec<_>>();
    for event in ser {
        let event = match event {
            SystemEventResult::Success => ListenForSystemEventResult::Success,
            SystemEventResult::Failed => ListenForSystemEventResult::Failed,
            SystemEventResult::Aborted => ListenForSystemEventResult::Aborted,
            SystemEventResult::Errored => ListenForSystemEventResult::Errored,
        };
        assert!(lf_ser.iter().any(|lf_event| *lf_event == event));
    }
}
