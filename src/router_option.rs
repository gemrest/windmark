/// Options that can be set for the `Router`
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum RouterOption {
  /// Trim trailing slashes from the URL path if it is present and a route
  /// match exists
  TrimTrailingSlashes,
  /// Add a trailing slash to the URL path if it is missing and a route
  /// match exists
  AddMissingTrailingSlash,
}
