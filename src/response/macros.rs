// This file is part of Windmark <https://github.com/gemrest/windmark>.
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
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
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

macro_rules! response {
  ($($name:tt),*) => {
    $(
      /// Trailing commas are not supported at the moment!
      #[macro_export]
      macro_rules! $name {
        ($body:expr /* $(,)? */) => {
          |_: ::windmark::context::RouteContext<'_>| ::windmark::Response::$name($body)
        };
        ($context:ident, $body:expr /* $(,)? */) => {
          |$context: ::windmark::context::RouteContext<'_>| ::windmark::Response::$name($body)
        };
      }
    )*
  };
}

response!(input);
response!(sensitive_input);
response!(success);
#[cfg(feature = "auto-deduce-mime")]
response!(binary_success_auto);
response!(temporary_redirect);
response!(permanent_redirect);
response!(temporary_failure);
response!(server_unavailable);
response!(cgi_error);
response!(proxy_error);
response!(slow_down);
response!(permanent_failure);
response!(not_found);
response!(gone);
response!(proxy_refused);
response!(bad_request);
response!(client_certificate_required);
response!(certificate_not_valid);

/// Trailing commas are not supported at the moment!
#[macro_export]
macro_rules! binary_success {
  ($body:expr, $mime:expr) => {
    |_: ::windmark::context::RouteContext<'_>| {
      ::windmark::Response::binary_success($body, $mime)
    }
  };
  ($body:expr) => {{
    #[cfg(not(feature = "auto-deduce-mime"))]
    compile_error!(
      "`binary_success` without a MIME type requires the `auto-deduce-mime` \
       feature to be enabled"
    );

    |_: ::windmark::context::RouteContext<'_>| {
      #[cfg(feature = "auto-deduce-mime")]
      return ::windmark::Response::binary_success_auto($body);

      // Suppress item not found warning
      #[cfg(not(feature = "auto-deduce-mime"))]
      ::windmark::Response::binary_success($body, "application/octet-stream")
    }
  }};
  ($context:ident, $body:expr, $mime:expr) => {
    |$context: ::windmark::context::RouteContext<'_>| {
      ::windmark::Response::binary_success($body, $mime)
    }
  };
  ($context:ident, $body:expr) => {{
    #[cfg(not(feature = "auto-deduce-mime"))]
    compile_error!(
      "`binary_success` without a MIME type requires the `auto-deduce-mime` \
       feature to be enabled"
    );

    |$context: ::windmark::context::RouteContext<'_>| {
      #[cfg(feature = "auto-deduce-mime")]
      return ::windmark::Response::binary_success_auto($body);

      // Suppress item not found warning
      #[cfg(not(feature = "auto-deduce-mime"))]
      ::windmark::Response::binary_success($body, "application/octet-stream")
    }
  }};
}
