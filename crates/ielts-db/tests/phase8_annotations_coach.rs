//! Phase 8: annotations, dictionary, vocabulary, coach threads.

use serde_json::json;
use tempfile::tempdir;

use ielts_db::{
    append_coach_message, attempt_score_snapshot, ensure_coach_thread, import_dictionary,
    list_annotations, list_coach_messages, list_vocab, lookup_term, migrate, open_connection,
    record_coach_failure, resolve_anchor, revalidate_annotations, review_vocab,
    submit_reading_attempt, upsert_annotation, upsert_vocab, AppendCoachMessageCommand,
    DbOpenOptions, DictionaryEntry, EnsureCoachThreadCommand, ImportDictionaryCommand,
    RecordCoachFailureCommand, ReadingSubmitCommand, ReviewVocabCommand, TextAnchor,
    UpsertAnnotationCommand, UpsertVocabCommand,
};

fn open_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempdir().unwrap();
    let mut conn = open_connection(&DbOpenOptions::create(dir.path().join("v2.db"))).unwrap();
    migrate(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn annotation_stable_anchor_and_mismatch() {
    let (_dir, conn) = open_db();
    let ann = upsert_annotation(
        &conn,
        &UpsertAnnotationCommand {
            id: None,
            attempt_id: None,
            asset_id: "asset-1".into(),
            scope: "passage".into(),
            question_id: None,
            kind: "highlight".into(),
            anchor: TextAnchor {
                text: "climate change".into(),
                before: Some("about".into()),
                after: Some("is".into()),
                occurrence: 0,
                start_offset: None,
                end_offset: None,
                content_fingerprint: Some("fp1".into()),
            },
            note_text: Some("key phrase".into()),
        },
    )
    .unwrap();
    assert!(!ann.id.is_empty());

    let doc = "Scientists talk about climate change is urgent.";
    let (s, e) = resolve_anchor(doc, &ann.anchor).unwrap();
    assert!(e > s);

    let list = list_annotations(&conn, "asset-1", None).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].note_text.as_deref(), Some("key phrase"));

    let checked = revalidate_annotations(&conn, "asset-1", "passage", "totally different text").unwrap();
    assert_eq!(checked[0].mismatch.as_deref(), Some("text_not_found"));
}

#[test]
fn dictionary_and_vocab_review() {
    let (_dir, conn) = open_db();
    import_dictionary(
        &conn,
        &ImportDictionaryCommand {
            entries: vec![DictionaryEntry {
                term: "ephemeral".into(),
                normalized_term: "ephemeral".into(),
                definition: "lasting a very short time".into(),
                phonetic: Some("/ɪˈfem.ər.əl/".into()),
                part_of_speech: Some("adj".into()),
                example: Some("Fame can be ephemeral.".into()),
                source_label: Some("builtin".into()),
                license: Some("CC".into()),
                payload: None,
                found: true,
            }],
        },
    )
    .unwrap();
    let hit = lookup_term(&conn, "Ephemeral").unwrap();
    assert!(hit.found);
    assert!(hit.definition.contains("short"));

    let miss = lookup_term(&conn, "zzzz-not-a-word").unwrap();
    assert!(!miss.found);

    let item = upsert_vocab(
        &conn,
        &UpsertVocabCommand {
            id: None,
            term: "ephemeral".into(),
            definition: Some(hit.definition.clone()),
            phonetic: hit.phonetic.clone(),
            part_of_speech: hit.part_of_speech.clone(),
            example: hit.example.clone(),
            source_asset_id: Some("a1".into()),
            source_attempt_id: None,
            tags: vec!["reading".into()],
        },
    )
    .unwrap();
    let reviewed = review_vocab(
        &conn,
        &ReviewVocabCommand {
            item_id: item.id.clone(),
            grade: 2,
        },
    )
    .unwrap();
    assert!(reviewed.review.unwrap().repetitions >= 1);
    assert_eq!(list_vocab(&conn, 20, 0).unwrap().len(), 1);
}

#[test]
fn coach_incremental_messages_failure_preserves_score() {
    let (_dir, conn) = open_db();
    // scored attempt first
    let sub = submit_reading_attempt(
        &conn,
        &ReadingSubmitCommand {
            attempt_id: "att-coach-1".into(),
            asset_id: "asset-c".into(),
            payload: json!({
                "examId": "asset-c",
                "answerKey": { "q1": "TRUE" },
                "interactionModel": {},
                "questionGroups": []
            }),
            answers: json!({ "q1": "TRUE" }),
            marked_questions: vec![],
            duration_ms: Some(1000),
            title_snapshot: Some("T".into()),
            idempotency_key: "c-sub".into(),
        },
    )
    .unwrap();
    let before = attempt_score_snapshot(&conn, "att-coach-1").unwrap();
    assert!(before.0.is_some());

    let thread = ensure_coach_thread(
        &conn,
        &EnsureCoachThreadCommand {
            thread_id: None,
            attempt_id: Some("att-coach-1".into()),
            asset_id: Some("asset-c".into()),
            kind: "review".into(),
        },
    )
    .unwrap();
    append_coach_message(
        &conn,
        &AppendCoachMessageCommand {
            thread_id: thread.id.clone(),
            role: "user".into(),
            content: "Please review my mistakes".into(),
            structured_payload: None,
            status: "completed".into(),
        },
    )
    .unwrap();
    append_coach_message(
        &conn,
        &AppendCoachMessageCommand {
            thread_id: thread.id.clone(),
            role: "assistant".into(),
            content: "Focus on TRUE/FALSE traps.".into(),
            structured_payload: Some(json!({ "kind": "review" })),
            status: "completed".into(),
        },
    )
    .unwrap();
    let msgs = list_coach_messages(&conn, &thread.id, Some(0), 50).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].sequence, 1);
    assert_eq!(msgs[1].sequence, 2);

    record_coach_failure(
        &conn,
        &RecordCoachFailureCommand {
            thread_id: thread.id.clone(),
            error: json!({ "code": "provider_timeout", "message": "timeout" }),
            preserve_scores: true,
        },
    )
    .unwrap();

    let after = attempt_score_snapshot(&conn, "att-coach-1").unwrap();
    assert_eq!(before, after);
    assert_eq!(sub.attempt.id, "att-coach-1");

    let more = list_coach_messages(&conn, &thread.id, Some(2), 50).unwrap();
    assert_eq!(more.len(), 1); // failure system message
    assert_eq!(more[0].status, "failed");
}
