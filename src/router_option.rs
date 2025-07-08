/// Options that can be set for the `Router`
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum RouterOption {
  /// Trim trailing slashes from the URL path
  TrimTrailingSlashes,
}
