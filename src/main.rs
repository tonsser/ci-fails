use lazy_static::lazy_static;
use quicli::prelude::*;
use regex::Regex;
use reqwest;
use serde_json::Value;
use std::collections::HashSet;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
    /// Build number
    #[structopt(long = "build-num", short = "b")]
    build_num: usize,

    /// Circleci token
    ///
    /// This argument is optional, if not provided it will look for a CIRCLECI_TOKEN environment
    /// variable
    #[structopt(long = "token", short = "t")]
    token: Option<String>,
}

fn main() -> CliResult {
    let args = Cli::from_args();

    let token = if let Some(token) = args.token {
        token
    } else {
        use std::env;
        env::var("CIRCLECI_TOKEN")
            .expect("Missing --token argument,or CIRCLECI_TOKEN environment variable")
    };

    let url = format!(
        "https://circleci.com/api/v1.1/project/github/tonsser/tonsser-api/{build_num}?circle-token={token}",
        build_num = args.build_num,
        token = token,
    );
    let body: Value = reqwest::get(&url).unwrap().json().unwrap();

    let output_urls = body["steps"]
        .as_array()
        .unwrap()
        .into_iter()
        .flat_map(|step| step["actions"].as_array().unwrap())
        .filter(|action| action["status"] != "success")
        .filter(|action| action["name"] == "script/ci/run-with-retries")
        .map(|action| action["output_url"].as_str().unwrap());

    let mut acc = HashSet::<String>::new();

    for url in output_urls {
        let json = reqwest::get(url)
            .unwrap()
            .json::<Value>()
            .unwrap()
            .as_array()
            .unwrap()
            .clone();

        let outputs = json
            .into_iter()
            .find(|thing| thing["type"] == "out")
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let test_output = outputs["message"].as_str().unwrap();
        let mut lines = test_output.lines().collect::<Vec<_>>();
        lines.reverse();

        for line in lines {
            if line.contains("Failed examples") {
                break;
            }

            if line.starts_with("rspec ") {
                acc.insert(test_file(line));
            }
        }
    }

    for line in acc {
        println!("{}", line);
    }

    Ok(())
}

fn test_file(line: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^rspec (?P<file>[^:\[]+)").unwrap();
    }

    let caps = RE.captures(line).unwrap();
    caps["file"].to_string().replace("'", "").replace("./", "")
}
