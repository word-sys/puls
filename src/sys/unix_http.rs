use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn parse(input: &str) -> Result<JsonValue, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut pos = 0;
        skip_ws(&chars, &mut pos);
        let val = parse_value(&chars, &mut pos)?;
        skip_ws(&chars, &mut pos);
        Ok(val)
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().and_then(|n| {
            if n >= 0.0 && n.is_finite() {
                Some(n as u64)
            } else {
                None
            }
        })
    }

    pub fn as_u16(&self) -> Option<u16> {
        self.as_u64().and_then(|n| u16::try_from(n).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("Unexpected end of JSON input".to_string());
    }

    match chars[*pos] {
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        '"' => parse_string(chars, pos).map(JsonValue::String),
        't' | 'f' => parse_bool(chars, pos),
        'n' => parse_null(chars, pos),
        c if c == '-' || c.is_ascii_digit() => parse_number(chars, pos),
        c => Err(format!("Unexpected character '{}' at pos {}", c, *pos)),
    }
}

fn parse_object(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1;
    skip_ws(chars, pos);
    let mut map = HashMap::new();

    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Ok(JsonValue::Object(map));
    }

    loop {
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '"' {
            return Err(format!("Expected string key in object at pos {}", *pos));
        }
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ':' {
            return Err(format!("Expected ':' after object key at pos {}", *pos));
        }
        *pos += 1;
        let val = parse_value(chars, pos)?;
        map.insert(key, val);

        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("Unterminated JSON object".to_string());
        }
        match chars[*pos] {
            ',' => *pos += 1,
            '}' => {
                *pos += 1;
                return Ok(JsonValue::Object(map));
            }
            c => return Err(format!("Expected ',' or '}}' in object, got '{}' at pos {}", c, *pos)),
        }
    }
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1;
    skip_ws(chars, pos);
    let mut arr = Vec::new();

    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Ok(JsonValue::Array(arr));
    }

    loop {
        let val = parse_value(chars, pos)?;
        arr.push(val);
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("Unterminated JSON array".to_string());
        }
        match chars[*pos] {
            ',' => *pos += 1,
            ']' => {
                *pos += 1;
                return Ok(JsonValue::Array(arr));
            }
            c => return Err(format!("Expected ',' or ']' in array, got '{}' at pos {}", c, *pos)),
        }
    }
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    *pos += 1;
    let mut out = String::new();

    while *pos < chars.len() {
        let c = chars[*pos];
        *pos += 1;
        match c {
            '"' => return Ok(out),
            '\\' => {
                if *pos >= chars.len() {
                    return Err("Unterminated escape in JSON string".to_string());
                }
                let esc = chars[*pos];
                *pos += 1;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        if *pos + 4 > chars.len() {
                            return Err("Incomplete unicode escape in JSON string".to_string());
                        }
                        let hex: String = chars[*pos..*pos + 4].iter().collect();
                        *pos += 4;
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| format!("Invalid unicode escape \\u{}", hex))?;
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }

    Err("Unterminated JSON string".to_string())
}

fn parse_bool(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    if chars[*pos..].starts_with(&['t', 'r', 'u', 'e']) {
        *pos += 4;
        Ok(JsonValue::Bool(true))
    } else if chars[*pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
        *pos += 5;
        Ok(JsonValue::Bool(false))
    } else {
        Err(format!("Invalid boolean token at pos {}", *pos))
    }
}

fn parse_null(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    if chars[*pos..].starts_with(&['n', 'u', 'l', 'l']) {
        *pos += 4;
        Ok(JsonValue::Null)
    } else {
        Err(format!("Invalid null token at pos {}", *pos))
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<JsonValue, String> {
    let start = *pos;
    if chars[*pos] == '-' {
        *pos += 1;
    }
    while *pos < chars.len() && (chars[*pos].is_ascii_digit() || matches!(chars[*pos], '.' | 'e' | 'E' | '+' | '-')) {
        *pos += 1;
    }
    let num_str: String = chars[start..*pos].iter().collect();
    num_str
        .parse::<f64>()
        .map(JsonValue::Number)
        .map_err(|_| format!("Invalid number '{}' at pos {}", num_str, start))
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn body_string(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json(&self) -> Result<JsonValue, String> {
        JsonValue::parse(&self.body_string())
    }
}

#[derive(Debug, Clone)]
pub struct UnixHttpClient {
    socket_path: PathBuf,
}

impl UnixHttpClient {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn get(&self, path: &str, timeout: Duration) -> Result<HttpResponse, String> {
        self.request("GET", path, &[], timeout)
    }

    pub fn post(&self, path: &str, body: &[u8], timeout: Duration) -> Result<HttpResponse, String> {
        self.request("POST", path, body, timeout)
    }

    pub fn request(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
        timeout: Duration,
    ) -> Result<HttpResponse, String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| format!("Failed to connect to {}: {}", self.socket_path.display(), e))?;

        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));

        let req_header = if body.is_empty() {
            format!(
                "{} {} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: puls/{}\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
                method,
                path,
                env!("CARGO_PKG_VERSION")
            )
        } else {
            format!(
                "{} {} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: puls/{}\r\nAccept: application/json\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                method,
                path,
                env!("CARGO_PKG_VERSION"),
                body.len()
            )
        };

        stream
            .write_all(req_header.as_bytes())
            .map_err(|e| format!("Socket write error: {}", e))?;

        if !body.is_empty() {
            stream
                .write_all(body)
                .map_err(|e| format!("Socket body write error: {}", e))?;
        }
        let _ = stream.flush();

        let mut raw = Vec::with_capacity(4096);
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => raw.extend_from_slice(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    if !raw.is_empty() && has_complete_http_response(&raw) {
                        break;
                    }
                    return Err(format!("Socket read error: {}", e));
                }
            }
            if has_complete_http_response(&raw) {
                break;
            }
        }

        parse_http_response(&raw)
    }
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

fn has_complete_http_response(raw: &[u8]) -> bool {
    let Some(hdr_end) = find_header_end(raw) else {
        return false;
    };
    let header_str = String::from_utf8_lossy(&raw[..hdr_end]);
    let body_bytes = &raw[hdr_end + 4..];

    let mut status_code = 0u16;
    if let Some(first_line) = header_str.lines().next() {
        let mut parts = first_line.split_whitespace();
        let _ = parts.next();
        if let Some(code_str) = parts.next() {
            status_code = code_str.parse::<u16>().unwrap_or(0);
        }
    }
    if status_code == 204 || status_code == 304 {
        return true;
    }

    for line in header_str.lines().skip(1) {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().to_ascii_lowercase();
            if key == "content-length" {
                if let Ok(expected_len) = val.parse::<usize>() {
                    return body_bytes.len() >= expected_len;
                }
            } else if key == "transfer-encoding" && val.contains("chunked") {
                return body_bytes.ends_with(b"0\r\n\r\n");
            }
        }
    }

    false
}

pub fn parse_http_response(raw: &[u8]) -> Result<HttpResponse, String> {
    let hdr_end = find_header_end(raw).ok_or_else(|| "Malformed HTTP response (missing header delimiter)".to_string())?;
    let header_text = String::from_utf8_lossy(&raw[..hdr_end]);
    let raw_body = &raw[hdr_end + 4..];

    let mut lines = header_text.lines();
    let status_line = lines.next().ok_or_else(|| "Empty HTTP status line".to_string())?;
    let mut status_parts = status_line.split_whitespace();
    let _http_ver = status_parts.next();
    let status_code = status_parts
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| format!("Invalid HTTP status line: {}", status_line))?;

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let is_chunked = headers
        .get("transfer-encoding")
        .map(|v| v.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false);

    let body = if status_code == 204 || status_code == 304 {
        Vec::new()
    } else if is_chunked {
        decode_chunked_body(raw_body)?
    } else if let Some(len) = headers.get("content-length").and_then(|v| v.parse::<usize>().ok()) {
        raw_body[..len.min(raw_body.len())].to_vec()
    } else {
        raw_body.to_vec()
    };

    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

pub fn decode_chunked_body(mut input: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let Some(crlf_pos) = input.windows(2).position(|w| w == b"\r\n") else {
            break;
        };
        let size_line = String::from_utf8_lossy(&input[..crlf_pos]);
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let chunk_size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| format!("Invalid chunk size hex: '{}'", size_hex))?;

        input = &input[crlf_pos + 2..];
        if chunk_size == 0 {
            break;
        }
        if input.len() < chunk_size {
            out.extend_from_slice(input);
            break;
        }
        out.extend_from_slice(&input[..chunk_size]);
        input = &input[chunk_size..];
        if input.starts_with(b"\r\n") {
            input = &input[2..];
        }
    }
    Ok(out)
}

// Docker non-TTY log streams use 8-byte frame headers: [stream_type, 0, 0, 0, len_b3, len_b2, len_b1, len_b0]
pub fn decode_docker_log_payload(raw: &[u8]) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }

    let looks_multiplexed = raw.len() >= 8
        && matches!(raw[0], 0..=2)
        && raw[1] == 0
        && raw[2] == 0
        && raw[3] == 0;

    if looks_multiplexed {
        let mut lines = Vec::new();
        let mut offset = 0;
        while offset + 8 <= raw.len() {
            let frame_len = u32::from_be_bytes([
                raw[offset + 4],
                raw[offset + 5],
                raw[offset + 6],
                raw[offset + 7],
            ]) as usize;
            offset += 8;
            let end = (offset + frame_len).min(raw.len());
            let text = String::from_utf8_lossy(&raw[offset..end]);
            for line in text.lines() {
                let clean = line.trim_end_matches('\r');
                if !clean.is_empty() {
                    lines.push(clean.to_string());
                }
            }
            offset = end;
        }
        return lines;
    }

    String::from_utf8_lossy(raw)
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn test_json_parser_objects_and_arrays() {
        let sample = r#"{
            "Id": "a1b2c3d4e5f60000",
            "Names": ["/puls-redis"],
            "State": "running",
            "Ports": [{"PrivatePort": 6379, "PublicPort": 6379, "Type": "tcp"}],
            "Memory": {"usage": 1048576, "active": true, "extra": null},
            "Escaped": "line1\nline2\t\"quoted\" \u0041"
        }"#;

        let val = JsonValue::parse(sample).expect("JSON parse failed");
        assert_eq!(val.get("Id").and_then(|v| v.as_str()), Some("a1b2c3d4e5f60000"));
        assert_eq!(
            val.get("Names")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str()),
            Some("/puls-redis")
        );
        let port = &val.get("Ports").unwrap().as_array().unwrap()[0];
        assert_eq!(port.get("PublicPort").and_then(|v| v.as_u16()), Some(6379));
        let mem = val.get("Memory").unwrap();
        assert_eq!(mem.get("usage").and_then(|v| v.as_u64()), Some(1048576));
        assert_eq!(mem.get("active").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(mem.get("extra"), Some(&JsonValue::Null));
        assert_eq!(
            val.get("Escaped").and_then(|v| v.as_str()),
            Some("line1\nline2\t\"quoted\" A")
        );
    }

    #[test]
    fn test_http_chunked_decoding() {
        let raw_http = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"ok\": \r\n5\r\ntrue}\r\n0\r\n\r\n";
        let resp = parse_http_response(raw_http).expect("parse chunked response failed");
        assert_eq!(resp.status_code, 200);
        let json = resp.json().expect("parse chunked json failed");
        assert_eq!(json.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_decode_docker_multiplexed_logs() {
        let mut payload = Vec::new();
        let msg1 = b"2026-11-21 INFO Starting server\n";
        payload.extend_from_slice(&[1, 0, 0, 0]);
        payload.extend_from_slice(&(msg1.len() as u32).to_be_bytes());
        payload.extend_from_slice(msg1);

        let msg2 = b"2026-11-21 WARN High memory\n";
        payload.extend_from_slice(&[2, 0, 0, 0]);
        payload.extend_from_slice(&(msg2.len() as u32).to_be_bytes());
        payload.extend_from_slice(msg2);

        let lines = decode_docker_log_payload(&payload);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "2026-11-21 INFO Starting server");
        assert_eq!(lines[1], "2026-11-21 WARN High memory");
    }

    #[test]
    fn test_unix_http_client_roundtrip() {
        let sock_path = std::env::temp_dir().join(format!("puls_unix_http_test_{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock_path);

        let listener = UnixListener::bind(&sock_path).expect("bind unix socket failed");
        let server_handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 512];
                let _ = stream.read(&mut req_buf);
                let resp = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"status\":\"OK\"}";
                let _ = stream.write_all(resp);
            }
        });

        let client = UnixHttpClient::new(&sock_path);
        let resp = client.get("/_ping", Duration::from_millis(1000)).expect("unix get failed");
        assert_eq!(resp.status_code, 200);
        let json = resp.json().expect("json failed");
        assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("OK"));

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&sock_path);
    }
}
