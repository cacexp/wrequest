use crate::{HttpMethod, Request};
use std::collections::HashMap;
use json::object;

#[test]
fn request1() {
    let request = Request::connect("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::CONNECT);
    assert_eq!(request.url(), "http://example.com/user" );    
}

#[test]
fn request2() {
    let request = Request::delete("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::DELETE);
    assert_eq!(request.url(), "http://example.com/user" );    
}

#[test]
fn request3() {
    let request = Request::get("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::GET);
    assert_eq!(request.url(), "http://example.com/user" );    
}

#[test]
fn request4() {
    let request = Request::head("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::HEAD);
    assert_eq!(request.url(), "http://example.com/user" );    
}

#[test]
fn request5() {
    let request = Request::options("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::OPTIONS);
    assert_eq!(request.url(), "http://example.com/user" );    
}
#[test]
fn request6() {
    let request = Request::patch("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::PATCH);
    assert_eq!(request.url(), "http://example.com/user" );    
}
#[test]
fn request7() {
    let request = Request::post("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::POST);
    assert_eq!(request.url(), "http://example.com/user" );    
}
#[test]
fn request8() {
    let request = Request::trace("http://example.com/user");
    assert_eq!(request.method(), HttpMethod::TRACE);
    assert_eq!(request.url(), "http://example.com/user" );    
}

#[test]
fn header1() {
    let mut request = Request::post("http://example.com/user");
    request.header("Content-Type", "application/json");
    // check header is case insensitive on get
    assert_eq!(request.get_header("content-type").unwrap(), "application/json");
    // check header is case insensitive on insert
    assert!(request.header("content-Type", "application/json"));
    assert_eq!(request.get_header("Content-type").unwrap(), "application/json");
}

#[test]
fn header2() {
    let mut request = Request::post("http://example.com/user");
    request.header("Content-Type", "application/json");
    request.header("Accept", "application/json");

    let iter = request.header_iter();

    let mut contained :  HashMap<&str, &str> =HashMap::new();

    for (name, value) in iter {
        contained.insert(name, value);
    }

    assert!(contained.contains_key("Content-Type"));
    assert_eq!(*contained.get("Content-Type").unwrap(), "application/json");

    assert!(contained.contains_key("Accept"));
    assert_eq!(*contained.get("Accept").unwrap(), "application/json");
  
}

#[test]
fn param1() {
    let mut request = Request::post("http://example.com/user");
    request.param("id", "1234");
    // check paran  is case sensitive on get
    assert!(request.get_param("ID").is_none());
    assert_eq!(request.get_param("id").unwrap(), "1234");
    // check header is case sensitive on insert
    assert!(! request.param("ID", "3456"));
    assert_eq!(request.get_param("id").unwrap(), "1234");
    assert!(request.param("id", "3456"));
    assert_eq!(request.get_param("id").unwrap(), "3456");
}

#[test]
fn param2() {
    let mut request = Request::post("http://example.com/user");
    request.param("id", "1234");
    request.param("departament", "marketing");

    let iter = request.param_iter();

    let mut contained :  HashMap<&str, &str> =HashMap::new();

    for (name, value) in iter {
        contained.insert(name, value);
    }

    assert!(contained.contains_key("id"));
    assert_eq!(*contained.get("id").unwrap(), "1234");

    assert!(contained.contains_key("departament"));
    assert_eq!(*contained.get("departament").unwrap(), "marketing");
}

#[test]
fn cookie() {
    let mut request = Request::post("http://example.com/user");
    request.cookie("id", "1234");
    // Cookies are case sensitive
    request.cookie("ID", "3456");
    request.cookie("departament", "marketing");


    let mut contained :  HashMap<&str, &str> =HashMap::new();

    for (name, value) in request.cookies() {
        contained.insert(name, value);
    }

    assert!(contained.contains_key("id"));
    assert_eq!(*contained.get("id").unwrap(), "1234");

    assert!(contained.contains_key("ID"));
    assert_eq!(*contained.get("ID").unwrap(), "3456");

    assert!(contained.contains_key("departament"));
    assert_eq!(*contained.get("departament").unwrap(), "marketing");
}

#[test]
fn  json1() {

    // Create a PUT https://service.com/users/ request

    let mut request = Request::put("https://service.com/users/");

    // Add a ?client_id=1234 param
    request.param("client_id", "1234");

    // Add request headers
    request.header("Content-Type", "application/json");
    request.header("Accept", "application/json");

    // Add a JSON Object as request body
    let data = object! {
        name: "John",
        surname: "Smith"
    };

    // JSON Object is encoded at the body
    request.json(&data);

    assert_eq!(request.get_header("Content-Type").unwrap(), "application/json" );

    let extracted = request.get_json();

    assert!(extracted.is_ok());

    assert_eq!(extracted.unwrap(), data);
}