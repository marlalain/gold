use std::borrow::Borrow;

pub enum HttpMethods {
	NOTHING,
	POST,
	GET,
	DELETE,
}

impl HttpMethods {
	pub fn from(string: String) -> HttpMethods {
		return match string.as_str() {
			"POST" => HttpMethods::POST,
			"GET" => HttpMethods::GET,
			"DELETE" => HttpMethods::DELETE,
			_ => panic!("Please provide a valid HTTP method")
		};
	}
}