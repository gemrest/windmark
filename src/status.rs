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

#[derive(Copy, Clone)]
pub enum Code {
  Input               = 10,
  SensitiveInput      = 11,
  Success             = 20,
  TemporaryRedirect   = 30,
  PermanentRedirect   = 31,
  TemporaryFailure    = 40,
  ServerUnavailable   = 41,
  CGIError            = 42,
  ProxyError          = 43,
  SlowDown            = 44,
  PermanentFailure    = 50,
  NotFound            = 51,
  Gone                = 52,
  ProxyRefused        = 53,
  BadRequest          = 59,
  ClientCertificateRequired = 60,
  CertificateNotAuthorised = 61,
  CertificateNotValid = 62,
}
