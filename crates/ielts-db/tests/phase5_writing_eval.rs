//! Phase 5: writing draft, evaluation state machine, checkpoints, idempotency.

use ielts_domain::domain::{Activity, AttemptMode, EvaluationStage, EvaluationStatus};
use ielts_domain::dto::{SaveDraftCommand, SubmitAttemptCommand, WritingEvaluationV4};
use tempfile::tempdir;

use ielts_db::{
    finish_evaluation, get_history_detail, get_writing_draft, list_events,
    load_evaluation_for_attempt, migrate, open_connection, prepare_evaluation,
    recover_interrupted_sessions, request_cancel, save_writing_draft, start_evaluation,
    submit_writing_attempt, DbOpenOptions, DeterministicProvider, ProviderError,
    StartEvaluationCommand, WritingProvider,
};
use ielts_domain::domain::WritingTaskType;
use ielts_domain::dto::{WritingFeedbackV4, WritingScoreV4};

fn open_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempdir().unwrap();
    let mut conn = open_connection(&DbOpenOptions::create(dir.path().join("v2.db"))).unwrap();
    migrate(&mut conn).unwrap();
    (dir, conn)
}

fn draft_cmd(id: &str, text: &str, key: &str) -> SaveDraftCommand {
    SaveDraftCommand {
        attempt_id: id.into(),
        activity: Activity::Writing,
        mode: AttemptMode::Bank,
        asset_id: None,
        content_text: Some(text.into()),
        prompt_snapshot: Some("Discuss both views.".into()),
        idempotency_key: key.into(),
    }
}

#[test]
fn draft_and_idempotent_submit() {
    let (_dir, conn) = open_db();
    let essay = "Practical skills matter in modern education. ".repeat(40);
    save_writing_draft(&conn, &draft_cmd("a1", &essay, "draft-1")).unwrap();
    let d = get_writing_draft(&conn, "a1").unwrap().unwrap();
    assert!(d.word_count > 10);

    let submitted = submit_writing_attempt(
        &conn,
        &SubmitAttemptCommand {
            attempt_id: "a1".into(),
            idempotency_key: "submit-1".into(),
        },
    )
    .unwrap();
    assert_eq!(
        format!("{:?}", submitted.status).to_ascii_lowercase(),
        "submitted"
    );

    let again = submit_writing_attempt(
        &conn,
        &SubmitAttemptCommand {
            attempt_id: "a1".into(),
            idempotency_key: "submit-1".into(),
        },
    )
    .unwrap();
    assert_eq!(again.id, submitted.id);
}

#[test]
fn evaluation_runs_stages_and_persists_checkpoints() {
    let (_dir, conn) = open_db();
    let essay = "Universities should balance theory and practice. ".repeat(50);
    save_writing_draft(&conn, &draft_cmd("a2", &essay, "d2")).unwrap();
    submit_writing_attempt(
        &conn,
        &SubmitAttemptCommand {
            attempt_id: "a2".into(),
            idempotency_key: "s2".into(),
        },
    )
    .unwrap();

    let provider = DeterministicProvider;
    let result = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a2".into(),
            idempotency_key: "eval-1".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        &provider,
    )
    .unwrap();

    assert!(matches!(
        result.evaluation.status,
        EvaluationStatus::Completed | EvaluationStatus::Degraded
    ));
    assert!(result.evaluation.score.is_some());
    assert!(!result.events.is_empty());
    assert!(result.events.iter().any(|e| e.event_type == "score"));

    // checkpoints exist
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM evaluation_checkpoints WHERE evaluation_id = ?1",
            rusqlite::params![result.session.evaluation_id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(n >= 2);

    // reload from DB only
    let loaded = load_evaluation_for_attempt(&conn, "a2").unwrap().unwrap();
    assert_eq!(
        loaded.score.as_ref().unwrap().overall,
        result.evaluation.score.as_ref().unwrap().overall
    );

    // idempotent start
    let again = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a2".into(),
            idempotency_key: "eval-1".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        &provider,
    )
    .unwrap();
    assert_eq!(again.session.evaluation_id, result.session.evaluation_id);
}

struct FailReviewProvider;
impl WritingProvider for FailReviewProvider {
    fn id(&self) -> &str {
        "fail-review"
    }
    fn model(&self) -> &str {
        "x"
    }
    fn score(
        &self,
        essay: &str,
        prompt: Option<&str>,
        task_type: Option<WritingTaskType>,
    ) -> Result<WritingScoreV4, ProviderError> {
        DeterministicProvider.score(essay, prompt, task_type)
    }
    fn review(
        &self,
        _essay: &str,
        _score: &WritingScoreV4,
    ) -> Result<WritingFeedbackV4, ProviderError> {
        Err(ProviderError {
            message: "review json invalid".into(),
            retryable: true,
        })
    }
}

#[test]
fn review_failure_degrades_but_keeps_score() {
    let (_dir, conn) = open_db();
    let essay = "Balanced education is important. ".repeat(40);
    save_writing_draft(&conn, &draft_cmd("a3", &essay, "d3")).unwrap();
    let result = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a3".into(),
            idempotency_key: "eval-deg".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        &FailReviewProvider,
    )
    .unwrap();
    assert_eq!(result.evaluation.status, EvaluationStatus::Degraded);
    assert!(result.evaluation.score.is_some());
    assert!(result.evaluation.degradation.is_some());
}

#[test]
fn cancel_keeps_draft_inputs() {
    let (_dir, conn) = open_db();
    let essay = "Keep my essay text safe. ".repeat(30);
    save_writing_draft(&conn, &draft_cmd("a4", &essay, "d4")).unwrap();
    // Pre-create a session-like path: start with a provider that we cancel mid-flight is hard
    // in sync mode; instead mark cancel before stages via manual session is overkill.
    // Verify request_cancel API + draft retained after interrupted recovery.
    let provider = DeterministicProvider;
    let result = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a4".into(),
            idempotency_key: "eval-c".into(),
            task_type: None,
            retry_of: None,
        },
        &provider,
    )
    .unwrap();
    // cancel after complete is no-op; draft still present
    let _ = request_cancel(&conn, &result.session.evaluation_id).unwrap();
    let draft = get_writing_draft(&conn, "a4").unwrap().unwrap();
    assert!(draft.content_text.contains("Keep my essay"));
}

#[test]
fn retry_creates_lineage() {
    let (_dir, conn) = open_db();
    let essay = "Retry lineage should preserve history. ".repeat(40);
    save_writing_draft(&conn, &draft_cmd("a5", &essay, "d5")).unwrap();
    let provider = DeterministicProvider;
    let first = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a5".into(),
            idempotency_key: "eval-r1".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        &provider,
    )
    .unwrap();
    let second = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a5".into(),
            idempotency_key: "eval-r2".into(),
            task_type: Some("task2".into()),
            retry_of: Some(first.session.evaluation_id.clone()),
        },
        &provider,
    )
    .unwrap();
    assert_ne!(first.session.evaluation_id, second.session.evaluation_id);
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM evaluation_lineage WHERE evaluation_id = ?1",
            rusqlite::params![second.session.evaluation_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn recover_marks_running_sessions_interrupted() {
    let (_dir, conn) = open_db();
    let essay = "Crash recovery test content. ".repeat(30);
    save_writing_draft(&conn, &draft_cmd("a6", &essay, "d6")).unwrap();
    // Insert synthetic running session
    conn.execute(
        "INSERT INTO attempts (id, activity, mode, status, started_at, duration_ms, schema_version, created_at, updated_at)
         VALUES ('a6', 'writing', 'bank', 'reviewing', '2025-01-01T00:00:00Z', 0, 1, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')
         ON CONFLICT(id) DO NOTHING",
        [],
    )
    .ok();
    conn.execute(
        "INSERT INTO writing_evaluations (id, attempt_id, status, stage, rubric_version, prompt_version, updated_at, started_at)
         VALUES ('e-run', 'a6', 'running', 'scoring', 'r', 'p', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO evaluation_sessions (
            id, attempt_id, evaluation_id, status, stage, revision, sequence, cancel_requested, started_at, updated_at
         ) VALUES ('s-run', 'a6', 'e-run', 'running', 'scoring', 1, 1, 0, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    let n = recover_interrupted_sessions(&conn).unwrap();
    assert!(n >= 1);
    let status: String = conn
        .query_row(
            "SELECT status FROM evaluation_sessions WHERE id = 's-run'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "interrupted");
    // draft still there
    assert!(get_writing_draft(&conn, "a6").unwrap().is_some());
}

#[test]
fn recovery_reconciles_result_event_and_attempt_projection() {
    let (_dir, conn) = open_db();
    let essay = "Recovery must leave one canonical writing state. ".repeat(30);
    save_writing_draft(&conn, &draft_cmd("a-reconcile", &essay, "d-reconcile")).unwrap();
    let prepared = prepare_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-reconcile".into(),
            idempotency_key: "eval-reconcile".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        "openai-compatible",
        "test-model",
    )
    .unwrap();

    // Simulate a process death after the durable session moved to scoring but
    // before its JSON snapshot was updated for that stage.
    conn.execute(
        "UPDATE evaluation_sessions SET status = 'running', stage = 'scoring', revision = 4
         WHERE id = ?1",
        rusqlite::params![prepared.session_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE writing_evaluations SET status = 'running', stage = 'scoring'
         WHERE id = ?1",
        rusqlite::params![prepared.evaluation_id],
    )
    .unwrap();

    assert_eq!(recover_interrupted_sessions(&conn).unwrap(), 1);
    assert_eq!(recover_interrupted_sessions(&conn).unwrap(), 0);

    let canonical = load_evaluation_for_attempt(&conn, "a-reconcile")
        .unwrap()
        .unwrap();
    assert_eq!(canonical.id, prepared.evaluation_id);
    assert_eq!(canonical.status, EvaluationStatus::Interrupted);
    assert_eq!(canonical.stage, EvaluationStage::Scoring);

    let result_json: String = conn
        .query_row(
            "SELECT result_json FROM writing_evaluations WHERE id = ?1",
            rusqlite::params![prepared.evaluation_id],
            |row| row.get(0),
        )
        .unwrap();
    let persisted: WritingEvaluationV4 = serde_json::from_str(&result_json).unwrap();
    assert_eq!(persisted.status, EvaluationStatus::Interrupted);
    assert_eq!(persisted.stage, EvaluationStage::Scoring);

    let events = list_events(&conn, &prepared.evaluation_id, 0).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "interrupted")
            .count(),
        1
    );
    assert!(events.iter().any(|event| {
        event.event_type == "interrupted"
            && event.payload["reason"] == "process_restarted"
            && event.payload["keptInputs"] == true
    }));

    let (attempt_status, attempt_completed_at): (String, Option<String>) = conn
        .query_row(
            "SELECT status, completed_at FROM attempts WHERE id = 'a-reconcile'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(attempt_status, "interrupted");
    assert!(attempt_completed_at.is_some());
    assert_eq!(
        get_writing_draft(&conn, "a-reconcile")
            .unwrap()
            .unwrap()
            .content_text,
        essay
    );
}

#[test]
fn retry_keeps_latest_result_when_an_older_provider_call_finishes_late() {
    let (_dir, conn) = open_db();
    let essay = "Latest retry must win over an old provider response. ".repeat(40);
    save_writing_draft(
        &conn,
        &draft_cmd("a-latest-retry", &essay, "d-latest-retry"),
    )
    .unwrap();

    let first = prepare_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-latest-retry".into(),
            idempotency_key: "eval-latest-first".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        "openai-compatible",
        "test-model",
    )
    .unwrap();
    let second = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-latest-retry".into(),
            idempotency_key: "eval-latest-second".into(),
            task_type: Some("task2".into()),
            retry_of: Some(first.evaluation_id.clone()),
        },
        &DeterministicProvider,
    )
    .unwrap();
    let second_score = second.evaluation.score.as_ref().unwrap().overall;

    // The stale request finishes after the retry. Its update timestamp is now
    // newer, but it is not the newest evaluation in the retry lineage.
    let late_score = WritingScoreV4 {
        overall: 5.0,
        task_response: 5.0,
        coherence: 5.0,
        lexical: 5.0,
        grammar: 5.0,
    };
    let late_feedback = WritingFeedbackV4 {
        overall: Some("stale response".into()),
        plan: vec![],
        paragraphs: vec![],
        sentences: vec![],
        rewrites: vec![],
    };
    finish_evaluation(&conn, &first, Ok(late_score), Some(late_feedback), None).unwrap();

    let latest = load_evaluation_for_attempt(&conn, "a-latest-retry")
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, second.session.evaluation_id);
    assert_eq!(latest.score.as_ref().unwrap().overall, second_score);

    let history = get_history_detail(&conn, "a-latest-retry").unwrap();
    assert_eq!(history.evaluation.unwrap().id, second.session.evaluation_id);
    let (attempt_status, attempt_score): (String, Option<f64>) = conn
        .query_row(
            "SELECT status, score_value FROM attempts WHERE id = 'a-latest-retry'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(attempt_status, "completed");
    assert_eq!(attempt_score, Some(second_score));
}

#[test]
fn recovery_of_an_old_session_does_not_override_a_completed_retry() {
    let (_dir, conn) = open_db();
    let essay = "Recovering an old session must not hide the newer retry. ".repeat(35);
    save_writing_draft(
        &conn,
        &draft_cmd("a-recovery-retry", &essay, "d-recovery-retry"),
    )
    .unwrap();

    let first = prepare_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-recovery-retry".into(),
            idempotency_key: "eval-recovery-old".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        "openai-compatible",
        "test-model",
    )
    .unwrap();
    let second = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-recovery-retry".into(),
            idempotency_key: "eval-recovery-new".into(),
            task_type: Some("task2".into()),
            retry_of: Some(first.evaluation_id.clone()),
        },
        &DeterministicProvider,
    )
    .unwrap();

    assert_eq!(recover_interrupted_sessions(&conn).unwrap(), 1);
    let latest = load_evaluation_for_attempt(&conn, "a-recovery-retry")
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, second.session.evaluation_id);
    assert!(matches!(
        latest.status,
        EvaluationStatus::Completed | EvaluationStatus::Degraded
    ));
    let attempt_status: String = conn
        .query_row(
            "SELECT status FROM attempts WHERE id = 'a-recovery-retry'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attempt_status, "completed");
}

#[test]
fn crash_during_unlocked_provider_call_recovers_without_losing_input() {
    let (_dir, conn) = open_db();
    let essay = "The provider call must not own the database lock. ".repeat(30);
    save_writing_draft(&conn, &draft_cmd("a-network", &essay, "d-network")).unwrap();
    let prepared = prepare_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-network".into(),
            idempotency_key: "eval-network".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        "openai-compatible",
        "test-model",
    )
    .unwrap();

    assert!(prepared.existing.is_none());
    assert_eq!(prepared.essay, essay);
    assert_eq!(recover_interrupted_sessions(&conn).unwrap(), 1);
    let status: String = conn
        .query_row(
            "SELECT status FROM evaluation_sessions WHERE id = ?1",
            rusqlite::params![prepared.session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "interrupted");
    assert_eq!(
        get_writing_draft(&conn, "a-network")
            .unwrap()
            .unwrap()
            .content_text,
        essay
    );
}

#[test]
fn prepare_returns_a_durable_handle_before_provider_io() {
    let (_dir, conn) = open_db();
    let essay = "A durable handle must exist before the network request. ".repeat(25);
    save_writing_draft(&conn, &draft_cmd("a-handle", &essay, "d-handle")).unwrap();

    let prepared = prepare_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a-handle".into(),
            idempotency_key: "eval-handle".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        "openai-compatible",
        "test-model",
    )
    .unwrap();

    assert_eq!(prepared.handle.attempt_id, "a-handle");
    assert_eq!(prepared.handle.session_id, prepared.session_id);
    assert_eq!(prepared.handle.evaluation_id, prepared.evaluation_id);
    assert_eq!(prepared.handle.sequence, 1);
    let snapshot = load_evaluation_for_attempt(&conn, "a-handle")
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.id, prepared.evaluation_id);
    assert_eq!(snapshot.status, EvaluationStatus::Queued);
    let events = list_events(&conn, &prepared.evaluation_id, 0).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "stage");
}

#[test]
fn cancel_is_immediately_persisted_and_idempotent_start_cannot_overwrite_it() {
    let (_dir, conn) = open_db();
    let essay = "Cancellation must not wait for an HTTP timeout. ".repeat(25);
    save_writing_draft(&conn, &draft_cmd("a-cancel-now", &essay, "d-cancel-now")).unwrap();
    let command = StartEvaluationCommand {
        attempt_id: "a-cancel-now".into(),
        idempotency_key: "eval-cancel-now".into(),
        task_type: Some("task2".into()),
        retry_of: None,
    };
    let prepared = prepare_evaluation(&conn, &command, "openai-compatible", "test-model").unwrap();

    assert!(request_cancel(&conn, &prepared.evaluation_id).unwrap());
    assert!(!request_cancel(&conn, &prepared.evaluation_id).unwrap());
    let snapshot = load_evaluation_for_attempt(&conn, "a-cancel-now")
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.status, EvaluationStatus::Interrupted);
    assert!(list_events(&conn, &prepared.evaluation_id, 0)
        .unwrap()
        .iter()
        .any(|event| event.event_type == "cancelled"));

    let result = start_evaluation(&conn, &command, &DeterministicProvider).unwrap();
    assert_eq!(result.evaluation.status, EvaluationStatus::Interrupted);
    assert_eq!(
        get_writing_draft(&conn, "a-cancel-now")
            .unwrap()
            .unwrap()
            .content_text,
        essay
    );
}

#[test]
fn events_have_monotonic_sequence() {
    let (_dir, conn) = open_db();
    let essay = "Sequence events for channel consumers. ".repeat(40);
    save_writing_draft(&conn, &draft_cmd("a7", &essay, "d7")).unwrap();
    let result = start_evaluation(
        &conn,
        &StartEvaluationCommand {
            attempt_id: "a7".into(),
            idempotency_key: "eval-seq".into(),
            task_type: Some("task2".into()),
            retry_of: None,
        },
        &DeterministicProvider,
    )
    .unwrap();
    let events = list_events(&conn, &result.session.evaluation_id, 0).unwrap();
    let mut last = 0u32;
    for e in &events {
        assert!(e.sequence > last);
        last = e.sequence;
        assert!(e.revision >= 1 || e.event_type == "completed");
    }
}
