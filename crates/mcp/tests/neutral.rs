//! MCP is a protocol, not a Claude feature (CLAUDE.md, *Architecture*).
//!
//! Whatever client is on the other end reads the same descriptions, so none
//! of them may name one: a tool that says "ask Claude" or "as in Codex" is
//! written for one caller and reads as a riddle to every other. A vendor this
//! crate's tools call (Gemini for a still, ElevenLabs for a line) is not a
//! client and may be named.

use scorsese_mcp::registry;

/// The names of clients, matched as whole words. "Claude Code" is caught by
/// "Claude".
const CLIENTS: [&str; 6] = ["Claude", "Anthropic", "GPT", "ChatGPT", "Codex", "OpenAI"];

/// The first client `text` names as a word of its own, if any.
fn client_named(text: &str) -> Option<&'static str> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    CLIENTS.into_iter().find(|client| {
        text.match_indices(client).any(|(at, _)| {
            !text[..at].ends_with(is_word) && !text[at + client.len()..].starts_with(is_word)
        })
    })
}

#[test]
fn a_client_named_as_a_word_is_found_and_a_vendor_is_not() {
    assert_eq!(client_named("ask Claude Code to bake it"), Some("Claude"));
    assert_eq!(client_named("as GPT-5 would"), Some("GPT"));
    assert_eq!(client_named("(ChatGPT)"), Some("ChatGPT"));
    assert_eq!(
        client_named("a still from Gemini, a line from ElevenLabs"),
        None
    );
    assert_eq!(client_named("Claudette's GPTs"), None);
}

#[test]
fn no_tool_or_argument_names_a_client() {
    let tools = registry();
    assert!(!tools.is_empty(), "no tools — this would pass vacuously");
    for tool in tools {
        let said = format!("{} {}", tool.description(), tool.schema());
        assert_eq!(
            client_named(&said),
            None,
            "`{}` names a client in what it says about itself: MCP is a protocol, \
             and every client reads the same words (CLAUDE.md, *Architecture*)",
            tool.name()
        );
    }
}
