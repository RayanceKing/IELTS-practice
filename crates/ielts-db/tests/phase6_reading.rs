//! Phase 6: reading scoring parity, drafts, idempotent submit.

use serde_json::json;
use tempfile::tempdir;

use ielts_db::{
    compare_answer, migrate, open_connection, patch_reading_answer, save_reading_draft,
    score_attempt, submit_reading_attempt, DbOpenOptions, MatchMode, ReadingDraftCommand,
    ReadingSubmitCommand,
};

fn open_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempdir().unwrap();
    let mut conn = open_connection(&DbOpenOptions::create(dir.path().join("v2.db"))).unwrap();
    migrate(&mut conn).unwrap();
    (dir, conn)
}

fn sample_payload() -> serde_json::Value {
    json!({
        "examId": "p1-demo",
        "meta": { "title": "Demo Passage", "category": "P1", "frequency": "high" },
        "questionCount": 4,
        "questionOrder": ["q1", "q2", "q3", "q4"],
        "answerKey": {
            "q1": "TRUE",
            "q2": "A",
            "q3": ["B", "C"],
            "q4": ["A", "D"]
        },
        "interactionModel": {
            "q4": { "control": "checkbox" }
        },
        "questionGroups": [
            { "kind": "tfng", "questionIds": ["q1"] },
            { "kind": "mcq", "questionIds": ["q2", "q3"] },
            { "kind": "multi", "questionIds": ["q4"] }
        ]
    })
}

#[test]
fn scoring_parity_aliases_and_modes() {
    let (ok, _, _, mode) = compare_answer(&json!("YES"), &json!("TRUE"), None);
    assert_eq!(ok, Some(true));
    assert_eq!(mode, MatchMode::Single);

    let (ok, _, _, mode) = compare_answer(&json!("B"), &json!(["A", "B"]), None);
    assert_eq!(ok, Some(true));
    assert_eq!(mode, MatchMode::Alternatives);

    let (ok, _, _, mode) = compare_answer(
        &json!(["D", "A"]),
        &json!(["A", "D"]),
        Some("checkbox"),
    );
    assert_eq!(ok, Some(true));
    assert_eq!(mode, MatchMode::Set);
}

#[test]
fn draft_patch_and_idempotent_submit() {
    let (_dir, conn) = open_db();
    let payload = sample_payload();

    save_reading_draft(
        &conn,
        &ReadingDraftCommand {
            attempt_id: "r-1".into(),
            asset_id: "p1-demo".into(),
            answers: json!({ "q1": "TRUE", "q2": "A" }),
            marked_questions: vec!["q3".into()],
            title_snapshot: Some("Demo Passage".into()),
            idempotency_key: "draft-1".into(),
        },
    )
    .unwrap();

    patch_reading_answer(&conn, "r-1", "q3", &json!("B"), true).unwrap();

    let result = submit_reading_attempt(
        &conn,
        &ReadingSubmitCommand {
            attempt_id: "r-1".into(),
            asset_id: "p1-demo".into(),
            payload: payload.clone(),
            answers: json!({
                "q1": "TRUE",
                "q2": "A",
                "q3": "B",
                "q4": ["A", "D"]
            }),
            marked_questions: vec!["q3".into()],
            duration_ms: Some(90_000),
            title_snapshot: Some("Demo Passage".into()),
            idempotency_key: "submit-1".into(),
        },
    )
    .unwrap();

    assert!(!result.idempotent_replay);
    assert!(result.score.accuracy > 0.9);
    assert_eq!(result.attempt.status, ielts_domain::AttemptStatus::Completed);
    assert!(result.attempt.answers.iter().any(|a| a.marked));

    let replay = submit_reading_attempt(
        &conn,
        &ReadingSubmitCommand {
            attempt_id: "r-1".into(),
            asset_id: "p1-demo".into(),
            payload,
            answers: json!({ "q1": "FALSE" }), // would change score if re-scored
            marked_questions: vec![],
            duration_ms: Some(1),
            title_snapshot: None,
            idempotency_key: "submit-1".into(),
        },
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.score.accuracy, result.score.accuracy);

    // only one history row
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM attempts WHERE id = 'r-1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn score_attempt_weights_checkbox() {
    let mut key = serde_json::Map::new();
    key.insert("q1".into(), json!("A"));
    key.insert("q2".into(), json!(["A", "B"]));
    let mut user = serde_json::Map::new();
    user.insert("q1".into(), json!("A"));
    user.insert("q2".into(), json!(["A", "B"]));
    let mut controls = serde_json::Map::new();
    controls.insert("q2".into(), json!("checkbox"));
    let (summary, _) = score_attempt(&key, &user, &controls, &serde_json::Map::new());
    assert_eq!(summary.total, 3.0);
    assert_eq!(summary.correct, 3.0);
    assert_eq!(summary.accuracy, 1.0);
}
