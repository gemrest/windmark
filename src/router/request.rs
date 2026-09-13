use url::Url;

pub(super) fn parse_uri(request: &str) -> Result<Url, String> {
  if !valid_uri_characters(request, false) {
    return Err("request URI contains invalid syntax".into());
  }

  if let Some((_, remainder)) = request.split_once(':') {
    let path_and_query = if let Some(authority) = remainder.strip_prefix("//") {
      let end = authority.find(['/', '?']).unwrap_or(authority.len());

      if authority[..end].contains('@') {
        return Err("request URI must not contain userinfo".into());
      }

      &authority[end..]
    } else {
      remainder
    };

    if path_and_query.contains(['[', ']']) {
      return Err("request URI contains an unescaped bracket".into());
    }
  }

  let url = Url::parse(request).map_err(|error| error.to_string())?;

  if !url.username().is_empty() || url.password().is_some() {
    return Err("request URI must not contain userinfo".into());
  }

  if url.scheme() == "gemini" && url.host_str().is_none_or(str::is_empty) {
    return Err("Gemini request URI requires a host".into());
  }

  Ok(url)
}

fn valid_uri_characters(request: &str, allow_fragment: bool) -> bool {
  let bytes = request.as_bytes();
  let mut position = 0;

  while position < bytes.len() {
    let byte = bytes[position];

    if byte == b'%' {
      if bytes
        .get(position + 1..position + 3)
        .is_none_or(|digits| !digits.iter().all(u8::is_ascii_hexdigit))
      {
        return false;
      }

      position += 3;
    } else if byte.is_ascii_alphanumeric()
      || b":/?[]@!$&'()*+,;=-._~".contains(&byte)
      || (allow_fragment && byte == b'#')
    {
      position += 1;
    } else {
      return false;
    }
  }

  true
}

pub(super) fn valid_uri_reference(reference: &str) -> bool {
  let first_segment =
    reference.split(['/', '?', '#']).next().unwrap_or_default();

  if let Some((scheme, _)) = first_segment.split_once(':') {
    if !scheme
      .as_bytes()
      .first()
      .is_some_and(u8::is_ascii_alphabetic)
      || !scheme
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(&byte))
    {
      return false;
    }
  }

  let remainder = reference
    .split_once(':')
    .filter(|(prefix, _)| !prefix.contains(['/', '?', '#']))
    .map_or(reference, |(_, remainder)| remainder);
  let path = if let Some(authority) = remainder.strip_prefix("//") {
    let end = authority.find(['/', '?', '#']).unwrap_or(authority.len());

    if !valid_authority(&authority[..end]) {
      return false;
    }

    &authority[end..]
  } else {
    remainder
  };

  valid_uri_characters(reference, true)
    && !path.contains(['[', ']'])
    && reference.matches('#').count() <= 1
}

fn valid_authority(authority: &str) -> bool {
  let host_and_port = if let Some((userinfo, host)) = authority.split_once('@')
  {
    if userinfo.contains(['[', ']']) {
      return false;
    }

    host
  } else {
    authority
  };

  if let Some(literal) = host_and_port.strip_prefix('[') {
    let Some((address, suffix)) = literal.split_once(']') else {
      return false;
    };
    let valid_address = address.strip_prefix(['v', 'V']).map_or_else(
      || address.parse::<std::net::Ipv6Addr>().is_ok(),
      |future| {
        future.split_once('.').is_some_and(|(version, address)| {
          !version.is_empty()
            && version.bytes().all(|byte| byte.is_ascii_hexdigit())
            && !address.is_empty()
            && address.bytes().all(|byte| {
              byte.is_ascii_alphanumeric()
                || b"-._~!$&'()*+,;=:".contains(&byte)
            })
        })
      },
    );

    return valid_address
      && (suffix.is_empty()
        || suffix
          .strip_prefix(':')
          .is_some_and(|port| port.bytes().all(|byte| byte.is_ascii_digit())));
  }

  let (host, port) = host_and_port
    .rsplit_once(':')
    .unwrap_or((host_and_port, ""));

  !host.contains(['@', ':', '[', ']'])
    && port.bytes().all(|byte| byte.is_ascii_digit())
}
