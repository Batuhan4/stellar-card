use crate::cli::OutputFormat;
use crate::error::AppError;
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Serialize)]
struct Meta {
    request_id: String,
    timestamp: String,
}

fn meta() -> Meta {
    Meta {
        request_id: format!("req_{}", Uuid::new_v4().simple()),
        timestamp: Utc::now().to_rfc3339(),
    }
}

pub fn emit_success<T: Serialize>(format: &OutputFormat, data: &T) -> Result<(), AppError> {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "data": data,
                    "meta": meta(),
                }))?
            );
        }
        OutputFormat::Table => {
            println!("{}", value_to_table(&serde_json::to_value(data)?));
        }
        OutputFormat::Plain => {
            println!("{}", value_to_plain(&serde_json::to_value(data)?));
        }
    }
    Ok(())
}

pub fn emit_error(format: &OutputFormat, error: &AppError) -> i32 {
    let payload = json!({
        "ok": false,
        "error": {
            "code": error.code.as_str(),
            "message": error.message,
            "details": error.details,
            "suggestion": error.suggestion,
        },
        "meta": meta(),
    });
    match format {
        OutputFormat::Json => eprintln!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
        ),
        OutputFormat::Table | OutputFormat::Plain => eprintln!("{}", error.message),
    }
    error.code.exit_code()
}

fn value_to_table(value: &Value) -> String {
    let mut lines = Vec::new();
    flatten(value, "", &mut lines, ": ");
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

fn value_to_plain(value: &Value) -> String {
    let mut lines = Vec::new();
    flatten(value, "", &mut lines, "=");
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

fn flatten(value: &Value, prefix: &str, out: &mut Vec<String>, separator: &str) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let next = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten(value, &next, out, separator);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let next = format!("{}[{}]", prefix, index);
                flatten(item, &next, out, separator);
            }
        }
        Value::Null => out.push(format!("{}{separator}null", prefix)),
        Value::Bool(v) => out.push(format!("{}{separator}{}", prefix, v)),
        Value::Number(v) => out.push(format!("{}{separator}{}", prefix, v)),
        Value::String(v) => out.push(format!("{}{separator}{}", prefix, v)),
    }
}
