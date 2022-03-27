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

pub enum Response {
  Input(String),
  SensitiveInput(String),
  Success(String),
  TemporaryFailure(String),
  ServerUnavailable(String),
  CGIError(String),
  ProxyError(String),
  SlowDown(String),
  PermanentFailure(String),
  NotFound(String),
  Gone(String),
  ProxyRefused(String),
  BadRequest(String),
  ClientCertificateRequired(String),
  CertificateNotAuthorised(String),
  CertificateNotValid(String),
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
    Response::ServerUnavailable(value) => {
      *status = 41;

      value
    }
    Response::CGIError(value) => {
      *status = 42;

      value
    }
    Response::ProxyError(value) => {
      *status = 43;

      value
    }
    Response::SlowDown(value) => {
      *status = 44;

      value
    }
    Response::PermanentFailure(value) => {
      *status = 50;

      value
    }
    Response::NotFound(value) => {
      *status = 51;

      value
    }
    Response::Gone(value) => {
      *status = 52;

      value
    }
    Response::ProxyRefused(value) => {
      *status = 53;

      value
    }
    Response::BadRequest(value) => {
      *status = 59;

      value
    }
    Response::ClientCertificateRequired(value) => {
      *status = 60;

      value
    }
    Response::CertificateNotAuthorised(value) => {
      *status = 61;

      value
    }
    Response::CertificateNotValid(value) => {
      *status = 62;

      value
    }
  }
}
