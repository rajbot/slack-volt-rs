mod elements;

pub use elements::*;

use serde_json::{json, Value};

pub fn section(text: &str) -> Value {
    json!({
        "type": "section",
        "text": { "type": "mrkdwn", "text": text }
    })
}

pub fn section_with_accessory(text: &str, accessory: Value) -> Value {
    json!({
        "type": "section",
        "text": { "type": "mrkdwn", "text": text },
        "accessory": accessory
    })
}

pub fn header(text: &str) -> Value {
    json!({
        "type": "header",
        "text": { "type": "plain_text", "text": text }
    })
}

pub fn divider() -> Value {
    json!({ "type": "divider" })
}

pub fn actions(block_id: &str, elements: Vec<Value>) -> Value {
    json!({
        "type": "actions",
        "block_id": block_id,
        "elements": elements
    })
}

pub fn context(elements: Vec<Value>) -> Value {
    json!({
        "type": "context",
        "elements": elements
    })
}

pub fn input(block_id: &str, label: &str, element: Value) -> Value {
    json!({
        "type": "input",
        "block_id": block_id,
        "label": { "type": "plain_text", "text": label },
        "element": element
    })
}

pub fn input_optional(block_id: &str, label: &str, element: Value) -> Value {
    json!({
        "type": "input",
        "block_id": block_id,
        "label": { "type": "plain_text", "text": label },
        "element": element,
        "optional": true
    })
}

pub fn modal(callback_id: &str, title: &str, blocks: Vec<Value>) -> Value {
    json!({
        "type": "modal",
        "callback_id": callback_id,
        "title": { "type": "plain_text", "text": title },
        "submit": { "type": "plain_text", "text": "Submit" },
        "close": { "type": "plain_text", "text": "Cancel" },
        "blocks": blocks
    })
}

pub fn modal_with_metadata(
    callback_id: &str,
    title: &str,
    blocks: Vec<Value>,
    private_metadata: &str,
) -> Value {
    json!({
        "type": "modal",
        "callback_id": callback_id,
        "title": { "type": "plain_text", "text": title },
        "submit": { "type": "plain_text", "text": "Submit" },
        "close": { "type": "plain_text", "text": "Cancel" },
        "blocks": blocks,
        "private_metadata": private_metadata
    })
}

pub fn mrkdwn(text: &str) -> Value {
    json!({ "type": "mrkdwn", "text": text })
}

pub fn plain_text(text: &str) -> Value {
    json!({ "type": "plain_text", "text": text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_block() {
        let block = section("hello *world*");
        assert_eq!(block["type"], "section");
        assert_eq!(block["text"]["type"], "mrkdwn");
        assert_eq!(block["text"]["text"], "hello *world*");
    }

    #[test]
    fn test_header_block() {
        let block = header("My Header");
        assert_eq!(block["type"], "header");
        assert_eq!(block["text"]["type"], "plain_text");
        assert_eq!(block["text"]["text"], "My Header");
    }

    #[test]
    fn test_divider_block() {
        let block = divider();
        assert_eq!(block["type"], "divider");
    }

    #[test]
    fn test_modal_with_metadata() {
        let m = modal_with_metadata("cb1", "Title", vec![divider()], "meta123");
        assert_eq!(m["type"], "modal");
        assert_eq!(m["callback_id"], "cb1");
        assert_eq!(m["title"]["text"], "Title");
        assert_eq!(m["private_metadata"], "meta123");
        assert_eq!(m["blocks"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_input_optional() {
        let block = input_optional("b1", "Label", json!({"type": "plain_text_input"}));
        assert_eq!(block["type"], "input");
        assert_eq!(block["optional"], true);
        assert_eq!(block["label"]["text"], "Label");
    }

    #[test]
    fn test_actions_block() {
        let block = actions("act1", vec![button("btn1", "Click me")]);
        assert_eq!(block["type"], "actions");
        assert_eq!(block["block_id"], "act1");
        assert_eq!(block["elements"].as_array().unwrap().len(), 1);
    }
}
