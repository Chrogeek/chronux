use semver::{Version, VersionReq};

pub const CHRONUX_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn is_compatible(check: &str) -> bool {
    let c = Version::parse(check).unwrap();
    let req = VersionReq::parse(&format!("^{}", CHRONUX_VERSION)).unwrap();
    req.matches(&c)
}
