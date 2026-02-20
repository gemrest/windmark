//! `cargo run --example fix_path --features response-macros`

use windmark::router_option::RouterOption;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .add_options(&[
      RouterOption::RemoveExtraTrailingSlash,
      RouterOption::AddMissingTrailingSlash,
      RouterOption::AllowCaseInsensitiveLookup,
    ])
    .mount(
      "/close",
      windmark::success!("Visit '/close/'; you should be close enough!"),
    )
    .run()
    .await
}
