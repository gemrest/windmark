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

use std::collections::HashMap;

use matchit::Params;
use openssl::x509::X509;
use url::Url;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct HookContext {
  pub peer_address: Option<std::net::SocketAddr>,
  pub url:          Url,
  pub parameters:   Option<HashMap<String, String>>,
  pub certificate:  Option<X509>,
}

impl HookContext {
  #[must_use]
  pub fn new(
    peer_address: std::io::Result<std::net::SocketAddr>,
    url: Url,
    parameters: Option<Params<'_, '_>>,
    certificate: Option<X509>,
  ) -> Self {
    Self {
      peer_address: peer_address.ok(),
      url,
      parameters: parameters.map(|p| crate::utilities::params_to_hashmap(&p)),
      certificate,
    }
  }
}
