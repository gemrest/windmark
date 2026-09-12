use std::{path::PathBuf, process::Command};

#[test]
fn consumer_api_matches_expected_behavior() {
  let manifest =
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/consumer/Cargo.toml");
  let target = std::env::var_os("CARGO_TARGET_DIR")
    .map_or_else(
      || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"),
      PathBuf::from,
    )
    .join("compatibility-consumer");
  #[cfg(feature = "tokio")]
  let runtime = "tokio";
  #[cfg(feature = "async-std")]
  let runtime = "async-std";
  let cases = [
    ("invalid_response_fields", "private", "auto-deduce-mime"),
    ("valid", "", "auto-deduce-mime"),
    ("automatic_mime", "", "auto-deduce-mime"),
    ("binary_context", "", "auto-deduce-mime"),
    ("async_context", "", "auto-deduce-mime"),
    ("automatic_mime_disabled", "binary_success_auto", ""),
    (
      "invalid_router_item",
      "requires a struct or impl block",
      "auto-deduce-mime",
    ),
    (
      "invalid_route_item",
      "requires a function",
      "auto-deduce-mime",
    ),
    (
      "invalid_router_fields",
      "requires a struct with named fields or a unit struct",
      "auto-deduce-mime",
    ),
    ("invalid_router_syntax", "expected", "auto-deduce-mime"),
    ("invalid_route_syntax", "expected", "auto-deduce-mime"),
    ("visibility", "", "auto-deduce-mime"),
    (
      "visibility_private",
      "struct `Capsule` is private",
      "auto-deduce-mime",
    ),
    (
      "visibility_restricted",
      "struct `Capsule` is private",
      "auto-deduce-mime",
    ),
    ("index_name", "", "auto-deduce-mime"),
    ("router_declarations", "", "auto-deduce-mime"),
    (
      "invalid_router_duplicate",
      "duplicate field initialiser",
      "auto-deduce-mime",
    ),
    (
      "invalid_router_unknown",
      "unknown field initialiser",
      "auto-deduce-mime",
    ),
    (
      "invalid_router_reserved",
      "reserved by Rossweisse",
      "auto-deduce-mime",
    ),
    (
      "invalid_route_argument",
      "only supported route argument",
      "auto-deduce-mime",
    ),
  ];

  for (name, diagnostic, extra_features) in cases {
    let features = if extra_features.is_empty() {
      runtime.to_owned()
    } else {
      format!("{runtime},{extra_features}")
    };
    let output = Command::new(env!("CARGO"))
      .arg(if diagnostic.is_empty() {
        "run"
      } else {
        "check"
      })
      .args(["--quiet", "--manifest-path"])
      .arg(&manifest)
      .arg("--target-dir")
      .arg(&target)
      .args(["--features", &features, "--bin", name])
      .output()
      .unwrap();
    let errors = String::from_utf8_lossy(&output.stderr);

    if name.starts_with("invalid_") {
      assert!(
        !errors.contains("custom attribute panicked"),
        "{name}: {errors}"
      );
      assert!(errors.contains(&format!("{name}.rs")), "{name}: {errors}");
    }

    if diagnostic.is_empty() {
      assert!(output.status.success(), "{name}: {errors}");
    } else {
      assert!(
        !output.status.success() && errors.contains(diagnostic),
        "{name}: {errors}"
      );
    }
  }
}
