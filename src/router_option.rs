/// These options configure the `Router`.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum RouterOption {
  /// If enabled, removes a trailing slash from the request URL path if a route
  /// exists for the path without the slash (e.g., `/foo/` becomes `/foo`).
  RemoveExtraTrailingSlash,
  /// If enabled, adds a trailing slash to the request URL path if a route
  /// exists for the path with the slash (e.g., `/foo` becomes `/foo/`).
  AddMissingTrailingSlash,
  /// If enabled, the router ignores ASCII case in static route segments.
  /// Parameter names and captured values retain their original spelling.
  /// Routes that conflict under this matching rule cannot be registered
  /// together while the option is enabled.
  AllowCaseInsensitiveLookup,
  /// If enabled, module hooks may run concurrently across requests, including
  /// multiple invocations of the same module. By default, each synchronous or
  /// asynchronous hook phase runs exclusively across its module collection.
  ///
  /// This option is shared by cloned routers, including running servers.
  /// Changes apply when a hook phase selects its scheduling mode. An exclusive
  /// phase waits for any concurrent phases to finish before invoking hooks.
  /// Poisoned synchronous modules are skipped in either mode.
  AllowConcurrentModules,
}
