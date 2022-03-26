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

pub struct Header {
  status: crate::status::Code,
  meta:   String,
}
impl ToString for Header {
  fn to_string(&self) -> String {
    format!("{} {}\r\n", self.status as u8, self.meta)
  }
}

pub enum Response {
  Input(String),
  SensitiveInput(String),
  Success(String),
  NotFound(String),
  TemporaryFailure(String),
  PermanentFailure(String),
}

pub(crate) fn to_value_set_status(
  response: Response,
  status: &mut i32,
) -> String {
  match response {
    Response::Input(value) => {
      *status = 10;

      value
    }
    Response::SensitiveInput(value) => {
      *status = 11;

      value
    }
    Response::Success(value) => {
      *status = 20;

      value
    }
    Response::TemporaryFailure(value) => {
      *status = 40;

      value
    }
    Response::NotFound(value) => {
      *status = 51;

      value
    }
    Response::PermanentFailure(value) => {
      *status = 50;

      value
    }
  }
}
