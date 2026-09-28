//! A Claude that answers from a script and remembers what it was sent.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scorsese_providers::api::anthropic::content::Block;
use scorsese_providers::api::anthropic::request::Request;
use scorsese_providers::claude::{Claude, ClaudeError, Response, Stop, Streamed};
use scorsese_providers::prices::claude::Usage;
use serde_json::Value;
use serde_json::value::RawValue;

/// What every scripted call reports: 1 000 in and 500 out, which is
/// 14 000 µ$ at Opus 5.5's rates and 15 400 µ$ charged.
pub(crate) const USAGE: Usage = Usage {
    input: 1_000,
    output: 500,
    cache_write_5m: 0,
    cache_write_1h: 0,
    cache_read: 0,
};

/// What one scripted call is charged, in micro-dollars.
pub(crate) const CHARGED: i64 = 15_400;

/// Replies to give, in order, and the requests received.
pub(crate) struct Script {
    replies: Mutex<VecDeque<Response>>,
    sent: Mutex<Vec<Request>>,
    delay: Duration,
}

impl Script {
    /// A script answering with `replies`, one per call.
    pub(crate) fn new(replies: Vec<Response>) -> Arc<Self> {
        Self::slow(replies, Duration::ZERO)
    }

    /// The same, each reply taking `delay` to arrive.
    pub(crate) fn slow(replies: Vec<Response>, delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            sent: Mutex::default(),
            delay,
        })
    }

    /// Answer with `replies` from now on, in place of what was left.
    pub(crate) fn replace(&self, replies: Vec<Response>) {
        *self.replies.lock().expect("the test setup holds") = replies.into();
    }

    /// Every request received, in order.
    pub(crate) fn requests(&self) -> Vec<Request> {
        self.sent.lock().expect("the test setup holds").clone()
    }

    /// Request `n`'s messages, each as the exact text sent.
    pub(crate) fn messages(&self, n: usize) -> Vec<String> {
        self.requests()[n]
            .messages
            .iter()
            .map(|message| message.get().to_owned())
            .collect()
    }

    /// Request `n`'s messages, parsed.
    pub(crate) fn parsed(&self, n: usize) -> Vec<Value> {
        let texts = self.messages(n);
        texts
            .iter()
            .map(|text| serde_json::from_str(text).expect("the test setup holds"))
            .collect()
    }
}

impl Claude for Script {
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Response, ClaudeError> {
        self.sent
            .lock()
            .expect("the test setup holds")
            .push(request.clone());
        std::thread::sleep(self.delay);
        let reply = self
            .replies
            .lock()
            .expect("the test setup holds")
            .pop_front();
        let reply = reply.unwrap_or_else(|| answers("(the script ran out)"));
        on(Streamed::Text(&reply.text()));
        on(Streamed::BlockEnd);
        Ok(reply)
    }
}

/// A reply that answers with `text`.
pub(crate) fn answers(text: &str) -> Response {
    reply(Block::Text { text: text.into() }, Stop::EndTurn)
}

/// A reply that calls `tool` with `input`.
pub(crate) fn calls(tool: &str, input: Value) -> Response {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let call = Block::ToolUse {
        id: format!("toolu_{}", NEXT.fetch_add(1, Ordering::Relaxed)),
        name: tool.into(),
        input: RawValue::from_string(input.to_string()).expect("the test setup holds"),
    };
    reply(call, Stop::ToolUse)
}

fn reply(block: Block, stop: Stop) -> Response {
    let thinking = Block::Thinking {
        thinking: "Looking first.".into(),
        signature: "c2lnbmF0dXJl".into(),
    };
    Response {
        id: "msg_scripted".into(),
        model: "claude-opus-5-5".into(),
        content: vec![thinking, block],
        stop,
        usage: USAGE,
    }
}
