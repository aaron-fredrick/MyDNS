use crate::config::{ResolverMode, ResolverPriority};
use crate::mydns::dns::upstream::UpstreamResolver;

#[test]
fn from_config_populates_recursive_defaults() {
    let resolver = UpstreamResolver::from_config(
        ResolverMode::Recursive,
        ResolverPriority::CloudflareFirst,
        "1.1.1.1:53".parse().unwrap(),
        None,
        Vec::new(),
    )
    .expect("resolver should build");

    assert_eq!(resolver.mode, ResolverMode::Recursive);
    assert_eq!(resolver.priority, ResolverPriority::CloudflareFirst);
    assert!(!resolver.root_hints.is_empty());
}

#[test]
fn from_config_preserves_forwarding_priority() {
    let resolver = UpstreamResolver::from_config(
        ResolverMode::Forwarding,
        ResolverPriority::RouterFirst,
        "1.1.1.1:53".parse().unwrap(),
        None,
        vec!["192.0.2.1:53".parse().unwrap()],
    )
    .expect("resolver should build");

    assert_eq!(resolver.mode, ResolverMode::Forwarding);
    assert_eq!(resolver.priority, ResolverPriority::RouterFirst);
    assert_eq!(resolver.root_hints, vec!["192.0.2.1:53".parse().unwrap()]);
}
