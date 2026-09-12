use std::{path::PathBuf, process::Command};

#[test]
fn consumer_api_and_macro_forms_match_expected_behavior() {
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
    ("valid", "", true),
    ("automatic_mime", "requires the `auto-deduce-mime`", false),
    ("binary_context", "cannot find value `context`", false),
    (
      "async_context",
      "cannot return reference to function parameter",
      false,
    ),
    ("visibility", "", false),
    ("visibility_private", "struct `Capsule` is private", false),
    (
      "visibility_restricted",
      "struct `Capsule` is private",
      false,
    ),
    (
      "index_name",
      "no function or associated item named `index`",
      false,
    ),
  ];

  for (name, diagnostic, automatic_mime) in cases {
    let features = if automatic_mime {
      format!("{runtime},auto-deduce-mime")
    } else {
      runtime.to_owned()
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
