use std::sync::atomic::{AtomicBool, Ordering};

use ielts_application::{
    AgentStore, ApplicationError, CoachStore, EventSink, WritingEvaluationStore,
};
use ielts_db::{
    AppendCoachMessageCommand, BeginAgentRunCommand, BeginAgentToolCallCommand, CoachMessage,
    EvaluationEvent, EvaluationRunResult, FinishAgentRunCommand, FinishAgentToolCallCommand,
    PreparedEvaluation, ProviderError, RecordCoachFailureCommand, StartEvaluationCommand,
};
use ielts_domain::dto::{WritingFeedbackV4, WritingScoreV4};
use serde_json::Value;
use tauri::ipc::Channel;

use super::state::AppDb;

pub(crate) struct ApplicationStore<'a> {
    db: &'a AppDb,
}

impl<'a> ApplicationStore<'a> {
    pub(crate) fn new(db: &'a AppDb) -> Self {
        Self { db }
    }
}

impl WritingEvaluationStore for ApplicationStore<'_> {
    fn prepare(
        &self,
        command: &StartEvaluationCommand,
        provider_id: &str,
        model: &str,
    ) -> Result<PreparedEvaluation, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::prepare_evaluation(conn, command, provider_id, model))
            .map_err(writing_error)
    }

    fn list_events(
        &self,
        evaluation_id: &str,
        after_sequence: u32,
    ) -> Result<Vec<EvaluationEvent>, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::list_events(conn, evaluation_id, after_sequence))
            .map_err(writing_error)
    }

    fn finish(
        &self,
        prepared: &PreparedEvaluation,
        score: Result<WritingScoreV4, ProviderError>,
        feedback: Option<WritingFeedbackV4>,
        review_error: Option<ProviderError>,
    ) -> Result<EvaluationRunResult, ApplicationError> {
        self.db
            .with_conn(|conn| {
                ielts_db::finish_evaluation(conn, prepared, score, feedback, review_error)
            })
            .map_err(writing_error)
    }

    fn request_cancel(&self, evaluation_id: &str) -> Result<bool, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::request_cancel(conn, evaluation_id))
            .map_err(writing_error)
    }
}

impl CoachStore for ApplicationStore<'_> {
    fn append_message(
        &self,
        command: &AppendCoachMessageCommand,
    ) -> Result<CoachMessage, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::append_coach_message(conn, command))
            .map_err(enrichment_error)
    }

    fn load_history(
        &self,
        thread_id: &str,
        limit: u32,
    ) -> Result<Vec<CoachMessage>, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::list_coach_messages(conn, thread_id, None, limit))
            .map_err(enrichment_error)
    }

    fn complete_run(
        &self,
        thread_id: &str,
        content: &str,
        payload: Option<Value>,
    ) -> Result<CoachMessage, ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::complete_coach_run(conn, thread_id, content, payload))
            .map_err(enrichment_error)
    }

    fn record_failure(&self, command: &RecordCoachFailureCommand) -> Result<(), ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::record_coach_failure(conn, command))
            .map(|_| ())
            .map_err(enrichment_error)
    }
}

impl AgentStore for ApplicationStore<'_> {
    fn begin_run(&self, run: &BeginAgentRunCommand) -> Result<(), ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::begin_agent_run(conn, run))
            .map_err(agent_error)
    }

    fn begin_tool_call(&self, call: &BeginAgentToolCallCommand) -> Result<(), ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::begin_agent_tool_call(conn, call))
            .map_err(agent_error)
    }

    fn finish_tool_call(&self, call: &FinishAgentToolCallCommand) -> Result<(), ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::finish_agent_tool_call(conn, call))
            .map_err(agent_error)
    }

    fn finish_run(&self, run: &FinishAgentRunCommand) -> Result<(), ApplicationError> {
        self.db
            .with_conn(|conn| ielts_db::finish_agent_run(conn, run))
            .map_err(agent_error)
    }
}

pub(crate) struct ChannelEventSink {
    channel: Channel<EvaluationEvent>,
    closed: AtomicBool,
}

impl ChannelEventSink {
    pub(crate) fn new(channel: Channel<EvaluationEvent>) -> Self {
        Self {
            channel,
            closed: AtomicBool::new(false),
        }
    }
}

impl EventSink for ChannelEventSink {
    fn emit(&self, event: EvaluationEvent) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        if let Err(error) = self.channel.send(event) {
            self.closed.store(true, Ordering::Relaxed);
            tracing::debug!(error = %error, "writing evaluation channel closed");
        }
    }
}

fn writing_error(error: ielts_db::DbError) -> ApplicationError {
    ApplicationError::new("writing.error", error.to_string(), false)
}

fn enrichment_error(error: ielts_db::DbError) -> ApplicationError {
    ApplicationError::new("enrichment.error", error.to_string(), false)
}

fn agent_error(error: ielts_db::DbError) -> ApplicationError {
    ApplicationError::new("agent.persistence_failed", error.to_string(), false)
}
