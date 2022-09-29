use std::{
    fs::File,
    net::{TcpListener, TcpStream}, 
    io::prelude::*,
    thread,
    time::Duration, 
    path::{PathBuf},
    collections::HashMap,
};
use rust_http_server::{
    ThreadPool,
    HttpRequest,
    HttpResponse,
    MessageBody,
};

const ADDRESS: &str = "127.0.0.1";
const PORT: &str = "7878";
const WEB_ROOT: &str = r"D:\personal\Wedding-Invitation";

struct Routes<'a> {
    routes: HashMap<&'a str, fn()>,
}

fn get_route() -> Routes<'static> {
    let mut routes: HashMap<_, fn()> = HashMap::new();
    routes.insert("/sleep", sleep);

    Routes { routes }
}

fn main() {
    let listener = TcpListener::bind(format!("{ADDRESS}:{PORT}")).unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    stream.read(&mut buffer).unwrap();
    let response: HttpResponse;

    let http_request = HttpRequest::new(buffer.clone());
    
    let route = get_route();

    match http_request.request_line.method {
        rust_http_server::HttpMethod::GET => {
            let mut headers = http_request.headers;
            let uri = http_request.request_line.request_uri;
            if route.routes.contains_key(uri.as_str()) {
                let dispatch_path = route.routes.get(uri.as_str()).unwrap();
                dispatch_path();
                let message = read_file("index.html".to_string());
                headers.headers.insert("content-length".to_string(), message.len().to_string());
                let message_body = MessageBody::new(message);
                response = HttpResponse::new("200".to_string(), headers, message_body);
            } else if file_exists(uri.clone()) {
                let mut message = Vec::new();
                let mut file = File::open(get_file(uri.clone())).expect("Unable to open file");
                file.read_to_end(&mut message).expect("Unable to read");
                
                headers.headers.insert("content-length".to_string(), message.len().to_string());
                
                let message_body = MessageBody::new(message);
                response = HttpResponse::new("200".to_string(), headers, message_body);
            } else {
                let mut message: Vec<u8> = Vec::new();
                if file_exists("404.html".to_owned()) {
                    let mut file = File::open(get_file("404.html".to_string())).expect("Unable to open file");
                    file.read_to_end(&mut message).expect("Unable to read");
                } else {
                    let mut file = File::open("404.html".to_string()).expect("Unable to open file");
                    file.read_to_end(&mut message).expect("Unable to read");
                }
                headers.headers.insert("content-length".to_string(), message.len().to_string());
                let message_body = MessageBody::new(message);
                response = HttpResponse::new("404".to_string(), headers, message_body);
            }
        },
        _ => {
            let headers = http_request.headers;
            let message_body = MessageBody::new([].to_vec());
            response = HttpResponse::new("404".to_string(), headers, message_body);
            println!("Method not implemented: {:#?}", http_request.request_line.method);
        }
    }

    send(response, stream);
}

fn file_exists(mut uri: String) -> bool {
    let mut path = PathBuf::from(WEB_ROOT);
    if uri == "/" {
        uri = "index.html".to_string();
    }
    if uri.starts_with("/") {
        uri.remove(0);
    }
    path.push( uri );

    path.exists() && path.is_file()
}

fn get_file(mut uri: String) -> PathBuf {
    let mut path = PathBuf::from(WEB_ROOT);
    if uri == "/" {
        uri = "index.html".to_string();
    }
    if uri.starts_with("/") {
        uri.remove(0);
    }
    path.push( uri );

    path
}

fn send(response: HttpResponse, mut stream: TcpStream) {
    let response_data = response.to_text();
    stream.write_all(response_data.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn sleep() {
    thread::sleep(Duration::from_secs(5));
}

fn read_file(filename: String) -> Vec<u8> {
    let mut path = PathBuf::from(WEB_ROOT);
    path.push( filename.clone() );
    let mut file_content = Vec::new();
    let mut file = File::open(filename).expect("Unable to open file");
    file.read_to_end(&mut file_content).expect("Unable to read");
    file_content
}