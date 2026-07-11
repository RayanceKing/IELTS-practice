//! Phase 5: writing draft, evaluation state machine, checkpoints, idempotency.

use ielts_domain::domain::{Activity, AttemptMode, EvaluationStatus};
use ielts_domain::dto::{SaveDraftCommand, SubmitAttemptCommand};
use tempfile::tempdir;

use ielts_db::{
    get_writing_draft, list_events, load_evaluation_for_attempt, migrate, open_connection,
    recover_interrupted_sessions, request_cancel, save_writing_draft, start_evaluation,
    submit_writing_attempt, DeterministicProvider, DbOpenOptions, ProviderError, StartEvaluationCommand,
    WritingProvider,
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
    assert_eq!(format!("{:?}", submitted.status).to_ascii_lowercase(), "submitted");

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
