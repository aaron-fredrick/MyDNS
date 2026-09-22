use hickory_proto::rr::Name;

pub(crate) fn test_name() -> Name {
    "example.com.".parse().expect("valid test name")
}
