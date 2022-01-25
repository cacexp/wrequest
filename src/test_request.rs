use crate::{HttpMethod, Request};
use std::collections::HashMap;
use json::object;
use url::Url;

#[test]
fn request1() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::connect(&url);
    assert_eq!(request.method(), HttpMethod::CONNECT);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}

#[test]
fn request2() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::delete(&url);
    assert_eq!(request.method(), HttpMethod::DELETE);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}

#[test]
fn request3() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::get(&url);
    assert_eq!(request.method(), HttpMethod::GET);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}

#[test]
fn request4() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::head(&url);
    assert_eq!(request.method(), HttpMethod::HEAD);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}

#[test]
fn request5() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::options(&url);
    assert_eq!(request.method(), HttpMethod::OPTIONS);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}
#[test]
fn request6() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::patch(&url);
    assert_eq!(request.method(), HttpMethod::PATCH);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}
#[test]
fn request7() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::post(&url);
    assert_eq!(request.method(), HttpMethod::POST);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}
#[test]
fn request8() {
    let url = Url::parse("http://example.com/user").unwrap();
    let request = Request::trace(&url);
    assert_eq!(request.method(), HttpMethod::TRACE);
    assert_eq!(request.url().as_str(), "http://example.com/user" );    
}

#[test]
fn header1() {
    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);
    request.header("Content-Type", "application/json");
    // check header is case insensitive on get
    assert_eq!(request.get_header("content-type").unwrap(), "application/json");
    // check header is case insensitive on insert
    request.header("content-Type", "application/json");
    assert_eq!(request.get_header("Content-type").unwrap(), "application/json");
}

#[test]
fn header2() {
    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);
    request.header("Content-Type", "application/json")
           .header("Accept", "application/json");

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
    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);
    request.param("id", "1234");
    // check paran  is case sensitive on get
    assert!(request.get_param("ID").is_none());
    assert_eq!(request.get_param("id").unwrap(), "1234");
    // check header is case sensitive on insert
    request.param("ID", "3456");
    assert_eq!(request.get_param("id").unwrap(), "1234");
    request.param("id", "3456");
    assert_eq!(request.get_param("id").unwrap(), "3456");
}

#[test]
fn param2() {
    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);
    request.param("id", "1234")
           .param("departament", "marketing");

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
    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);
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

    let url = Url::parse("http://example.com/user").unwrap();
    let mut request = Request::connect(&url);

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