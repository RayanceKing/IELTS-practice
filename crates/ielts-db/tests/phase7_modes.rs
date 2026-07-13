//! Phase 7: suite / endless / memorize / timer state machines.

use serde_json::json;
use tempfile::tempdir;

use ielts_db::{
    advance_endless, create_endless_session, create_memorize_session, create_suite_session,
    finish_memorize_session, get_suite_session, list_history, migrate, open_connection,
    remaining_pool, submit_endless_passage, submit_reading_attempt, submit_suite_passage,
    AdvanceEndlessCommand, CreateEndlessCommand, CreateMemorizeCommand, CreateSuiteCommand,
    DbOpenOptions, PassageStatus, ReadingSubmitCommand, SubmitEndlessCommand,
    SubmitSuitePassageCommand, SuiteAssetSeed, TimerMode, TimerState,
};
use ielts_domain::domain::{Activity, SuiteFlowMode, SuiteStatus};
use ielts_domain::dto::ListHistoryQuery;

fn open_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempdir().unwrap();
    let mut conn = open_connection(&DbOpenOptions::create(dir.path().join("v2.db"))).unwrap();
    migrate(&mut conn).unwrap();
    (dir, conn)
}

fn payload(exam_id: &str) -> serde_json::Value {
    json!({
        "examId": exam_id,
        "answerKey": { "q1": "TRUE", "q2": "A" },
        "interactionModel": {},
        "questionGroups": []
    })
}

fn suite_sequence() -> Vec<SuiteAssetSeed> {
    vec![
        SuiteAssetSeed {
            asset_id: "p1".into(),
            title: Some("P1".into()),
            category: Some("P1".into()),
        },
        SuiteAssetSeed {
            asset_id: "p2".into(),
            title: Some("P2".into()),
            category: Some("P2".into()),
        },
        SuiteAssetSeed {
            asset_id: "p3".into(),
            title: Some("P3".into()),
            category: Some("P3".into()),
        },
    ]
}

#[test]
fn timer_pause_and_countdown_policy() {
    let mut t = TimerState::new_suite(1_000);
    t.mode = TimerMode::Countdown;
    t.limit_seconds = Some(10);
    assert!(!t.should_auto_submit(5_000));
    t.pause(5_000);
    assert_eq!(t.elapsed_seconds(20_000), 4);
    t.resume(20_000);
    assert!(t.should_auto_submit(26_000));
}

#[test]
fn suite_create_submit_and_recover() {
    let (_dir, conn) = open_db();
    let session = create_suite_session(
        &conn,
        &CreateSuiteCommand {
            flow_mode: Some("simulation".into()),
            frequency_scope: Some("all".into()),
            seed: Some("s1".into()),
            sequence: suite_sequence(),
            timer: None,
            idempotency_key: Some("create-suite-1".into()),
        },
    )
    .unwrap();
    assert_eq!(session.status, SuiteStatus::Active);
    assert_eq!(session.flow_mode, SuiteFlowMode::Simulation);
    assert_eq!(session.current_index, 0);

    let replay = create_suite_session(
        &conn,
        &CreateSuiteCommand {
            flow_mode: Some("simulation".into()),
            frequency_scope: None,
            seed: None,
            sequence: suite_sequence(),
            timer: None,
            idempotency_key: Some("create-suite-1".into()),
        },
    )
    .unwrap();
    assert_eq!(replay.session_id, session.session_id);

    let r1 = submit_suite_passage(
        &conn,
        &SubmitSuitePassageCommand {
            suite_id: session.session_id.clone(),
            asset_id: "p1".into(),
            payload: payload("p1"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: Some(30_000),
            title_snapshot: Some("P1".into()),
            timer_snapshot: None,
            idempotency_key: "suite-sub-1".into(),
        },
    )
    .unwrap();
    assert_eq!(r1.suite_session.current_index, 1);
    assert_eq!(r1.suite_session.aggregate.submitted_passages, 1);

    let bad = submit_suite_passage(
        &conn,
        &SubmitSuitePassageCommand {
            suite_id: session.session_id.clone(),
            asset_id: "p3".into(),
            payload: payload("p3"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: None,
            title_snapshot: None,
            timer_snapshot: None,
            idempotency_key: "suite-sub-bad".into(),
        },
    );
    assert!(bad.is_err());

    let loaded = get_suite_session(&conn, &session.session_id).unwrap();
    assert_eq!(loaded.current_index, 1);
    assert_eq!(loaded.sequence[0].status, PassageStatus::Submitted);

    let r2 = submit_suite_passage(
        &conn,
        &SubmitSuitePassageCommand {
            suite_id: session.session_id.clone(),
            asset_id: "p2".into(),
            payload: payload("p2"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: Some(10_000),
            title_snapshot: None,
            timer_snapshot: None,
            idempotency_key: "suite-sub-2".into(),
        },
    )
    .unwrap();
    let r3 = submit_suite_passage(
        &conn,
        &SubmitSuitePassageCommand {
            suite_id: session.session_id.clone(),
            asset_id: "p3".into(),
            payload: payload("p3"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: Some(10_000),
            title_snapshot: None,
            timer_snapshot: None,
            idempotency_key: "suite-sub-3".into(),
        },
    )
    .unwrap();
    assert_eq!(r3.suite_session.status, SuiteStatus::Completed);
    assert_eq!(r3.suite_session.aggregate.submitted_passages, 3);
    assert!(r2.submission.score.accuracy > 0.0);

    let again = submit_suite_passage(
        &conn,
        &SubmitSuitePassageCommand {
            suite_id: session.session_id.clone(),
            asset_id: "p3".into(),
            payload: payload("p3"),
            answers: json!({ "q1": "FALSE" }),
            marked_questions: vec![],
            duration_ms: Some(1),
            title_snapshot: None,
            timer_snapshot: None,
            idempotency_key: "suite-sub-3".into(),
        },
    )
    .unwrap();
    assert_eq!(
        again.submission.score.accuracy,
        r3.submission.score.accuracy
    );
}

#[test]
fn endless_pool_and_advance() {
    let (_dir, conn) = open_db();
    let session = create_endless_session(
        &conn,
        &CreateEndlessCommand {
            pool: vec!["a".into(), "b".into(), "c".into()],
            pool_policy: None,
            idempotency_key: Some("e1".into()),
        },
    )
    .unwrap();
    assert_eq!(session.current_asset_id.as_deref(), Some("a"));

    let r = submit_endless_passage(
        &conn,
        &SubmitEndlessCommand {
            session_id: session.id.clone(),
            asset_id: "a".into(),
            payload: payload("a"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: Some(5_000),
            title_snapshot: None,
            idempotency_key: "e-sub-1".into(),
        },
    )
    .unwrap();
    assert_eq!(r.next_asset_id.as_deref(), Some("b"));
    assert_eq!(remaining_pool(&r.session).len(), 2);

    let advanced = advance_endless(
        &conn,
        &AdvanceEndlessCommand {
            session_id: session.id.clone(),
            next_asset_id: Some("c".into()),
        },
    )
    .unwrap();
    assert_eq!(advanced.current_asset_id.as_deref(), Some("c"));
}

#[test]
fn memorize_excluded_from_history() {
    let (_dir, conn) = open_db();
    let mem = create_memorize_session(
        &conn,
        &CreateMemorizeCommand {
            asset_id: "mem-asset".into(),
            title_snapshot: Some("Mem".into()),
            payload: None,
            idempotency_key: Some("m1".into()),
        },
    )
    .unwrap();
    assert!(mem.read_only);
    assert!(!mem.enters_history);

    submit_reading_attempt(
        &conn,
        &ReadingSubmitCommand {
            attempt_id: "normal-1".into(),
            asset_id: "normal-asset".into(),
            payload: payload("normal-asset"),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec![],
            duration_ms: Some(1000),
            title_snapshot: Some("N".into()),
            idempotency_key: "n1".into(),
        },
    )
    .unwrap();

    let page = list_history(
        &conn,
        &ListHistoryQuery {
            activity: Some(Activity::Reading),
            search: None,
            start_date: None,
            end_date: None,
            min_score: None,
            max_score: None,
            limit: 50,
            offset: 0,
            cursor: None,
        },
    )
    .unwrap();
    assert!(page.items.iter().all(|i| i.id != mem.attempt.id));
    assert!(page.items.iter().any(|i| i.id == "normal-1"));

    finish_memorize_session(&conn, &mem.attempt.id).unwrap();
}
