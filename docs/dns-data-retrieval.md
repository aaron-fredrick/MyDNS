# DNS Data Retrieval and Lookup Architecture

Branch: `production-readiness`

## Purpose

This note records the DNS-specific data retrieval methods that should be considered as part of MyDNS production-readiness work. DNS lookup is a high-volume, latency-sensitive operation, so the persistent database should not automatically become the per-request lookup path.

## Retrieval methods to account for

### 1. Exact-match indexed lookup

The fundamental authoritative lookup is:

```text
FQDN + record type + class -> Resource Records
```

A relational persistence layer can support this with a B-tree index over the normalized DNS name, with the record type/class included as appropriate.

Target characteristics:

- Deterministic exact lookup.
- Proper normalization of case and trailing dots.
- Composite indexing where query patterns justify it.
- No full-table scans on the DNS request path.

### 2. In-memory hash lookup

Because most DNS requests are exact-name lookups, MyDNS should support an in-memory representation optimized for very low-latency reads.

Conceptually:

```text
DNS query
   -> in-memory lookup
      -> hit: answer
      -> miss: lower-level resolution path
```

Hash-based indexing provides approximately O(1) average lookup for exact keys and should be considered for the hot path.

A suitable logical key is:

```text
(normalized_name, record_type, record_class)
```

### 3. Trie / radix-tree lookup

DNS names are hierarchical, so a trie or compressed radix tree should be considered for operations that require walking domain ancestry rather than only exact matching.

Example:

```text
com
└── example
    ├── www
    ├── api
    ├── mail
    └── ns1
```

A radix-tree representation is particularly relevant to:

- Closest enclosing zone lookup.
- Authoritative zone selection.
- Delegation boundaries.
- Wildcard handling.
- Hierarchical name traversal.

### 4. Hierarchical / longest-ancestor matching

For a query such as:

```text
host.service.example.com.
```

The resolver/authoritative engine may need to determine the closest applicable zone or ancestor rather than simply finding an exact record.

This is conceptually similar to longest-prefix matching, but applied to DNS name labels rather than IP prefixes.

The implementation must preserve DNS label boundaries and must not treat arbitrary string prefixes as valid domain matches.

### 5. Positive and negative caching

Caching is a first-class retrieval mechanism, not merely an optimization.

MyDNS should support:

- Positive response caching.
- Negative caching.
- TTL-based expiration.
- Pruning of expired entries.
- Cache invalidation when authoritative data changes.
- Safe concurrent access.
- Persistence/recovery where required by the product design.

Negative caching must distinguish DNS outcomes such as NXDOMAIN from successful names with no record of the requested type (NODATA).

## Recommended production architecture

The persistent database should be treated as the durable source of truth, while the DNS request path should use an optimized read representation.

```text
                    ┌───────────────────┐
                    │   DNS Request     │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │   Lookup Engine   │
                    └─────────┬─────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
             ┌──────────────┐    ┌──────────────┐
             │ DNS Cache /  │    │ Zone / Name  │
             │ Hash Index   │    │ Index        │
             └──────┬───────┘    └──────┬───────┘
                    │                   │
                    └─────────┬─────────┘
                              │ miss / reload
                              ▼
                    ┌───────────────────┐
                    │ Persistent DB     │
                    │ (source of truth) │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ Index / Cache     │
                    │ rebuild or update │
                    └───────────────────┘
```

The exact implementation may use one or more of these structures. The important production-readiness requirement is that every DNS request must not synchronously execute a relational database query simply because the record exists in persistent storage.

## Authoritative server vs recursive resolver

The retrieval model depends on MyDNS's role.

### Authoritative DNS

The primary durable model is zone/record data:

```text
Zones
  -> example.com.

Records
  -> www.example.com. A 192.0.2.10
  -> mail.example.com. MX ...
```

The hot path should be optimized for authoritative record retrieval and zone ownership checks.

### Recursive resolver

The dominant model is a cache containing learned responses:

```text
google.com. A
example.com. NS
example.org. AAAA
```

The resolver must additionally handle cache freshness, negative responses, upstream selection/failure, and iterative/recursive resolution behavior as applicable.

## Production-readiness requirements

Before V1.0.0, the lookup layer should be evaluated for:

- Exact lookup correctness.
- Case-insensitive DNS-name normalization.
- Trailing-dot normalization.
- DNS label-boundary correctness.
- Zone/ancestor selection.
- Record-type/class separation.
- Cache hit/miss behavior.
- Positive and negative caching.
- TTL expiration.
- Concurrent cache access.
- Cache invalidation after record changes.
- Database-to-index synchronization.
- Startup index/cache loading.
- Safe behavior after database failure.
- Memory growth under large zones and high query rates.
- Lookup latency under concurrent load.
- No unbounded per-request allocations or retained request history.
- Deterministic behavior during reloads and shutdown.

## Design principle

Use the database for **durability and authoritative state**, and use purpose-built in-memory indexes/caches for the **DNS read path**. Select the specific data structure based on the lookup operation: B-tree indexes for persistent exact queries, hash tables for hot exact-match retrieval, radix/trie structures for hierarchical DNS-name operations, and TTL-aware caches for repeated responses.
