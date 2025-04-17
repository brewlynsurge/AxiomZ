pub struct Info {
    name: String,
    version: String,
    part: String
}

impl Info {
    pub fn new(part: &str) -> Self {
        let version = env!("CARGO_PKG_VERSION");

        Self {
            name: String::from("SurfX"),
            version: String::from(version),
            part: String::from(part)
        }
    }

    pub fn display_head(&self) {
        println!("{} [{}]", self.part, self.version);
        println!("(c) {}. All rights are reserved.\n", self.name);
    }
}