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

use std::{future::Future, pin::Pin};

use crate::{context::RouteContext, Response};

#[allow(clippy::module_name_repetitions)]
pub trait RouteResponse: Send + Sync {
  fn call(
    &mut self,
    context: RouteContext<'_>,
  ) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

impl<T, F> RouteResponse for T
where
  T: FnMut(RouteContext<'_>) -> F + Send + Sync,
  F: Future<Output = Response> + Send + 'static,
{
  fn call(
    &mut self,
    context: RouteContext<'_>,
  ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
    Box::pin((*self)(context))
  }
}