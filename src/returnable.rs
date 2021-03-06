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

use matchit::Params;
use openssl::x509::X509;
use tokio::net::TcpStream;
use url::Url;

pub struct RouteContext<'a> {
  pub tcp:         &'a TcpStream,
  pub url:         &'a Url,
  pub params:      &'a Params<'a, 'a>,
  pub certificate: &'a Option<X509>,
}
impl<'a> RouteContext<'a> {
  pub const fn new(
    tcp: &'a TcpStream,
    url: &'a Url,
    params: &'a Params<'a, 'a>,
    certificate: &'a Option<X509>,
  ) -> Self {
    Self {
      tcp,
      url,
      params,
      certificate,
    }
  }
}

pub struct ErrorContext<'a> {
  pub tcp:         &'a TcpStream,
  pub url:         &'a Url,
  pub certificate: &'a Option<X509>,
}
impl<'a> ErrorContext<'a> {
  pub const fn new(
    tcp: &'a TcpStream,
    url: &'a Url,
    certificate: &'a Option<X509>,
  ) -> Self {
    Self {
      tcp,
      url,
      certificate,
    }
  }
}

pub struct CallbackContext<'a> {
  pub tcp:         &'a TcpStream,
  pub url:         &'a Url,
  pub params:      Option<&'a Params<'a, 'a>>,
  pub certificate: &'a Option<X509>,
}
impl<'a> CallbackContext<'a> {
  pub const fn new(
    tcp: &'a TcpStream,
    url: &'a Url,
    params: Option<&'a Params<'a, 'a>>,
    certificate: &'a Option<X509>,
  ) -> Self {
    Self {
      tcp,
      url,
      params,
      certificate,
    }
  }
}
