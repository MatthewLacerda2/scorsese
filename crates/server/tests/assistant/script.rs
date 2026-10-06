//! A model that answers from a script and remembers what it was sent — in
//! the wire of whichever model the request names.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scorsese_providers::api::anthropic::content::Block;
use scorsese_providers::api::anthropic::request::{Message as Wire, MessageContent, Role};
use scorsese_providers::chat::{
    self, Chat, ChatError, Message, Part, Reply, Request, Stop, Streamed, Usage, Vendor,
};
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

/// What every scripted reply thinks, streamed as a progress note (#767).
pub(crate) const THOUGHT: &str = "Weighing the cut privately.";

/// What one scripted call is charged, in micro-dollars.
pub(crate) const CHARGED: i64 = 15_400;

/// Replies to give, in order, and the requests received.
pub(crate) struct Script {
    replies: Mutex<VecDeque<Reply>>,
    sent: Mutex<Vec<Request>>,
    delay: Duration,
}

impl Script {
    /// A script answering with `replies`, one per call.
    pub(crate) fn new(replies: Vec<Reply>) -> Arc<Self> {
        Self::slow(replies, Duration::ZERO)
    }

    /// The same, each reply taking `delay` to arrive.
    pub(crate) fn slow(replies: Vec<Reply>, delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            sent: Mutex::default(),
            delay,
        })
    }

    /// Answer with `replies` from now on, in place of what was left.
    pub(crate) fn replace(&self, replies: Vec<Reply>) {
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

impl Chat for Script {
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Reply, ChatError> {
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
        let mut reply = reply.unwrap_or_else(|| answers("(the script ran out)"));
        reply.model = request.model.id().to_owned();
        reply.native = native(request, &reply.message);
        // A real reply thinks first, and both vendors stream that as notes.
        on(Streamed::Progress(THOUGHT));
        on(Streamed::BlockEnd);
        on(Streamed::Text(&reply.text()));
        on(Streamed::BlockEnd);
        Ok(reply)
    }
}

/// `message` as the request's model would have sent it: on Claude, after a
/// thinking block, the way a real reply carries one.
fn native(request: &Request, message: &Message) -> Box<RawValue> {
    if request.model.vendor() == Vendor::Google {
        return chat::freeze(request.model, message).expect("the test setup holds");
    }
    let frozen = chat::freeze(request.model, message).expect("the test setup holds");
    let parsed: Value = serde_json::from_str(frozen.get()).expect("the test setup holds");
    let mut blocks = vec![Block::Thinking {
        thinking: "Looking first.".into(),
        signature: "c2lnbmF0dXJl".into(),
    }];
    for block in parsed["content"].as_array().into_iter().flatten() {
        blocks.push(match block["type"].as_str() {
            Some("tool_use") => Block::ToolUse {
                id: block["id"].as_str().unwrap_or_default().into(),
                name: block["name"].as_str().unwrap_or_default().into(),
                input: RawValue::from_string(block["input"].to_string())
                    .expect("the test setup holds"),
            },
            _ => Block::Text {
                text: block["text"].as_str().unwrap_or_default().into(),
            },
        });
    }
    let message = Wire {
        role: Role::Assistant,
        content: MessageContent::Blocks(blocks),
    };
    message.raw().expect("the test setup holds")
}

/// A reply that answers with `text`.
pub(crate) fn answers(text: &str) -> Reply {
    reply(Part::Text { text: text.into() }, Stop::EndTurn)
}

/// A reply that calls `tool` with `input`.
pub(crate) fn calls(tool: &str, input: Value) -> Reply {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let call = Part::Call {
        id: format!("toolu_{}", NEXT.fetch_add(1, Ordering::Relaxed)),
        name: tool.into(),
        input,
    };
    reply(call, Stop::ToolUse)
}

/// A reply of `part`, stopped by `stop`; its model and bytes are filled in
/// when it is sent.
pub(crate) fn reply(part: Part, stop: Stop) -> Reply {
    Reply {
        model: String::new(),
        message: Message::Assistant {
            content: vec![part],
        },
        native: RawValue::from_string("null".into()).expect("the test setup holds"),
        stop,
        usage: USAGE,
        thinking: None,
    }
}
