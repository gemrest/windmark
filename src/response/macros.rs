macro_rules! sync_response {
  ($($name:ident),*) => {
    $(
      /// This macro does not accept a trailing comma.
      #[macro_export]
      macro_rules! $name {
        ($body:expr) => {
          |_: $crate::context::RouteContext| $crate::response::Response::$name($body)
        };
        ($context:ident, $body:expr) => {
          |$context: $crate::context::RouteContext| $crate::response::Response::$name($body)
        };
      }
    )*
  };
}

macro_rules! async_response {
  ($($name:ident),*) => {
    $(::paste::paste! {
      /// This macro does not accept a trailing comma.
      #[macro_export]
      macro_rules! [< $name _async >] {
        ($body:expr) => {
          |_: $crate::context::RouteContext| async { $crate::response::Response::$name($body) }
        };
        ($context:ident, $body:expr) => {
          |$context: $crate::context::RouteContext| async {
            // Move the request context while preserving borrows of other captures.
            #[allow(clippy::redundant_locals)]
            let $context = $context;

            $crate::response::Response::$name($body)
          }
        };
      }
    })*
  };
}

macro_rules! response {
  ($($name:ident),* $(,)?) => {
    $(
      sync_response!($name);
      async_response!($name);
    )*
  };
}

response!(
  input,
  sensitive_input,
  success,
  temporary_redirect,
  permanent_redirect,
  temporary_failure,
  server_unavailable,
  cgi_error,
  proxy_error,
  slow_down,
  permanent_failure,
  not_found,
  gone,
  proxy_refused,
  bad_request,
  client_certificate_required,
  certificate_not_authorised,
  certificate_not_valid,
);

#[cfg(feature = "auto-deduce-mime")]
response!(binary_success_auto);

/// Build a binary response handler with an explicit or inferred MIME type.
///
/// Automatic inference requires Windmark's `auto-deduce-mime` feature. Use
/// `binary_success!(context => body)` to bind a context when inferring MIME;
/// the two-expression `(body, mime)` form always specifies MIME explicitly.
#[macro_export]
macro_rules! binary_success {
  ($body:expr, $mime:expr $(,)?) => {
    |_: $crate::context::RouteContext| {
      $crate::response::Response::binary_success($body, $mime)
    }
  };
  ($body:expr $(,)?) => {
    |_: $crate::context::RouteContext| {
      $crate::response::Response::binary_success_auto($body)
    }
  };
  ($context:ident, $body:expr, $mime:expr $(,)?) => {
    |$context: $crate::context::RouteContext| {
      $crate::response::Response::binary_success($body, $mime)
    }
  };
  ($context:ident => $body:expr $(,)?) => {
    |$context: $crate::context::RouteContext| {
      $crate::response::Response::binary_success_auto($body)
    }
  };
}
