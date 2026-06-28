use crate::providers::traits::Message;

/// Rough estimate: 1 token ≈ 4 chars for English text.
const CHARS_PER_TOKEN: usize = 4;

pub struct ContextCompactor {
    /// Max total characters across all messages before compaction triggers.
    pub max_context_chars: usize,
    /// How many recent messages to always preserve.
    pub preserve_recent: usize,
    /// Minimum chars for a summary slot. Inline content shorter than
    /// this is never worth spilling.
    pub min_spill_chars: usize,
}

impl Default for ContextCompactor {
    fn default() -> Self {
        Self {
            // ~100K tokens worth of chars
            max_context_chars: 400_000,
            preserve_recent: 10,
            min_spill_chars: 5_000,
        }
    }
}

impl ContextCompactor {
    pub fn new(max_context_chars: usize, preserve_recent: usize) -> Self {
        Self {
            max_context_chars,
            preserve_recent,
            min_spill_chars: 5_000,
        }
    }

    /// Estimate approximate token cost of a message.
    pub fn estimate_tokens(msg: &Message) -> usize {
        msg.content.len() / CHARS_PER_TOKEN
    }

    /// Total estimated tokens across all messages.
    pub fn total_tokens(messages: &[Message]) -> usize {
        messages.iter().map(Self::estimate_tokens).sum()
    }

    /// Total chars across all messages.
    pub fn total_chars(messages: &[Message]) -> usize {
        messages.iter().map(|m| m.content.len()).sum()
    }

    /// Returns `true` if the context should be compacted.
    pub fn should_compact(&self, messages: &[Message]) -> bool {
        Self::total_chars(messages) > self.max_context_chars
    }

    /// Compact by removing the largest tool-result messages that are
    /// outside the preserve window. Returns how many bytes were removed.
    pub fn compact(&self, messages: &mut Vec<Message>, spill_marker: &str) -> usize {
        if !self.should_compact(messages) {
            return 0;
        }

        let keep_from = if messages.len() > self.preserve_recent {
            messages.len() - self.preserve_recent
        } else {
            0
        };

        let mut removed: usize = 0;
        // Work backwards from the oldest messages we're allowed to touch.
        // Prioritise removing large tool results.
        for i in (0..keep_from).rev() {
            if !self.should_compact(messages) {
                break;
            }
            let msg = &messages[i];
            if msg.role == "tool" && msg.content.len() > self.min_spill_chars {
                removed += msg.content.len();
                messages[i].content = spill_marker.to_string();
            }
        }

        // If still over budget, drop entire oldest messages (FIFO).
        while self.should_compact(messages) && !messages.is_empty() && messages.len() > self.preserve_recent {
            let dropped = messages.remove(0);
            removed += dropped.content.len();
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.into(),
            content: content.into(),
            ..Default::default()
        }
    }

    #[test]
    fn small_context_no_compact() {
        let c = ContextCompactor::default();
        let msgs = vec![msg("user", "hi"), msg("assistant", "hello")];
        assert!(!c.should_compact(&msgs));
    }

    #[test]
    fn large_context_triggers_compact() {
        let c = ContextCompactor {
            min_spill_chars: 50, // lower threshold for test
            ..ContextCompactor::new(100, 2)
        };
        let mut msgs = vec![
            msg("user", "start"),
            msg("tool", &"x".repeat(200)),
            msg("user", "middle"),
            msg("tool", &"y".repeat(200)),
        ];
        assert!(c.should_compact(&msgs));
        let removed = c.compact(&mut msgs, "[spilled]");
        assert!(removed > 0);
        // The total size should be reduced from the original
        assert!(removed > 200, "removed={removed}");
        // The most recent message is always preserved
        assert_eq!(msgs.last().unwrap().content, "y".repeat(200));
    }

    #[test]
    fn preserves_recent_messages() {
        let c = ContextCompactor::new(100, 2);
        let mut msgs = vec![
            msg("tool", &"x".repeat(200)),
            msg("user", "keep me"),
        ];
        c.compact(&mut msgs, "[spilled]");
        assert_eq!(msgs.last().unwrap().content, "keep me");
    }

    #[test]
    fn drops_entire_old_messages_if_needed() {
        let c = ContextCompactor::new(50, 1);
        let mut msgs = vec![
            msg("user", "old"),
            msg("tool", &"b".repeat(500)),
            msg("assistant", "newest"),
        ];
        c.compact(&mut msgs, "[spilled]");
        // should have dropped at least one message
        assert!(msgs.len() < 3);
        assert_eq!(msgs.last().unwrap().content, "newest");
    }
}
