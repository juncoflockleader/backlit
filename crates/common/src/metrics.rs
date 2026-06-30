use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldValue<'a> {
    Bool(bool),
    Str(&'a str),
    U64(u64),
}

pub fn event_json(name: &str, fields: &[(&str, FieldValue<'_>)]) -> String {
    let mut out = String::from("{\"event\":");
    push_json_string(&mut out, name);

    for (key, value) in fields {
        out.push(',');
        push_json_string(&mut out, key);
        out.push(':');
        push_field_value(&mut out, *value);
    }

    out.push('}');
    out
}

fn push_field_value(out: &mut String, value: FieldValue<'_>) {
    match value {
        FieldValue::Bool(value) => out.push_str(if value { "true" } else { "false" }),
        FieldValue::Str(value) => push_json_string(out, value),
        FieldValue::U64(value) => {
            let _ = write!(out, "{value}");
        }
    }
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');

    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }

    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{event_json, FieldValue};

    #[test]
    fn writes_json_metric_line() {
        let line = event_json(
            "compositor.start",
            &[
                ("backend", FieldValue::Str("headless")),
                ("elapsed_ms", FieldValue::U64(4)),
                ("smoke_test", FieldValue::Bool(true)),
            ],
        );

        assert_eq!(
            line,
            "{\"event\":\"compositor.start\",\"backend\":\"headless\",\"elapsed_ms\":4,\"smoke_test\":true}"
        );
    }

    #[test]
    fn escapes_strings() {
        let line = event_json("quote", &[("value", FieldValue::Str("\"back\\lit\"\n"))]);

        assert_eq!(
            line,
            "{\"event\":\"quote\",\"value\":\"\\\"back\\\\lit\\\"\\n\"}"
        );
    }
}
