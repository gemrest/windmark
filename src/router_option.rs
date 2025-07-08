/// Options that can be set for the `Router`
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum RouterOption {
  /// If enabled, removes a trailing slash from the request URL path if a route
  /// exists for the path without the slash (e.g., `/foo/` becomes `/foo`).
  RemoveExtraTrailingSlash,
  /// If enabled, adds a trailing slash to the request URL path if a route
  /// exists for the path with the slash (e.g., `/foo` becomes `/foo/`).
  AddMissingTrailingSlash,
}
