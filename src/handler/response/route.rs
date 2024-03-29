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

use async_trait::async_trait;

use crate::{context::RouteContext, response::Response};

#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait RouteResponse: Send + Sync {
  async fn call(&mut self, context: RouteContext) -> Response;
}

#[async_trait]
impl<T, F> RouteResponse for T
where
  T: FnMut(RouteContext) -> F + Send + Sync,
  F: std::future::Future<Output = Response> + Send + 'static,
{
  async fn call(&mut self, context: RouteContext) -> Response {
    (*self)(context).await
  }
}
