use corvus::observability::{redact_observer_payload, Observer, ObserverEvent, ObserverMetric};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CapturingObserver {
    events: Mutex<Vec<ObserverEvent>>,
    metrics: Mutex<Vec<ObserverMetric>>,
}

impl Observer for CapturingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events
            .lock()
            .expect("events mutex poisoned")
            .push(event.clone());
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        self.metrics
            .lock()
            .expect("metrics mutex poisoned")
            .push(metric.clone());
    }

    fn name(&self) -> &str {
        "capturing"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn conductor_event_sequence_and_metric_increments_are_observable() {
    let observer: Arc<dyn Observer> = Arc::new(CapturingObserver::default());

    corvus::observability::record_conductor_lifecycle(
        observer.as_ref(),
        "task-1",
        "step-1",
        "queued",
        3,
    );
    corvus::observability::record_conductor_lifecycle(
        observer.as_ref(),
        "task-1",
        "step-1",
        "running",
        2,
    );
    corvus::observability::record_conductor_lifecycle(
        observer.as_ref(),
        "task-1",
        "step-1",
        "completed",
        1,
    );

    let concrete = observer
        .as_any()
        .downcast_ref::<CapturingObserver>()
        .expect("capturing observer");
    let events = concrete.events.lock().expect("events mutex poisoned");
    let metrics = concrete.metrics.lock().expect("metrics mutex poisoned");

    assert_eq!(events.len(), 3);
    assert_eq!(metrics.len(), 3);
    assert!(matches!(
        events[0],
        ObserverEvent::ConductorStepLifecycle { ref status, .. } if status == "queued"
    ));
    assert!(matches!(
        events[1],
        ObserverEvent::ConductorStepLifecycle { ref status, .. } if status == "running"
    ));
    assert!(matches!(
        events[2],
        ObserverEvent::ConductorStepLifecycle { ref status, .. } if status == "completed"
    ));
}

#[test]
fn sensitive_observability_payloads_are_redacted() {
    let secret = redact_observer_payload("api_key=super-secret");
    assert_eq!(secret, "***REDACTED***");

    let clean = redact_observer_payload("scheduler queue depth high");
    assert_eq!(clean, "scheduler queue depth high");
}
