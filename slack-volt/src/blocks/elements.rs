use serde_json::{json, Value};

pub fn button(action_id: &str, text: &str) -> Value {
    json!({
        "type": "button",
        "action_id": action_id,
        "text": { "type": "plain_text", "text": text }
    })
}

pub fn button_with_value(action_id: &str, text: &str, value: &str) -> Value {
    json!({
        "type": "button",
        "action_id": action_id,
        "text": { "type": "plain_text", "text": text },
        "value": value
    })
}

pub fn link_button(action_id: &str, text: &str, url: &str) -> Value {
    json!({
        "type": "button",
        "action_id": action_id,
        "text": { "type": "plain_text", "text": text },
        "url": url
    })
}

pub fn plain_text_input(action_id: &str) -> Value {
    json!({
        "type": "plain_text_input",
        "action_id": action_id
    })
}

pub fn plain_text_input_multiline(action_id: &str) -> Value {
    json!({
        "type": "plain_text_input",
        "action_id": action_id,
        "multiline": true
    })
}

pub fn plain_text_input_with_placeholder(action_id: &str, placeholder: &str) -> Value {
    json!({
        "type": "plain_text_input",
        "action_id": action_id,
        "placeholder": { "type": "plain_text", "text": placeholder }
    })
}

pub fn datepicker(action_id: &str) -> Value {
    json!({
        "type": "datepicker",
        "action_id": action_id
    })
}

pub fn timepicker(action_id: &str) -> Value {
    json!({
        "type": "timepicker",
        "action_id": action_id
    })
}

pub fn static_select(action_id: &str, placeholder: &str, options: Vec<Value>) -> Value {
    json!({
        "type": "static_select",
        "action_id": action_id,
        "placeholder": { "type": "plain_text", "text": placeholder },
        "options": options
    })
}

pub fn option(text: &str, value: &str) -> Value {
    json!({
        "text": { "type": "plain_text", "text": text },
        "value": value
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button() {
        let b = button("act1", "Click");
        assert_eq!(b["type"], "button");
        assert_eq!(b["action_id"], "act1");
        assert_eq!(b["text"]["text"], "Click");
    }

    #[test]
    fn test_button_with_value() {
        let b = button_with_value("act1", "Go", "val1");
        assert_eq!(b["value"], "val1");
    }

    #[test]
    fn test_datepicker() {
        let d = datepicker("dp1");
        assert_eq!(d["type"], "datepicker");
        assert_eq!(d["action_id"], "dp1");
    }

    #[test]
    fn test_timepicker() {
        let t = timepicker("tp1");
        assert_eq!(t["type"], "timepicker");
        assert_eq!(t["action_id"], "tp1");
    }

    #[test]
    fn test_static_select_with_options() {
        let s = static_select("sel1", "Pick one", vec![
            option("Opt A", "a"),
            option("Opt B", "b"),
        ]);
        assert_eq!(s["type"], "static_select");
        assert_eq!(s["action_id"], "sel1");
        let opts = s["options"].as_array().unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0]["value"], "a");
        assert_eq!(opts[1]["text"]["text"], "Opt B");
    }

    #[test]
    fn test_plain_text_input_with_placeholder() {
        let i = plain_text_input_with_placeholder("inp1", "Type here...");
        assert_eq!(i["type"], "plain_text_input");
        assert_eq!(i["placeholder"]["text"], "Type here...");
    }
}
