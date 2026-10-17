//! What the chat says when the model could not answer (#1435).
//!
//! Never the vendor's own words: `…/v1/messages answered 413: {"error": …}`
//! is implementation, and the user never deals with implementation. Those go
//! to the server's log. What the user reads is what happened and the way
//! out, and the panel's localized line under it (`ending.failed`) says the
//! way out again in their own language.

use scorsese_providers::chat::ChatError;

/// Why a turn's model could not answer, in words for the user.
pub(super) fn unreached(error: &ChatError) -> String {
    if error.too_large() {
        "This conversation has grown too long for the assistant to read. Start a new \
         conversation: it carries on with the project as it is now."
            .to_owned()
    } else {
        "The assistant could not be reached. Send your message again in a moment.".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use scorsese_providers::api::http::HttpError;
    use scorsese_providers::chat::ClaudeError;

    use super::*;

    fn refused(status: u16) -> ChatError {
        ChatError::Claude(ClaudeError::Http(HttpError::Refused {
            url: "https://api.anthropic.com/v1/messages".into(),
            status,
            body: r#"{"error": {"type": "request_too_large"}}"#.into(),
        }))
    }

    #[test]
    fn a_conversation_too_large_to_send_says_to_start_a_new_one_and_nothing_of_the_vendor() {
        for error in [
            refused(413),
            ChatError::Claude(ClaudeError::Api {
                kind: "request_too_large".into(),
                message: "Request exceeds the maximum size".into(),
            }),
        ] {
            let said = unreached(&error);
            assert!(said.contains("Start a new conversation"), "{said}");
            assert!(!said.contains("413") && !said.contains('{'), "{said}");
        }
        let said = unreached(&refused(529));
        assert!(
            !said.contains("new conversation") && !said.contains("anthropic"),
            "{said}"
        );
    }
}
