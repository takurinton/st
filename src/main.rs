use reqwest;
use select::document::Document;
use select::predicate::Name;
use std::collections::{HashSet, HashMap};

fn get_urls() -> Vec<String> {
    // コマンドを実行するディレクトリからの相対パス
    let sites = match std::fs::read_to_string("url_list.txt") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading url_list.txt: {}", e);
            std::process::exit(1);
        }
    };
    sites.split("\n").map(|s| s.to_string()).collect()
}

#[tokio::main]
async fn main() {
    let urls = get_urls();

    let mut results = HashMap::new();

    for url in urls {
        match get_technologies(&url).await {
            Ok(technologies) => {
                results.insert(url.to_string(), technologies);
            }
            Err(e) => eprintln!("Error processing {}: {}", url, e),
        }
    }

    for (url, technologies) in results.iter() {
        println!("URL: {}", url);
        for tech in technologies {
            println!("  - {}", tech);
        }
    }
}

async fn get_technologies(url: &str) -> Result<HashSet<String>, reqwest::Error> {
    let resp = reqwest::get(url).await?.text().await?;
    let document = Document::from(resp.as_str());

    let mut technologies = HashSet::new();

    if document.find(Name("script")).any(|n| n.attr("src").unwrap_or("").contains("next")) {
        technologies.insert("Next.js".to_string());
    }
    // and more

    Ok(technologies)
}
