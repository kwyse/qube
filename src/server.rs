#[derive(Debug)]
struct Request {
    path: String,
    body: String,
}

#[derive(Debug)]
struct Response {
    body: String,
}

impl Response {
    fn new(body: &str) -> Self {
        Self {
            body: body.to_string(),
        }
    }
}

impl ToString for Response {
    fn to_string(&self) -> String {
        ["HTTP/1.1 200 OK", "Connection: close"].join("\r\n") + "\r\n\r\n" + &self.body
    }
}

use std::io;
use std::thread;

pub fn serve<F: FnOnce(&str) -> String + Send + Sync + 'static + Copy>(callback: F) -> io::Result<()> {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::io::{BufRead, BufReader};

    let listener = TcpListener::bind("127.0.0.1:12343")?;
    for stream in listener.incoming().filter_map(Result::ok) {
        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let headers = reader.by_ref().lines()
                .filter_map(Result::ok)
                .take_while(|line| !line.is_empty())
                .collect::<String>();

            let request = parse_request(&headers);
            let response_body = &callback(&request.path);
            let response = Response::new(&response_body);

            reader.into_inner()
                .write_all(response.to_string().as_bytes())
                .expect("Unable to write response to stream");
        });
    }

    Ok(())
}

fn parse_request(request: &str) -> Request {
    let mut request_parts = request.splitn(2, "\r\n\r\n");

    let mut path = "".to_string();
    let mut body = "".to_string();

    if let Some(request_line_and_headers) = request_parts.next() {
        let mut parts = request_line_and_headers.lines();
        if let Some(request_line) = parts.next() {
            if let Some(request_path) = request_line.split(" ").nth(1) {
                path = request_path.to_string();
            }
        }
    }

    if let Some(request_body) = request_parts.next() {
        body = request_body.to_string();
    }

    Request { path, body }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_in_request() {
        let request_line = "GET /path HTTP/1.1";
        let headers = ["Host: localhost"];
        let terminator = "\r\n\r\n";

        let mut request_line_and_headers = Vec::new();
        request_line_and_headers.push(request_line);
        request_line_and_headers.extend(&headers);
        request_line_and_headers.push(terminator);

        let request_string = request_line_and_headers.join("\r\n");
        let request = parse_request(&request_string);

        assert_eq!(request.path, "/path".to_string());
    }

    #[test]
    fn parse_body_in_request() {
        let request_line = "GET /path HTTP/1.1";
        let headers = ["Host: localhost"];
        let terminator = "\r\n";

        let mut request_line_and_headers = Vec::new();
        request_line_and_headers.push(request_line);
        request_line_and_headers.extend(&headers);
        request_line_and_headers.push(terminator);

        let body = r#"{"message": "some JSON message"}"#;
        let request_string = request_line_and_headers.join("\r\n") + body;
        let request = parse_request(&request_string);

        assert_eq!(request.body, r#"{"message": "some JSON message"}"#.to_string());
    }

    #[test]
    fn build_echo_response_from_request() {
        let request_line = "GET /path HTTP/1.1";
        let headers = ["Host: localhost"];
        let terminator = "\r\n";

        let mut request_line_and_headers = Vec::new();
        request_line_and_headers.push(request_line);
        request_line_and_headers.extend(&headers);
        request_line_and_headers.push(terminator);

        let body = r#"{"message": "some JSON message"}"#;
        let request_string = request_line_and_headers.join("\r\n") + body;
        let request = parse_request(&request_string);

        let response = Response::new(&request.body);
        let response_str = response.to_string();
        let mut lines = response_str.lines();

        assert_eq!(lines.next().unwrap(), "HTTP/1.1 200 OK".to_string());
        assert_eq!(lines.next().unwrap(), "Connection: close".to_string());
        assert!(lines.next().unwrap().is_empty());

        let response_body = lines.collect::<String>();
        assert_eq!(response_body, r#"{"message": "some JSON message"}"#.to_string());
    }
}