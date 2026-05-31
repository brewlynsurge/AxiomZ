pub struct UrlFrontier;

impl UrlFrontier {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_url(&self) -> String {
        return String::from("https://en.wikipedia.org/wiki/Main_Page")
    }
}