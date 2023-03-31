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

use crate::{
  response::Response,
  returnable,
  returnable::{CallbackContext, RouteContext},
};

pub type RouteResponse =
  Box<dyn FnMut(RouteContext<'_>) -> Response + Send + Sync>;
pub type ErrorResponse =
  Box<dyn FnMut(returnable::ErrorContext<'_>) -> Response + Send + Sync>;
pub type Callback = Box<dyn FnMut(CallbackContext<'_>) + Send + Sync>;
pub type CleanupCallback =
  Box<dyn FnMut(CallbackContext<'_>, &mut Response) + Send + Sync>;
pub type Partial = Box<dyn FnMut(RouteContext<'_>) -> String + Send + Sync>;
