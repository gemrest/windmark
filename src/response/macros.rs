// This file is part of Windmark <https://github.com/gemrest/windmark>.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2023 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

macro_rules! sync_response {
  ($($name:ident),*) => {
    $(
      /// Trailing commas are not supported at the moment!
      #[macro_export]
      macro_rules! $name {
        ($body:expr /* $(,)? */) => {
          |_: $crate::context::RouteContext| $crate::response::Response::$name($body)
        };
        ($context:ident, $body:expr /* $(,)? */) => {
          |$context: $crate::context::RouteContext| $crate::response::Response::$name($body)
        };
      }
    )*
  };
}

macro_rules! async_response {
  ($($name:ident),*) => {
    $(::paste::paste! {
      /// Trailing commas are not supported at the moment!
      #[macro_export]
      macro_rules! [< $name _async >] {
        ($body:expr /* $(,)? */) => {
          |_: $crate::context::RouteContext| async { $crate::response::Response::$name($body) }
        };
        ($context:ident, $body:expr /* $(,)? */) => {
          |$context: $crate::context::RouteContext| async { $crate::response::Response::$name($body) }
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
  certificate_not_valid,
);

#[cfg(feature = "auto-deduce-mime")]
response!(binary_success_auto);

/// Trailing commas are not supported at the moment!
#[macro_export]
macro_rules! binary_success {
  ($body:expr, $mime:expr) => {
    |_: $crate::context::RouteContext| {
      $crate::response::Response::binary_success($body, $mime)
    }
  };
  ($body:expr) => {{
    #[cfg(not(feature = "auto-deduce-mime"))]
    compile_error!(
      "`binary_success` without a MIME type requires the `auto-deduce-mime` \
       feature to be enabled"
    );

    |_: $crate::context::RouteContext| {
      #[cfg(feature = "auto-deduce-mime")]
      return $crate::response::Response::binary_success_auto($body);

      // Suppress item not found warning
      #[cfg(not(feature = "auto-deduce-mime"))]
      $crate::response::Response::binary_success(
        $body,
        "application/octet-stream",
      )
    }
  }};
  ($context:ident, $body:expr, $mime:expr) => {
    |$context: $crate::context::RouteContext| {
      $crate::response::Response::binary_success($body, $mime)
    }
  };
  ($context:ident, $body:expr) => {{
    #[cfg(not(feature = "auto-deduce-mime"))]
    compile_error!(
      "`binary_success` without a MIME type requires the `auto-deduce-mime` \
       feature to be enabled"
    );

    |$context: $crate::context::RouteContext| {
      #[cfg(feature = "auto-deduce-mime")]
      return $crate::response::Response::binary_success_auto($body);

      // Suppress item not found warning
      #[cfg(not(feature = "auto-deduce-mime"))]
      $crate::response::Response::binary_success(
        $body,
        "application/octet-stream",
      )
    }
  }};
}
