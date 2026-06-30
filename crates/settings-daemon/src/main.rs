use backlit_common::metrics::{event_json, FieldValue};

fn main() {
    println!(
        "{}",
        event_json(
            "settings_daemon.stub_ready",
            &[
                ("display", FieldValue::Bool(false)),
                ("input", FieldValue::Bool(false)),
                ("power", FieldValue::Bool(false)),
            ],
        )
    );
}
