use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{ToSocketAddrs, TcpListener};
use std::thread;

#[derive(Debug)]
struct Request {
    path: String,
}

impl Request {
    fn new(request_str: &str) -> Self {
        request_str.lines()
            .next()
            .map(str::split_whitespace)
            .and_then(|mut parts| parts.nth(1))
            .map(ToString::to_string)
            .map(|path| Request { path })
            .unwrap_or(Request { path: "".to_string() })
    }
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

pub fn serve<A, F>(addr: A, callback: F) -> io::Result<()>
where
    A: ToSocketAddrs,
    F: FnOnce(&str) -> String + Copy + Send + Sync + 'static
{
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming().filter_map(Result::ok) {
        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let headers = reader.by_ref().lines()
                .filter_map(Result::ok)
                .take_while(|line| !line.is_empty())
                .collect::<String>();

            let request = Request::new(&headers);
            let response_body = &callback(&request.path);
            let response = Response::new(&response_body);

            reader.into_inner()
                .write_all(response.to_string().as_bytes())
                .expect("Unable to write response to stream");
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_in_request_when_it_is_missing() {
        let request_line = "GET";
        let headers = ["Host: localhost"];
        let terminator = "\r\n\r\n";

        let mut request_line_and_headers = Vec::new();
        request_line_and_headers.push(request_line);
        request_line_and_headers.extend(&headers);
        request_line_and_headers.push(terminator);

        let request_string = request_line_and_headers.join("\r\n");
        let request = Request::new(&request_string);

        assert_eq!(request.path, "".to_string());
    }

    #[test]
    fn parse_path_in_request_when_it_is_populated() {
        let request_line = "GET /path HTTP/1.1";
        let headers = ["Host: localhost"];
        let terminator = "\r\n\r\n";

        let mut request_line_and_headers = Vec::new();
        request_line_and_headers.push(request_line);
        request_line_and_headers.extend(&headers);
        request_line_and_headers.push(terminator);

        let request_string = request_line_and_headers.join("\r\n");
        let request = Request::new(&request_string);

        assert_eq!(request.path, "/path".to_string());
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

        let request_string = request_line_and_headers.join("\r\n");
        let request = Request::new(&request_string);

        let response = Response::new(&request.path);
        let response_str = response.to_string();
        let mut lines = response_str.lines();

        assert_eq!(lines.next().unwrap(), "HTTP/1.1 200 OK".to_string());
        assert_eq!(lines.next().unwrap(), "Connection: close".to_string());
        assert!(lines.next().unwrap().is_empty());

        let response_body = lines.collect::<String>();
        assert_eq!(response_body, "/path".to_string());
    }
}