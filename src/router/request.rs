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
  let remainder = reference
    .split_once(':')
    .filter(|(prefix, _)| !prefix.contains(['/', '?', '#']))
    .map_or(reference, |(_, remainder)| remainder);
  let path = remainder.strip_prefix("//").map_or(remainder, |authority| {
    &authority[authority.find(['/', '?', '#']).unwrap_or(authority.len())..]
  });

  valid_uri_characters(reference, true)
    && !path.contains(['[', ']'])
    && reference.matches('#').count() <= 1
    && Url::parse("gemini://localhost/")
      .expect("the base URI is valid")
      .join(reference)
      .is_ok()
}
