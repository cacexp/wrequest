// Copyright 2022 Juan A. Cáceres (cacexp@gmail.com)
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
//! request.insert_param("client_id", "1234");
//! 
//! // Add request headers
//! request.insert_header("Content-Type", "application/json")
//!        .insert_header("Accept", "application/json");
//! 
//! // Add a request cookie
//! request.insert_cookie("session", "1234");
//! 
//! // Add a JSON Object as request body
//! let data = object! {
//!    name: "John",
//!    surname: "Smith"
//! };
//! 
//! // JSON Object is encoded at the body
//! request.set_json(&data);
//! 
//! assert_eq!(request.headers().get("Content-Type").unwrap(), "application/json" );
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
//! response.insert_header("Content-Type", "application/json");
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
//! response.insert_cookie(cookie);
//! 
//! // JSON Object is encoded at the body
//! response.set_json(&data);
//! 
//! assert_eq!(response.headers().get("Content-Type").unwrap(), "application/json" );
//! 
//! ```
//!  
//! # Future Features
//! * Multipart
//!  

#![allow(dead_code)]

use url::Url;
use unicase::UniCase;
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use std::str::from_utf8;
use std::io::{ErrorKind, Error};
use json::JsonValue;
use std::fmt;
use std::collections::HashMap;
use std::iter::Iterator;
use wcookie::SetCookie;
use std::ops::{Deref, DerefMut};


/// `Content-Type` header name
pub const CONTENT_TYPE: &str = "Content-Type";
/// `Content-Type` header value for JSON encoded in UTF-8
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

/// Map of HTTP message headers. Header keys are case-insensitive.
pub struct HeaderMap {
    map : CaseInsensitiveHashMap<String>
}

impl HeaderMap {
    /// Constructor
    pub fn new() -> HeaderMap {
        HeaderMap {
            map: CaseInsensitiveHashMap::new()
        }
    }

    /// Insert a header with `key` and `value`. Returns `true` if there was a previous header with the same `key`.
    pub fn insert<K,V>(&mut self, key: K, value: V) -> bool
    where K: Into<String>,
          V: Into<String> {
        self.map.insert(key.into(),value.into()).is_some()
    }
   
    /// Returns `true` if there is a header with `key`. Note keys are case-insensitive.
    pub fn contains_key<K>(&self, key: K) -> bool
    where
        K: Into<String>
    {
        self.map.contains_key(key.into())
    }

    /// Gets a reference to the header value if any.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }

    /// Gets an iterator to a tuple of `(key, value)`
    pub fn iter(&self) -> HeaderIter {
        HeaderIter {
            iter: self.map.iter()
        }
    }
}

impl From<Vec<(String, String)>> for HeaderMap {
    ///Converts a `Vec<(String, String)>` to a `HeaderMap`. It takes ownership of contained `String` values.
    fn from(value: Vec<(String, String)>) -> Self { 
        let mut owned = value;
        let mut result = HeaderMap::new();
        loop {
            if let Some((k,v)) = owned.pop() {
                result.insert(k,v);
            } else {
                break;
            }
        }

        result
    }
}

impl From<&Vec<(&str, &str)>> for HeaderMap {
    ///Converts a `Vec<(&str, &str)>` to a `HeaderMap`
    fn from(value: &Vec<(&str, &str)>) -> Self { 
        let mut result = HeaderMap::new();
        for (k, v) in value.iter() {
            result.insert(*k,*v);
        }
        result
    }
}


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


/// Base struct for Request params and cookies. Keys are case-sensitive.
pub struct KeyValueMap {
    map : HashMap<String, String>
}

impl KeyValueMap {
    /// Constructor
    pub fn new() -> KeyValueMap {
        KeyValueMap {
            map: HashMap::new()
        }
    }
    /// Insert a `key`/`value`
    pub fn insert<K,V>(&mut self, key: K, value: V) -> bool
    where K: Into<String>,
          V: Into<String> {
        self.map.insert(key.into(),value.into()).is_some()
    }

    /// Gets the `value` assotiated to a `key`, if any.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }

    /// Checks the map contains a value with `key`
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Generates an interator to `(key, value)`
    pub fn iter(&self) -> KeyValueIter {
        KeyValueIter {
            iter: self.map.iter()
        }
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

/// Base message struct for Request and Response
pub struct HttpMessage {
    /// Request headers
    headers: HeaderMap,
    /// Request body (not implemented multi-part yet)
    body: MessageBody
}

impl HttpMessage {
    /// Constructor
    pub fn new() -> HttpMessage {
        HttpMessage {
            headers : HeaderMap::new(),
            body: MessageBody::None
        }
    }

    /// Inserts a header with `key` and `value`
    pub fn insert_header<K,V>(&mut self, key: K, value: V) -> &mut Self
    where K: Into<String>,
          V: Into<String> {
        self.headers.insert(key, value);
        self
    } 
    
    /// Gets the headers map
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Gets a mutable reference to the headers map
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
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
    pub fn set_body(&mut self, data: Vec<u8>) -> &mut Self {
        self.body = MessageBody::Single(data);
        self
    }

    /// Gets body data if any, returns `None` if there is no single body
    pub fn body (&self) -> Option<&Vec<u8>> {
        if let MessageBody::Single(ref body) = self.body {
            Some(body)
        } else {
            None
        }
    }

     /// Sets a json object as request body. The `data` object is marshaled into a buffer using UTF8 coding.
     /// Returns `true` if request body is overriden
     pub fn set_json(&mut self, data: &JsonValue) -> &mut Self {
        let pretty = data.pretty(4);
        self.headers.insert(CONTENT_TYPE, APPLICATION_JSON);
        self.set_body(pretty.into_bytes())
    }

    /// Checks if the Response has body and tries to parse as a `json::JsonValue'
    pub fn json(&self) -> Result<JsonValue, Error> {
        if ! self.body.is_single() {
            return Err(Error::new(ErrorKind::InvalidData, "Empty body"));
        }

        let str_body = from_utf8(self.body().unwrap());

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
/// * (optional) Request body
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
    /// Original URL as string
    target: String,
    //  Target URL, is none if not parsed properly
    url: Option<Url>,
    /// Request Cookies
    cookies: KeyValueMap,
    /// Request params
    params: KeyValueMap  
}

impl Request {

    /// Hidden constructor
    pub fn new<S>(method: HttpMethod, url:S) -> Request 
    where S: Into<String>
    {
        let target = url.into();
        let parsed_url = Url::parse(target.as_str());

        Request {
            base: HttpMessage::new(),
            method,
            target,
            url: parsed_url.ok(),
            cookies: KeyValueMap::new(),
            params: KeyValueMap::new()
        }
    }

    /// Creates a `CONNECT` request builder
    pub fn connect<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::CONNECT, url)
    }

    /// Creates a `DELETE` request builder
    pub fn delete<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::DELETE, url)
    }

    /// Creates a `GET` request builder
    pub fn get<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::GET, url)
    }

    /// Creates a `HEAD` request builder
    pub fn head<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::HEAD, url)
    }

    /// Creates a `OPTIONS` request builder
    pub fn options<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::OPTIONS, url)
    }

    /// Creates a `PATCH` request builder
    pub fn patch<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::PATCH, url)
    }

    /// Creates a `POST` request builder
    pub fn post<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::POST, url)
    }

    /// Creates a `PUT` request builder
    pub fn put<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::PUT, url)
    }
    
    /// Creates a `TRACE` request builder
    pub fn trace<S>(url: S) -> Request 
    where S: Into::<String>
    {
        Self::new(HttpMethod::TRACE, url)
    }

    /// Gets HTTP Method
    pub fn method(&self) -> HttpMethod {
        self.method
    }

    /// Insert a request param with `key` and `value`. Param keys are case-sensitive.
    pub fn insert_param<K, V>(&mut self, key: K, value: V) -> &mut Self 
    where K: Into<String>,
          V: Into<String>
    {
        self.params.insert(key, value);
        self
    }

    /// Gets a params map reference
    pub fn params(&self) -> &KeyValueMap {
        &self.params
    }

    /// Gets a mutable params map reference
    pub fn params_mut(&mut self) -> &mut KeyValueMap {
        &mut self.params
    }

    /// Insert a cookie param with `key` and `value`. Param keys are case-sensitive.
    pub fn insert_cookie<K, V>(&mut self, key: K, value: V) -> &mut Self 
    where K: Into<String>,
          V: Into<String>
    {
        self.cookies.insert(key, value);
        self
    }

    /// Gets a cookie map reference
    pub fn cookies(&self) -> &KeyValueMap {
        &self.cookies
    }

    /// Gets a mutable params map reference
    pub fn cookies_mut(&mut self) -> &mut KeyValueMap {
        &mut self.params
    }

    /// Gets the target URL as an string
    pub fn target(&self) -> &str {
        self.target.as_str()
    }
    
    /// Gets `url::Url` reference if `target` is well-formed
    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
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
        writeln!(f, "{} {}", self.method, &self.target)?;
        let headers = self.headers.iter();
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
/// 
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
    /// Inserts a Response cookie
    pub fn insert_cookie(&mut self, value: SetCookie) -> &mut Self {
        self.cookies.push(value);
        self
    }

    /// Get Response SetCookies
    pub fn cookies(&self) -> Vec<&SetCookie> {
        self.cookies.iter().collect()
    }

    /// Adds an Authorization header guide
    pub fn insert_auth_headers<S: Into<String>>(&mut self, auth: S) -> &mut Self {
        self.auth.push(auth.into());
        self
    }

    /// Get a reference of Response Authorization headers
    pub fn auth_headers(&self) -> &Vec<String> {
        &self.auth
    }

    /// Get a mutable reference of the Response Authorization headers
    pub fn auth_headers_mut(&mut self) -> &mut Vec<String> {
        &mut self.auth
    }

    /// Adds a Proxy Authorization header guide
    pub fn insert_proxy_auth_header<S: Into<String>>(&mut self, auth: S) -> &mut Self {
        self.proxy_auth.push(auth.into());
        self
    }

    /// Get a reference of Proxy Authorization headers
    pub fn proxy_auth_headers(&self) -> &Vec<String> {
        &self.proxy_auth
    }

    /// Get a mutable reference of Proxy Authorization headers
    pub fn proxy_auth_headers_mut(&mut self) -> &mut Vec<String> {
        &mut self.proxy_auth
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