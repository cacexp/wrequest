// Copyright 2021 Juan A. Cáceres (cacexp@gmail.com)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # HTTP Request and Response implementation in Rust
//! `wrequest` is a crate that implements HTTP requests and responses. 
//! 
//! ## Request creation
//! ```
//! use wrequest::Request;
//! use json::object;
//! 
//! // Create a PUT https://service.com/users/ request
//! 
//! let mut request = Request::put("https://service.com/users/");
//! 
//! // Add a ?client_id=1234 param
//! request.param("client_id", "1234");
//! 
//! // Add request headers
//! request.header("Content-Type", "application/json");
//! request.header("Accept", "application/json");
//! 
//! // Add a request cookie
//! request.cookie("session", "1234");
//! 
//! // Add a JSON Object as request body
//! let data = object! {
//!    name: "John",
//!    surname: "Smith"
//! };
//! 
//! // JSON Object is encoded at the body
//! request.json(&data);
//! 
//! assert_eq!(request.get_header("Content-Type").unwrap(), "application/json" );
//! 
//! ```
//! ## Response creation
//! 
//! ```
//! use wrequest::Response;
//! use wcookie::SetCookie;
//! use std::time::Duration;
//! use json::object;
//! 
//! 
//! // Create a HTTP 200 OK response
//! 
//! let mut response = Response::new(wrequest::HTTP_200_OK);
//! 
//! // Add response headers
//! response.header("Content-Type", "application/json");
//! 
//! // Add a JSON Object as request body
//! let data = object! {
//!    name: "John",
//!    surname: "Smith"
//! };
//! 
//! // Add a `Set-Cookie` header
//! let mut cookie = SetCookie::new("session", "1234");
//! cookie.max_age = Some(Duration::new(3600, 0));
//! response.cookie(cookie);
//! 
//! // JSON Object is encoded at the body
//! response.json(&data);
//! 
//! assert_eq!(response.get_header("Content-Type").unwrap(), "application/json" );
//! 
//! ```
//!  
//! # Future Features
//! * Multipart
//!  

#![allow(dead_code)]

use case_insensitive_hashmap::CaseInsensitiveHashMap;
use std::str::from_utf8;
use std::io::{ErrorKind, Error};
use json::JsonValue;
use std::fmt;
use std::collections::HashMap;
use wcookie::SetCookie;
use unicase::UniCase;
use std::ops::{Deref, DerefMut};

/// `Content-Type` header name
pub const CONTENT_TYPE: &str = "Content-Type";
/// `Content-Type` header value for JJSON encoded in UTF-8
pub const APPLICATION_JSON: &str = "application/json";
/// `Accept` header name
pub const ACCEPT: &str = "Accept";

/// HTTP Request Method
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum HttpMethod {GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE, PATCH}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
                match self {
                    Self::GET => "GET",
                    Self::HEAD => "HEAD",
                    Self::POST => "POST",
                    Self::PUT=> "PUT",
                    Self::DELETE=> "DELETE",
                    Self::CONNECT => "CONNECT",
                    Self::OPTIONS => "OPTIONS",
                    Self::TRACE => "TRACE",
                    Self::PATCH=> "PATCH"
                }
        )
    }
}

// Message Body
#[derive(Clone, PartialEq, Debug)]
enum MessageBody {
    None,
    Single(Vec<u8>),
    MultiPart
}

impl MessageBody {
    fn is_none(&self) -> bool {
        matches!(*self, Self::None)
    }
    fn is_single(&self) -> bool {
        matches!(*self, Self::Single(_))
    }
    fn is_multipart(&self) -> bool {
        matches!(*self, Self::MultiPart)
    }
}

/// Case-insensitive string
type CaseInsensitiveString = UniCase<String>;
type Headers = CaseInsensitiveHashMap<String>;


/// Iterator over request headers
/// 
/// Many thanks to [Returning Rust Iterators](https://depth-first.com/articles/2020/06/22/returning-rust-iterators/)
pub struct HeaderIter<'a> {
    iter: std::collections::hash_map::Iter<'a, UniCase<String>, String>
}

impl<'a> Iterator for HeaderIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, value)| (key.as_str(), value.as_str()) )
    }
}

/// Iterator Over key/value parameters or cookies
pub struct KeyValueIter<'a> {
    iter: std::collections::hash_map::Iter<'a, String, String>
}

impl<'a> Iterator for KeyValueIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, value)| (key.as_str(), value.as_str()) )
    }
}

/// Base message for Request and Response
pub struct HttpMessage {
    /// Request headers
    headers: Headers,
    /// Request body (not implemented multi-part yet)
    body: MessageBody
}

impl HttpMessage {
    fn new() -> HttpMessage {
        HttpMessage {
            headers : Headers::new(),
            body: MessageBody::None
        }
    }

    /// Sets a header by key=value, returns `true` if there was a previous header and its value is overrided
    pub fn header<K, V>(&mut self, key: K, value: V) -> bool
    where K: Into<String>, V: Into<String> {
        self.headers.insert(CaseInsensitiveString::new(key.into()), value.into()).is_some()
    }

    /// Gets a header by key, returns `None` if not found
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).as_ref().map(|s| s.as_str())
    }

    /// Gets an iterator over the message headers
    pub fn header_iter(&self) -> HeaderIter {
        HeaderIter {
            iter: self.headers.iter()
        }
    }

    /// Checks if the request has a single body
    pub fn has_single_body(&self) -> bool {
        self.body.is_single()
    }

    /// Checks if the request has a multi-part body
    pub fn has_multipart_body(&self) -> bool {
        self.body.is_multipart()
    }

    /// Sets a single body
    pub fn body(&mut self, data: Vec<u8>) {
        self.body = MessageBody::Single(data);
    }

    /// Gets body data if any, returns `None` if there is no single body
    pub fn get_body (&self) -> Option<&Vec<u8>> {
        if let MessageBody::Single(ref body) = self.body {
            Some(body)
        } else {
            None
        }
    }

     /// Sets a json object as request body. The `data` object is marshaled into a buffer using UTF8 coding.
     pub fn json(&mut self, data: &JsonValue) {
        let pretty = data.pretty(4);
        self.body = MessageBody::Single(pretty.into_bytes());
        self.header(CONTENT_TYPE, APPLICATION_JSON);
    }

    /// Checks if the Response has body and tries to parse as a `json::JsonValue'
    pub fn get_json(&self) -> Result<JsonValue, Error> {
        if ! self.has_single_body() {
            return Err(Error::new(ErrorKind::InvalidData, "Empty body"));
        }

        let str_body = from_utf8(self.get_body().unwrap());

        if str_body.is_err() {
            return Err(Error::new(ErrorKind::InvalidData, str_body.err().unwrap()));
        }

        let result = json::parse(str_body.unwrap());

        return if result.is_ok() {
            Ok(result.unwrap())
        } else {
            Err(Error::new(ErrorKind::InvalidData, result.err().unwrap()))
        }
    }
}


/// HTTP request
/// 
/// A request is composed of:
/// * HTTP method (`GET`, `POST`, `PUT`, `DELETE`, ... )
/// * Target URL, for example, `https://myservice.com/users`
/// * (optional) Request headers.
/// * (optional) Request path parameters, for example, `id` in the  URL `https://myservice.com/users?id=1111`.
/// * (optional) Server cookies
/// * (optional) Request body of type `Vec[u8]`
/// 
/// Request headers, parameters and cookies are represented by a pair of `name` and `value` strings.
/// 
/// For more information see [HTTP Request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#http_requests).
///

pub struct Request {
    /// Base message
    base: HttpMessage,
    /// HTTP method
    method: HttpMethod,    
    /// Target URL
    url: String,  
    /// Request Cookies
    cookies: HashMap<String, String>,
    /// Request params
    params: HashMap<String, String>   
}

impl Request {

    // Hidden constructor
    fn new<S>(method: HttpMethod, url: S) -> Request 
    where S: Into<String> {
        Request {
            base: HttpMessage::new(),
            method,
            url: url.into(),
            cookies: HashMap::new(),
            params: HashMap::new()
        }
    }

    /// Creates a `CONNECT` request builder
    pub fn connect<S>(url: S) -> Request 
        where S : Into<String> {
        Self::new(HttpMethod::CONNECT, url)
    }

    /// Creates a `DELETE` request builder
    pub fn delete<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::DELETE, url)
    }

    /// Creates a `GET` request builder
    pub fn get<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::GET, url)
    }

    /// Creates a `HEAD` request builder
    pub fn head<S>(url: S) -> Request 
    where S : Into<String>  {
        Self::new(HttpMethod::HEAD, url)
    }

    /// Creates a `OPTIONS` request builder
    pub fn options<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::OPTIONS, url)
    }

    /// Creates a `PATCH` request builder
    pub fn patch<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::PATCH, url)
    }

    /// Creates a `POST` request builder
    pub fn post<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::POST, url)
    }

    /// Creates a `PUT` request builder
    pub fn put<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::PUT, url)
    }
    
    /// Creates a `TRACE` request builder
    pub fn trace<S>(url: S) -> Request 
    where S : Into<String> {
        Self::new(HttpMethod::TRACE, url)
    }

    /// Gets HTTP Method
    pub fn method(&self) -> HttpMethod {
        self.method
    }

    /// Gets the target URL
    pub fn url(&self) -> &str {
        self.url.as_str()
    }    

    /// Sets a Cookie, returns `true` if a cookie value is overriden
    /// 
    /// Cookie names are case-sensitive.
    pub fn cookie<K, V>(&mut self, key: K, value: V) -> bool 
    where K: Into<String>, V: Into<String> {
        self.cookies.insert(key.into(), value.into()).is_some()
    }

    /// Gets a `(&key, &value)` vector of request cookies
    pub fn cookies(&self) -> Vec<(&str, &str)> {
       self.cookies.iter().map(|(n,v)| (n.as_str(), v.as_str())).collect()
    }

    /// Sets a request query parameter, returns `true` if a param value is overriden
    pub fn param<K, V>(&mut self, key: K, value: V) -> bool 
    where K: Into<String>, V: Into<String> {
        self.params.insert(key.into(), value.into()).is_some()
    }

    /// Gets a param value, returns `None` if there is no param with this `key`
    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).as_ref().map(|s| s.as_str())
    } 

    /// Gets a `(&key, &value)` iterator of request query params
    pub fn param_iter(&self) -> KeyValueIter {
        KeyValueIter {
            iter: self.params.iter()
        }
    }
}

impl Deref for Request {
    type Target = HttpMessage;

    fn deref(&self) -> &HttpMessage {
        &self.base
    }
}

impl DerefMut for Request {
    fn deref_mut(&mut self) -> &mut HttpMessage {
        &mut self.base
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", self.method, &self.url)?;
        let headers = self.header_iter();
        for (key,value) in headers {
            writeln!(f, "{}={}", key, value)?;
        }
        Ok(())
    }
}

// HTTP Response status code
pub type HttpStatusCode = u16;

/// HTTP 100 CONTINUE status code
pub const HTTP_100_CONTINUE: u16 = 100;
/// HTTP 101 SWITCHING_PROTOCOLS status code
pub const HTTP_101_SWITCHING_PROTOCOLS: u16 = 101;
/// HTTP 200 OK status code
pub const HTTP_200_OK: u16 = 200;
/// HTTP 201 CREATED status code
pub const HTTP_201_CREATED: u16 = 201;
/// HTTP 202 ACCEPTED status code
pub const HTTP_202_ACCEPTED: u16 = 202;
/// HTTP 203 NON-AUTHORIZATIVE INFORMATION status code
pub const HTTP_203_NON_AUTHORIZATIVE_INFORMATION: u16 = 203;
/// HTTP 204 NO CONTENT status code
pub const HTTP_204_NO_CONTENT: u16 = 204;
/// HTTP 205 RESET CONTENT status code
pub const HTTP_205_RESET_CONTENT: u16 = 205;
/// HTTP 300 MULTIPLE CHOICES status code
pub const HTTP_300_MULTIPLE_CHOICES: u16 = 300;
/// HTTP 301 MOVED PERMANENTLY status code
pub const HTTP_301_MOVED_PERMANENTLY: u16 = 301;
/// HTTP 302 FOUND status code
pub const HTTP_302_FOUND: u16 = 302;
/// HTTP 303 SEE OTHER status code
pub const HTTP_303_SEE_OTHER: u16 = 303;
/// HTTP 305 RESET CONTENT status code
pub const HTTP_305_RESET_CONTENT: u16 = 305;
/// HTTP 307 TEMPORARY REDIRECT status code
pub const HTTP_307_TEMPORARY_REDIRECT: u16 = 307;
/// HTTP 400 BAD REQUEST status code
pub const HTTP_400_BAD_REQUEST: u16 = 400;
/// HTTP 401 UNAUTHORIZED status code
pub const HTTP_401_UNAUTHORIZED: u16 = 401;
/// HTTP 402 BAD REQUEST status code
pub const HTTP_402_FORBIDDEN: u16 = 402;
/// HTTP 404 NOT FOUND status code
pub const HTTP_404_NOT_FOUND: u16 = 404;
/// HTTP 405 METHOD NOT ALLOWED status code
pub const HTTP_405_METHOD_NOT_ALLOWED: u16 = 405;
/// HTTP 406 NOT ACCEPTABLE status code
pub const HTTP_406_NOT_ACCEPTABLE: u16 = 406;
/// HTTP 408 REQUEST_TIMEOUT status code
pub const HTTP_408_REQUEST_TIMEOUT: u16 = 408;
/// HTTP 409 CONFLICT status code
pub const HTTP_409_CONFLICT: u16 = 409;
/// HTTP 410 GONE status code
pub const HTTP_410_GONE: u16 = 410;
/// HTTP 411 LENGTH REQUIRED status code
pub const HTTP_411_LENGTH_REQURED: u16 = 411;
/// HTTP 413 PAYLOAD TOO LARGE status code
pub const HTTP_413_PAYLOAD_TOO_LARGE: u16 = 413;
/// HTTP 414 URI TOO LARGE status code
pub const HTTP_414_URI_TOO_LONG: u16 = 414;
/// HTTP 415 UNSUPORTED MEDIA TYPE status code
pub const HTTP_415_UNSUPORTED_MEDIA_TYPE: u16 = 415;
/// HTTP 417 EXPECTATION FAILED status code
pub const HTTP_417_EXPECTATION_FAILED: u16 = 417;
/// HTTP 426 UPGRADE REQUIRED status code
pub const HTTP_426_UPGRADE_REQUIRED: u16 = 426;
/// HTTP 500 INTERNAL_SERVE_ERROR status code
pub const HTTP_500_INTERNAL_SERVE_ERROR: u16 = 500;
/// HTTP 501 NOT IMPLEMENTED status code
pub const HTTP_501_NOT_IMPLEMENTED: u16 = 501;
/// HTTP 502 BAD_GATEWAY status code
pub const HTTP_502_BAD_GATEWAY: u16 = 502;
/// HTTP 503 SERVICE UNAVAILABLE status code
pub const HTTP_503_SERVICE_UNAVAILABLE: u16 = 503;
/// HTTP 504 GATEWAY TIMEOUT status code
pub const HTTP_504_GATEWAY_TIMEOUT: u16 = 504;
/// HTTP 505 HTTP VERSION NOT SUPPORTED status code
pub const HTTP_505_HTTP_VERSION_NOT_SUPPORTED: u16 = 505;



/// List of `Set-Cookie` headers in a HTTP Response
type SetCookies = Vec<SetCookie>;

/// HTTP Response
/// An HTTP Response is formed by:
/// * Status code
/// * (optional) Response headers
/// * (optional) Server's cookies (header `Set-Cookie`)
/// * (optional) Response body
/// 
/// For mor information see [HTTP Response](https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages#http_responses)
pub struct Response {
    /// Base Message
    base: HttpMessage,
    /// Response status code
    status_code: HttpStatusCode,
    /// Response cookies
    cookies: SetCookies,
    /// Authorize headers in HTTP `401 Not Authorized` responses
    auth: Vec<String>,
    /// Proxy authorize headers in HTTP `401 Not Authorized` responses
    proxy_auth: Vec<String>
}


impl Response {
    /// Response default constructor, only sets the status code.
    /// After constructing the value, as struct members are public, they can be
    /// accessed directly
    pub fn new(status: HttpStatusCode) -> Response {
        Response {
            base: HttpMessage::new(),
            cookies: SetCookies::new(),
            status_code: status,
            auth: Vec::new(),
            proxy_auth: Vec::new()        
        }
    }

    /// Get the Response status code
    pub fn status_code(&self) -> HttpStatusCode {
        self.status_code
    }

    /// Set Response cookie
    pub fn cookie(&mut self, value: SetCookie) {
        self.cookies.push(value)
    }

    /// Get Response SetCookies
    pub fn cookies(&self) -> Vec<&SetCookie> {
        self.cookies.iter().collect()
    }

    /// Adds an Authorization header guide
    pub fn auth<S: Into<String>>(&mut self, auth: S) {
        self.auth.push(auth.into());
    }

    /// Get Request Authorization headers
    pub fn auth_headers(&self) -> Vec<&str> {
        self.auth.iter().map(AsRef::as_ref).collect()
    }

    /// Adds a Proxy Authorization header guide
    pub fn proxy_auth<S: Into<String>>(&mut self, auth: S) {
        self.proxy_auth.push(auth.into());
    }

    /// Get Request Proxy Authorization headers
    pub fn proxy_auth_headers(&self) -> Vec<&str> {
        self.proxy_auth.iter().map(AsRef::as_ref).collect()
    }
}

impl Deref for Response {
    type Target = HttpMessage;

    fn deref(&self) -> &HttpMessage {
        &self.base
    }
}

impl DerefMut for Response {
    fn deref_mut(&mut self) -> &mut HttpMessage {
        &mut self.base
    }
}

#[cfg(test)]
mod test_request;

#[cfg(test)]
mod test_response;