#[derive(Default)]
pub enum HttpMethods {
    #[default]
    POST,
    GET,
    DELETE,
    PATCH,
}

impl HttpMethods {
    pub fn from(string: String) -> HttpMethods {
        return match string.as_ref() {
            "POST" => HttpMethods::POST,
            "GET" => HttpMethods::GET,
            "DELETE" => HttpMethods::DELETE,
            "PATCH" => HttpMethods::PATCH,
            _ => panic!("Please provide a valid HTTP method"),
        };
    }
}
