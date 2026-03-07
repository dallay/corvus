use async_trait::async_trait;
use corvus::conductor::classifier::{
    ChainedClassifier, Confidence, RuleBasedClassifier, StaticLlmClassifier, TaskClassifier,
};
use corvus::conductor::planner::{PlanModel, PlannedStep, Planner, PlannerConfigView, TaskPlan};
use corvus::conductor::{StepId, TaskDomain, TaskOrigin, TaskPriority, TaskRequest};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::time::{sleep, Duration};

fn request(description: &str) -> TaskRequest {
    TaskRequest {
        description: description.to_string(),
        origin: TaskOrigin::Cli {
            working_dir: "/tmp/repo".to_string(),
        },
        priority: TaskPriority::Normal,
        context: None,
        workspace_hint: None,
        timeout_ms: None,
        tags: Vec::new(),
        domain: TaskDomain::Composite,
    }
}

struct MockPlanModel {
    calls: Arc<AtomicUsize>,
    delay_ms: u64,
    response: MockResponse,
    prompts: Arc<Mutex<Vec<String>>>,
}

enum MockResponse {
    Ok(TaskPlan),
}

#[async_trait]
impl PlanModel for MockPlanModel {
    async fn decompose(&self, _request: &TaskRequest, prompt: &str) -> anyhow::Result<TaskPlan> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.prompts
            .lock()
            .expect("prompts mutex poisoned")
            .push(prompt.to_string());
        if self.delay_ms > 0 {
            sleep(Duration::from_millis(self.delay_ms)).await;
        }
        match &self.response {
            MockResponse::Ok(plan) => Ok(plan.clone()),
        }
    }
}

fn step(id: &str, domain: TaskDomain, deps: Vec<&str>) -> PlannedStep {
    PlannedStep {
        id: StepId::new(id).expect("valid step id"),
        domain,
        description: format!("step {id}"),
        depends_on: deps
            .into_iter()
            .map(|dep| StepId::new(dep).expect("valid dep"))
            .collect(),
    }
}

#[tokio::test]
async fn rule_based_classifier_fast_path_and_confidence() {
    let rule = RuleBasedClassifier;

    let coding = rule
        .classify("fix failing test in scheduler")
        .await
        .unwrap();
    assert_eq!(coding.domain, TaskDomain::Coding);
    assert_eq!(coding.confidence, Confidence::High);

    let ambiguous = rule.classify("please handle this somehow").await.unwrap();
    assert_eq!(ambiguous.domain, TaskDomain::Composite);
    assert_eq!(ambiguous.confidence, Confidence::Low);
}

#[tokio::test]
async fn fast_path_avoids_slow_model_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let planner = Planner::new(
        PlannerConfigView {
            max_planning_time_ms: 100,
            fast_path_budget_ms: 10,
            prompt_path: None,
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(MockPlanModel {
            calls: Arc::clone(&calls),
            delay_ms: 0,
            response: MockResponse::Ok(TaskPlan {
                steps: vec![step("x", TaskDomain::Coding, vec![])],
            }),
            prompts: Arc::clone(&prompts),
        }),
    );

    let start = Instant::now();
    let plan = planner.plan(&request("fix typo in README")).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(elapsed < Duration::from_millis(10));
}

#[tokio::test]
async fn slow_path_is_timeout_bounded() {
    let planner = Planner::new(
        PlannerConfigView {
            max_planning_time_ms: 10,
            fast_path_budget_ms: 10,
            prompt_path: None,
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(MockPlanModel {
            calls: Arc::new(AtomicUsize::new(0)),
            delay_ms: 50,
            response: MockResponse::Ok(TaskPlan {
                steps: vec![step("a", TaskDomain::Coding, vec![])],
            }),
            prompts: Arc::new(Mutex::new(Vec::new())),
        }),
    );

    let error = planner
        .plan(&request("orchestrate an unknown mixed workflow"))
        .await
        .expect_err("slow path should timeout");
    assert!(error.to_string().contains("planning timed out"));
}

#[tokio::test]
async fn malformed_plan_is_rejected() {
    let planner = Planner::new(
        PlannerConfigView {
            max_planning_time_ms: 100,
            fast_path_budget_ms: 10,
            prompt_path: None,
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(MockPlanModel {
            calls: Arc::new(AtomicUsize::new(0)),
            delay_ms: 0,
            response: MockResponse::Ok(TaskPlan {
                steps: vec![step("bad", TaskDomain::Composite, vec![])],
            }),
            prompts: Arc::new(Mutex::new(Vec::new())),
        }),
    );

    let error = planner
        .plan(&request("orchestrate an unknown mixed workflow"))
        .await
        .expect_err("composite steps should be rejected");
    assert!(error.to_string().contains("malformed plan"));
}

#[tokio::test]
async fn dag_cycle_detection_rejects_invalid_plan() {
    let planner = Planner::new(
        PlannerConfigView {
            max_planning_time_ms: 100,
            fast_path_budget_ms: 10,
            prompt_path: None,
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(MockPlanModel {
            calls: Arc::new(AtomicUsize::new(0)),
            delay_ms: 0,
            response: MockResponse::Ok(TaskPlan {
                steps: vec![
                    step("a", TaskDomain::Coding, vec!["b"]),
                    step("b", TaskDomain::Research, vec!["a"]),
                ],
            }),
            prompts: Arc::new(Mutex::new(Vec::new())),
        }),
    );

    let error = planner
        .plan(&request("orchestrate an unknown mixed workflow"))
        .await
        .expect_err("cycle should be rejected");
    let rendered = error.to_string();
    assert!(
        rendered.to_ascii_lowercase().contains("cycle"),
        "expected cycle error, got: {rendered}",
    );
}

#[tokio::test]
async fn planner_loads_conductor_prompt_for_slow_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let prompt_path = temp.path().join("CONDUCTOR.md");
    std::fs::write(&prompt_path, "# Prompt\nSystem instructions").expect("write prompt file");

    let prompts = Arc::new(Mutex::new(Vec::new()));
    let planner = Planner::new(
        PlannerConfigView {
            max_planning_time_ms: 100,
            fast_path_budget_ms: 10,
            prompt_path: Some(prompt_path),
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(MockPlanModel {
            calls: Arc::new(AtomicUsize::new(0)),
            delay_ms: 0,
            response: MockResponse::Ok(TaskPlan {
                steps: vec![step("p1", TaskDomain::Coding, vec![])],
            }),
            prompts: Arc::clone(&prompts),
        }),
    );

    let _ = planner
        .plan(&request("orchestrate an unknown mixed workflow"))
        .await
        .expect("plan should succeed");

    let captured = prompts.lock().expect("prompts mutex poisoned");
    assert_eq!(captured.len(), 1);
    assert!(captured[0].contains("System instructions"));
}
