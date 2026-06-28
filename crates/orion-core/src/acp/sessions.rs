//! ACP session registry — tracks per-session state in memory.

use crate::acp::types::{PendingPermission, SessionId, SessionUpdate};
use dashmap::DashMap;
use parking_lot::Mutex as PMutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Per-session state shared between the ACP dispatcher and the prompt handler.
pub struct SessionState {
    pub session_id: SessionId,
    pub cwd: std::path::PathBuf,
    pub messages: PMutex<Vec<SessionMessage>>,
    pub cancel_token: CancellationToken,
    pub pending_permissions: Arc<PMutex<Vec<PendingPermission>>>,
    pub permission_added: Arc<Notify>,
    pub current_turn: Arc<AtomicUsize>,
    pub inbound_tx: tokio::sync::mpsc::UnboundedSender<SessionUpdate>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

/// All live sessions.
#[derive(Default, Clone)]
pub struct SessionRegistry {
    sessions: Arc<DashMap<SessionId, Arc<SessionState>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, state: Arc<SessionState>) {
        self.sessions.insert(state.session_id.clone(), state);
    }

    pub fn get(&self, id: &SessionId) -> Option<Arc<SessionState>> {
        self.sessions.get(id).map(|e| e.value().clone())
    }

    pub fn remove(&self, id: &SessionId) -> Option<Arc<SessionState>> {
        self.sessions.remove(id).map(|(_, v)| v)
    }

    pub fn list(&self) -> Vec<SessionId> {
        self.sessions.iter().map(|e| e.key().clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

impl SessionState {
    pub fn new(
        session_id: SessionId,
        cwd: std::path::PathBuf,
        inbound_tx: tokio::sync::mpsc::UnboundedSender<SessionUpdate>,
    ) -> Arc<Self> {
        Arc::new(Self {
            session_id,
            cwd,
            messages: PMutex::new(Vec::new()),
            cancel_token: CancellationToken::new(),
            pending_permissions: Arc::new(PMutex::new(Vec::new())),
            permission_added: Arc::new(Notify::new()),
            current_turn: Arc::new(AtomicUsize::new(0)),
            inbound_tx,
        })
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
        // Wake any permission waiters so they unblock.
        let pending = self.pending_permissions.lock();
        for p in pending.iter() {
            p.notify.notify_waiters();
        }
    }

    pub fn turn_count(&self) -> usize {
        self.current_turn.load(Ordering::SeqCst)
    }

    pub fn begin_turn(&self) {
        self.current_turn.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn session_registry_roundtrip() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let id = SessionId::new();
        let state = SessionState::new(id.clone(), PathBuf::from("/tmp"), tx);
        registry.insert(state.clone());
        assert_eq!(registry.len(), 1);
        let fetched = registry.get(&id).unwrap();
        assert_eq!(fetched.session_id, id);
        registry.remove(&id);
        assert!(registry.is_empty());
    }

    #[test]
    fn session_cancel_does_not_panic() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let state = SessionState::new(SessionId::new(), PathBuf::from("/tmp"), tx);
        state.cancel();
        assert!(state.cancel_token.is_cancelled());
    }

    #[test]
    fn session_message_serializes() {
        let m = SessionMessage {
            role: MessageRole::User,
            content: "hello".into(),
            timestamp: chrono::Utc::now(),
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["role"], "user");
        assert_eq!(v["content"], "hello");
    }

    #[test]
    fn turn_counter_increments() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let state = SessionState::new(SessionId::new(), PathBuf::from("/tmp"), tx);
        assert_eq!(state.turn_count(), 0);
        state.begin_turn();
        state.begin_turn();
        assert_eq!(state.turn_count(), 2);
    }
}
