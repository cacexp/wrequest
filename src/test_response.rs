use crate::*;
use std::collections::{HashMap, HashSet};
use json::object;
use wcookie::SetCookie;
use std::time::Duration;

#[test]
fn status_code() {
    let response = Response::new(HTTP_200_OK);
    assert_eq!(response.status_code(), HTTP_200_OK);
}

#[test]
fn header1() {
    let mut response = Response::new(HTTP_200_OK);
    response.insert_header("Content-Type", "application/json");
    // check header is case insensitive on get
    assert_eq!(response.headers.get("content-type").unwrap(), "application/json");
    // check header is case insensitive on insert
    assert_eq!(response.headers.get("Content-type").unwrap(), "application/json");
}

#[test]
fn header2() {
    let mut response = Response::new(HTTP_200_OK);
    response.insert_header("Content-Type", "application/json");
    response.insert_header("Second", "now");

    let iter = response.headers().iter();

    let mut contained :  HashMap<&str, &str> =HashMap::new();

    for (name, value) in iter {
        contained.insert(name, value);
    }

    assert!(contained.contains_key("Content-Type"));
    assert_eq!(*contained.get("Content-Type").unwrap(), "application/json");

    assert!(contained.contains_key("Second"));
    assert_eq!(*contained.get("Second").unwrap(), "now");
  
}

#[test]
fn cookie1() {
    let mut cookie1 = SetCookie::new("session", "1234");
    cookie1.max_age = Some(Duration::new(3600, 0));

    let cookie2 = SetCookie::new("Session", "3456");

    let mut response = Response::new(HTTP_200_OK);

    response.insert_cookie(cookie1);
    response.insert_cookie(cookie2);

    let mut contained:  HashSet<&str> = HashSet::new();

    for c in  response.cookies().iter() {
        contained.insert(c.name.as_str());
    }

    assert!(contained.contains("session"));
    assert!(contained.contains("Session"));

}

#[test]
fn  json1() {

    // Create a https://service.com/users/ response

    let mut response = Response::new(HTTP_200_OK);

    // Add a JSON Object as response body
    let data = object! {
        name: "John",
        surname: "Smith"
    };

    // JSON Object is encoded at the body
    response.set_json(&data);

    assert_eq!(response.headers().get("Content-Type").unwrap(), "application/json" );

    let extracted = response.json();

    assert!(extracted.is_ok());

    assert_eq!(extracted.unwrap(), data);
}