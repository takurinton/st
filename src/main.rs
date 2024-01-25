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

// <script id="__NEXT_DATA__" type="application/json">、または <script src="/_next/static/..."> があるかどうかで判断
// MEMO: SSG, SSR, app router, or CSR の区別はこれだけではできない
async fn is_next_js(document: Document) -> Result<bool, reqwest::Error> {
    if document
        .find(Name("script"))
        .any(|n| n.attr("id").unwrap_or("") == "__NEXT_DATA__")
    {
        return Ok(true);
    }

    if document
        .find(Name("script"))
        .any(|n| n.attr("src").unwrap_or("").starts_with("/_next/static"))
    {
        return Ok(true);
    }

    Ok(false)
}

// fetch した JS のなかに `@license React` があるかどうかで判断
async fn is_react(document: Document, url: &str) -> Result<bool, reqwest::Error> {
    let mut js_urls = document
        .find(Name("script"))
        .filter_map(|n| n.attr("src"))
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    for js_url in js_urls.iter_mut() {
        if !js_url.starts_with("http") {
            js_url.insert_str(0, url);
        }
    }

    let mut react = false;
    for js_url in js_urls {
        let js = reqwest::get(&js_url).await?.text().await?;
        if js.contains("@license React") {
            react = true;
            break;
        }
    }

    Ok(react)
}

// <div id="___gatsby"> があるかどうかで判断
async fn is_gatsby(document: Document) -> Result<bool, reqwest::Error> {
    if document.find(Name("div")).any(|n| n.attr("id").unwrap_or("") == "___gatsby") {
        return Ok(true);
    }

    Ok(false)
}

async fn get_technologies(url: &str) -> Result<HashSet<String>, reqwest::Error> {
    let resp = reqwest::get(url).await?.text().await?;
    let document = Document::from(resp.as_str());

    let mut technologies = HashSet::new();

    // Next.js
    if is_next_js(document.clone()).await? {
        technologies.insert("Next.js".to_string());
    }

    if is_react(document.clone(), url).await? {
        technologies.insert("React".to_string());
    }

    // Gatsby
    if is_gatsby(document.clone()).await? {
        technologies.insert("Gatsby".to_string());
    }

    // and more

    Ok(technologies)
}
