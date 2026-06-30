use backlit_common::metrics::{event_json, FieldValue};

fn main() {
    println!(
        "{}",
        event_json(
            "portal_backend.stub_ready",
            &[
                ("screenshot", FieldValue::Bool(false)),
                ("screencast", FieldValue::Bool(false)),
                ("file_chooser", FieldValue::Bool(false)),
            ],
        )
    );
}
