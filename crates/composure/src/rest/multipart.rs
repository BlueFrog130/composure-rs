//! [Uploading files](https://discord.com/developers/docs/reference#uploading-files): `payload_json`
//! plus one `files[n]` part per file.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::types::File;

use super::{Error, Result};

/// Encoded body and its `Content-Type`.
///
/// When `payload` has no `attachments`, one `{id, filename, description}` entry per file is added
/// so descriptions reach Discord; an explicit `attachments` is sent as is.
pub(crate) fn encode(payload: &impl Serialize, files: &[File]) -> Result<(Vec<u8>, String)> {
    let mut payload = match serde_json::to_value(payload).map_err(Error::Encode)? {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    if !payload.contains_key("attachments") {
        let attachments = files
            .iter()
            .enumerate()
            .map(|(id, file)| {
                let mut attachment = Map::new();
                attachment.insert("id".into(), id.into());
                attachment.insert("filename".into(), file.name.clone().into());
                if let Some(description) = &file.description {
                    attachment.insert("description".into(), description.clone().into());
                }
                Value::Object(attachment)
            })
            .collect();
        payload.insert("attachments".into(), Value::Array(attachments));
    }
    let payload = serde_json::to_vec(&payload).map_err(Error::Encode)?;

    let boundary = boundary(&payload, files);
    let size = payload.len() + files.iter().map(|f| f.bytes.len() + 200).sum::<usize>() + 200;
    let mut body = Vec::with_capacity(size);

    part_header(
        &mut body,
        &boundary,
        "payload_json",
        None,
        Some("application/json"),
    );
    body.extend_from_slice(&payload);
    for (id, file) in files.iter().enumerate() {
        body.extend_from_slice(b"\r\n");
        part_header(
            &mut body,
            &boundary,
            &format!("files[{id}]"),
            Some(&file.name),
            Some(
                file.content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
            ),
        );
        body.extend_from_slice(&file.bytes);
    }
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    Ok((body, format!("multipart/form-data; boundary={boundary}")))
}

/// A plain form: text fields plus one `file` part (sticker uploads)
pub(crate) fn encode_form(fields: &[(&str, &str)], file: &File) -> (Vec<u8>, String) {
    let text: Vec<u8> = fields.iter().flat_map(|(_, value)| value.bytes()).collect();
    let boundary = boundary(&text, std::slice::from_ref(file));
    let mut body = Vec::with_capacity(text.len() + file.bytes.len() + 200 * (fields.len() + 2));
    for (name, value) in fields {
        part_header(&mut body, &boundary, name, None, None);
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    part_header(
        &mut body,
        &boundary,
        "file",
        Some(&file.name),
        Some(
            file.content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        ),
    );
    body.extend_from_slice(&file.bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (body, format!("multipart/form-data; boundary={boundary}"))
}

fn part_header(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: Option<&str>,
    content_type: Option<&str>,
) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"").as_bytes());
    if let Some(filename) = filename {
        body.extend_from_slice(format!("; filename=\"{}\"", escape(filename)).as_bytes());
    }
    body.extend_from_slice(b"\r\n");
    if let Some(content_type) = content_type {
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
    }
    body.extend_from_slice(b"\r\n");
}

fn escape(filename: &str) -> String {
    filename
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ")
}

/// A boundary appearing in no part
fn boundary(payload: &[u8], files: &[File]) -> String {
    let mut boundary = String::from("composure-boundary-7f3a9c");
    let contains = |haystack: &[u8], needle: &str| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    };
    let mut n = 0u32;
    while contains(payload, &boundary) || files.iter().any(|f| contains(&f.bytes, &boundary)) {
        n += 1;
        boundary.push_str(&format!("{n:x}"));
    }
    boundary
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn encodes_payload_and_files() {
        let files = [File::new("a.txt", b"hello".to_vec())
            .description("alt")
            .content_type("text/plain")];
        let (body, content_type) = encode(&json!({"content": "hi"}), &files).unwrap();
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .unwrap();
        let body = String::from_utf8(body).unwrap();
        assert_eq!(
            body,
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"payload_json\"\r\nContent-Type: application/json\r\n\r\n\
                 {{\"attachments\":[{{\"description\":\"alt\",\"filename\":\"a.txt\",\"id\":0}}],\"content\":\"hi\"}}\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"files[0]\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\n\
                 hello\r\n--{boundary}--\r\n"
            )
        );
    }

    #[test]
    fn encodes_forms() {
        let file = File::new("s.png", b"PNG".to_vec()).content_type("image/png");
        let (body, content_type) = encode_form(&[("name", "wave"), ("tags", "hi")], &file);
        let boundary = content_type.rsplit('=').next().unwrap();
        assert_eq!(
            String::from_utf8(body).unwrap(),
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nwave\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"tags\"\r\n\r\nhi\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"s.png\"\r\nContent-Type: image/png\r\n\r\n\
                 PNG\r\n--{boundary}--\r\n"
            )
        );
    }

    #[test]
    fn boundary_avoids_file_contents() {
        let files = [File::new("a", b"composure-boundary-7f3a9c".to_vec())];
        let (body, content_type) = encode(&json!({}), &files).unwrap();
        let boundary = content_type.rsplit('=').next().unwrap();
        assert_ne!(boundary, "composure-boundary-7f3a9c");
        assert!(String::from_utf8_lossy(&body).contains(&format!("--{boundary}--")));
    }
}
