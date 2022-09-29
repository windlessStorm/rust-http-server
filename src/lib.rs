use std::{
    sync::{ mpsc, Arc, Mutex }, 
    thread,
    collections::HashMap,
};

#[derive(Debug)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,

}

const HTTP_VERSION: &str = "1.1";

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, reciever: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn( move || loop {
            let message =  reciever.lock().unwrap().recv();
            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });
        Worker { 
            id, 
            thread: Some(thread),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Create a new Threadpool.
    /// 
    /// The size is the number of threads in the pool.
    /// 
    /// # Panics
    /// 
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, reciever) = mpsc::channel();
        let reciever = Arc::new(Mutex::new(reciever));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            // let worker = Worker::new(id, reciever);
            workers.push(Worker::new(id, Arc::clone(&reciever)));
        }
        ThreadPool { 
            workers, 
            sender: Some(sender), 
        }
    }
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
        {
            let job = Box::new(f);
            self.sender.as_ref().unwrap().send(job).unwrap();
        }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub message: MessageBody
}


#[derive(Debug)]
pub struct RequestLine {
    pub method: HttpMethod,
    pub request_uri: String,
    pub http_version: String,
}

#[derive(Debug)]
pub struct Headers {
    pub headers: HashMap<String, String>
}

#[derive(Debug)]
pub struct MessageBody{
    pub message_body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(req: [u8; 2048]) -> HttpRequest {
        let req = std::string::String::from_utf8((&req).to_vec()).unwrap();

        let request_header_body: Vec<String> = req.split("\r\n\r\n").map(|s| s.to_string()).collect();
        let request_header = request_header_body[0].clone();
        let message_body =  request_header_body[1].clone().into_bytes();
        let mut header_lines: Vec<String> = request_header.split("\r\n").map(|s| s.to_string()).collect();
        
        let request_line: String = header_lines[0].clone();
        let request_line = RequestLine::new(request_line);
        
        header_lines.remove(0);
        let header_lines = Headers::new(header_lines);

        let message_body = MessageBody::new(message_body);
        HttpRequest { 
            request_line, 
            headers: header_lines, 
            message: message_body 
        }
    }
}

impl RequestLine {
    pub fn new(request_line: String) -> RequestLine {
        let req: Vec<&str> = request_line.split(" ").collect();
        assert!(req.len() == 3);

        let method = match req[0] {
            "GET" => HttpMethod::GET,
            "HEAD" => HttpMethod::HEAD,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "CONNECT" => HttpMethod::CONNECT,
            "OPTIONS" => HttpMethod::OPTIONS,
            "TRACE" => HttpMethod::TRACE,
            _ => {
                panic!("Unsupported value for http method!");
            }
        };

        let request_uri = req[1].to_string();
        let http_version = req[2].to_string();
        RequestLine { 
            method, 
            request_uri, 
            http_version 
        }
    }
}

impl Headers {
    pub fn new(header_lines: Vec<String>) -> Headers {
        let mut headers = HashMap::new();
        for line in header_lines.clone() {
            let key_value: Vec<&str> = line.split(":").collect();
            let key = key_value[0].trim();
            let value = key_value[1].trim();
            headers.insert(key.to_string(), value.to_string());
        }
        Headers { headers }
    }
    
    pub fn to_text(&self) -> String {
        let mut header_text = String::new();
        for (key, value) in &self.headers {
            header_text.push_str(&format!("{}: {}\r\n", key, value));
        };

        header_text
    }
}

impl MessageBody {
    pub fn new(message_body: Vec<u8>) -> MessageBody {
        MessageBody { message_body }
    }

    pub fn to_text(&self) -> String {
        format!("{}", String::from_utf8_lossy(&self.message_body))
    }
}

pub struct HttpResponse {
    pub status_line: StatusLine,
    pub headers: Headers,
    pub message_body: MessageBody,
}

pub struct StatusLine {
    pub http_version: String,
    pub status_code: String,
    pub reason: String,
}

pub struct StatusCode {
    code: String,
    reason: String
}

impl HttpResponse {
    pub fn new(code: String, headers: Headers, message_body: MessageBody) -> HttpResponse {
        let status_line = StatusLine::new("HTTP".to_string() + HTTP_VERSION, code);
        HttpResponse { status_line, headers, message_body }
    }
    pub fn to_text(&self) -> String {
        format!("{}\r\n{}\r\n{}",
            self.status_line.to_text(), 
            self.headers.to_text(), 
            self.message_body.to_text()
        )
    }
}

impl StatusLine {
    pub fn new(http_version: String, status_code: String) -> StatusLine {
        let status = StatusCode::new(status_code);
        StatusLine { http_version, status_code: status.code, reason: status.reason }
    }
    fn to_text(&self) -> String {
        format!("{} {} {}", self.http_version, self.status_code, self.reason)
    }
}

impl StatusCode {
    fn new(code: String) ->  StatusCode {
        let reason = match code.as_str() {
            "200" => {
                "OK".to_string()
            },
            "404" => {
                "Not Found".to_string()
            },
            _ => {
                panic!("{} status code not supported!", code)
            }
        };
        StatusCode { code, reason }
    }
}
