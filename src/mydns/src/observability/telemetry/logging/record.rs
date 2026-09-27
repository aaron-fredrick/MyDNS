use std::collections::HashMap;

use chrono::{DateTime, Utc};

// https://opentelemetry.io/docs/specs/otel/logs/data-model/
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub observed_timestamp: Option<DateTime<Utc>>,

    // Trace Context Fields (https://opentelemetry.io/docs/specs/otel/logs/data-model/#trace-context-fields)
    pub trace_id: Option<[u8; 16]>, // https://www.w3.org/TR/trace-context/#trace-id
    pub span_id: Option<[u8; 8]>,   // https://www.w3.org/TR/trace-context/#span-id
    pub trace_flags: Option<u8>, // TODO: Add 8-bit tracing flags for sampling and trace-level recommendations. (https://www.w3.org/TR/trace-context/#trace-flags)

    // Security Fields (https://opentelemetry.io/docs/specs/otel/logs/data-model/#security-fields)
    // * OpenTelemetry recommends mapping SeverityNumber to SeverityText and defining short, stable severity names.
    // * See: https://opentelemetry.io/docs/specs/otel/logs/data-model/#field-severitynumber
    pub severity_number: Option<u8>,
    pub severity_text: Option<String>,

    pub body: Option<String>, // ? OTel Body supports AnyValue; revisit if structured bodies are needed.
    pub resource: Option<Resource>,
    pub instrumentation_scope: Option<InstrumentationScope>,
    pub attributes: HashMap<String, AnyValue>, // ? TODO: Align with OTel Attribute Collections and AnyValue types: https://opentelemetry.io/docs/specs/otel/common/#attribute-collections
    pub event_name: Option<String>
}

pub struct Resource {
    pub attributes: HashMap<String, AnyValue>,
    pub schema_url: Option<String>,
}

// TODO: Review OpenTelemetry InstrumentationScope and define this structure properly. (https://opentelemetry.io/docs/specs/otel/common/instrumentation-scope/)
pub struct InstrumentationScope {
    pub name: String,
    pub version: Option<String>,
    pub schema_url: Option<String>,
    pub attributes: HashMap<String, AnyValue>,
}