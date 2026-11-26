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
    ("invalid_response_fields", "private"),
    ("valid", ""),
    ("automatic_mime", ""),
    ("binary_context", ""),
    ("async_context", ""),
    ("automatic_mime_disabled", "binary_success_auto"),
    ("invalid_router_item", "requires a struct or impl block"),
    ("invalid_route_item", "requires a function"),
    (
      "invalid_router_fields",
      "requires a struct with named fields or a unit struct",
    ),
    ("invalid_router_syntax", "expected"),
    ("invalid_route_syntax", "expected"),
    ("visibility", ""),
    ("visibility_private", "struct `Capsule` is private"),
    ("visibility_restricted", "struct `Capsule` is private"),
    ("index_name", ""),
    ("router_declarations", ""),
    ("invalid_router_duplicate", "duplicate field initialiser"),
    ("invalid_router_unknown", "unknown field initialiser"),
    ("invalid_router_reserved", "reserved by Rossweisse"),
    ("invalid_route_argument", "only supported route argument"),
  ];

  for (name, diagnostic) in cases {
    let features = if name == "automatic_mime_disabled" {
      runtime.to_owned()
    } else {
      format!("{runtime},auto-deduce-mime")
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
