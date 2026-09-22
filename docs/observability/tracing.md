# Tracing

## Goal

Tracing explains **how a request moved through MyDNS** and where time was spent.

The project already uses `tracing` and `tracing-subscriber`, and Samply is attached as a subscriber layer. The next step is consistent span instrumentation rather than adding ad-hoc timers everywhere.

## Trace hierarchy

### Process

```text
mydns
├── dns.server
├── http.server
└── background
```

### DNS request

```text
dns.request
├── parse
├── zone_lookup
├── blocklist_lookup
├── cache_lookup
├── local_record_lookup
├── upstream.resolve
│   ├── upstream.request
│   └── retry
└── response
```

Only spans that represent meaningful work should be created. Do not create a span for every trivial helper.

### HTTP request

```text
http.request
├── authentication
├── handler
│   └── database operation
└── response
```

### Database

Use semantic child spans:

- `db.connect`
- `db.migrate`
- `db.records.list`
- `db.records.create`
- `db.records.update`
- `db.records.delete`
- `db.zones.load`
- `db.settings.get`

Do not record raw SQL text as a normal span attribute.

## Span attributes

Safe bounded attributes:

- component
- operation
- protocol
- DNS record type
- response code
- outcome
- upstream identifier
- HTTP method
- normalized route
- status code

Avoid:

- full Authorization values
- passwords
- JWTs
- raw SQL parameters
- unbounded domain/client identifiers as routine attributes

## Error semantics

A span should record an error when an operation fails.

The span should also carry a bounded outcome such as:

- success
- timeout
- refused
- not_found
- validation_error
- internal_error

Do not rely solely on log messages to understand failures.

## Sampling

Recommended initial policy:

- lifecycle/background spans: retain
- errors: retain
- slow requests: retain
- normal high-volume DNS requests: sampled
- normal API requests: sampled at a higher rate than DNS if volume permits

Sampling must not remove metrics.

## Trace/log correlation

When a tracing span exists, structured logs emitted inside that span should inherit the trace context.

This allows:

```text
metric anomaly
   -> timestamp/window
   -> trace
   -> structured log
   -> exact failing operation
```

## Performance

Tracing must be low overhead:

- avoid formatting expensive fields unless emitted
- avoid recording entire DNS messages
- avoid high-cardinality attributes
- sample high-volume successful DNS requests
- keep error/slow-request visibility high
