#[macro_use] extern crate mime;
extern crate hyper;
extern crate hyper_native_tls;
extern crate url;

extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate clap;
extern crate rpassword;

use std::io::{Read, Write};


static BASE_URI: &'static str = "https://newsblur.com";

struct NewsBlurClient {
    client: hyper::Client,
    headers: hyper::header::Headers,
}

#[derive(Serialize, Deserialize, Debug)]
struct StarredStories {
    stories: Vec<serde_json::Value>
}

impl NewsBlurClient {
    pub fn login(&mut self, username: &str, password: Option<&str>) {
        use hyper::header::{Cookie, SetCookie};
        let mut params = vec![
            ("username", username),
        ];
        if let Some(password) = password {
            params.push(("password", password));
        }
        let body = {
            let mut serializer = url::form_urlencoded::Serializer::new("".to_string());
            serializer.extend_pairs(params);
            serializer.finish()
        };
        let response = self.client.post(&(BASE_URI.to_string() + "/api/login"))
            .headers(self.headers.clone())
            .body(&body)
            .send().unwrap();
        self.headers.set(Cookie(response.headers.get::<SetCookie>().unwrap().0.clone()));
        
    }
    pub fn get_starred_stories(&self, page: Option<u64>, story_hashes: Option<Vec<String>>) -> String {let mut params = vec![];
        if let Some(page) = page {
            params.push(("page", page.to_string()));
        }
        if let Some(story_hashes) = story_hashes {
            params.append(&mut story_hashes.iter().map(|s| ("h", s.clone())).collect::<Vec<_>>());
        }
        let body = {
            let mut serializer = url::form_urlencoded::Serializer::new("".to_string());
            serializer.extend_pairs(params);
            serializer.finish()
        };
        let mut response = self.client.post(&(BASE_URI.to_string() + "/reader/starred_stories"))
            .headers(self.headers.clone())
            .body(&body)
            .send().unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        body
    }
}

fn main() {
    let matches = {
        use clap::{App, Arg};
        App::new("get-newsblur")
            .version("0.0")
            .author("Colin Kinloch <colin@kinlo.ch>")
            .about("Gets NewsBlur")
            .arg(Arg::with_name("username")
                .short("u")
                .long("username")
                .help("NewsBlur username")
                .takes_value(true))
            .arg(Arg::with_name("password")
                .short("p")
                .long("password")
                .help("NewsBlur password")
                .takes_value(true))
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .help("Output file")
                .takes_value(true))
            .arg(Arg::with_name("force")
                .short("f")
                .long("force")
                .help("Overwrite output file"))
            .get_matches()
    };
    
    let client = {
        use hyper::Client;
        use hyper::net::HttpsConnector;
        use hyper_native_tls::NativeTlsClient;
        let tls = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(tls);
        Client::with_connector(connector)
    };
    let headers = {
        use hyper::header::{Headers, UserAgent, ContentType};
        let mut headers = Headers::new();
        headers.set(UserAgent("CERN-LineMode/2.15 libwww/2.17b3".to_string()));
        headers.set(ContentType(mime!(Application/WwwFormUrlEncoded)));
        headers
    };
    let mut nb_client = NewsBlurClient {
        client: client,
        headers: headers
    };
    
    let username = if let Some(username) = matches.value_of("username") {
        username.to_string()
    } else {
        print!("Username: ");
        std::io::stdout().flush().unwrap();
        let mut username = String::new();
        std::io::stdin().read_line(&mut username).unwrap();
        username.pop();
        username
    };
    
    let password = if let Some(password) = matches.value_of("password") {
        password.to_string()
    } else {
        rpassword::prompt_password_stdout("Password: ").unwrap()
    };
    
    // TODO: is no password == empty string?
    let password = if password.is_empty() { None } else { Some(password.as_str()) };
    
    // TODO: Wipe password from memory?
    nb_client.login(username.as_str(), password);
    
    let path = if let Some(output) = matches.value_of("output") {
        std::path::Path::new(output)
    } else {
        std::path::Path::new("starred_stories.json")
    };
    if !matches.is_present("force") && path.exists() {
        panic!("File {} exists", path.display());
    }
    let file = std::fs::File::create(path).unwrap();
    let mut all_starred_stories = Vec::new();
    let mut i = 0;
    loop {
        println!("Getting page {}", i);
        let body = nb_client.get_starred_stories(Some(i), None);
        let starred: StarredStories = serde_json::from_str(body.as_str()).unwrap();
        if starred.stories.is_empty() { break };
        all_starred_stories.append(&mut starred.stories.clone());
        let out = json!({
            "stories": all_starred_stories
        });
        serde_json::ser::to_writer(&file, &out).unwrap();
        i += 1;
    }
}
