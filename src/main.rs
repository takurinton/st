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

struct Libs {
    react: bool,
    vue: bool,
}

// fetch した JS のなかに `@license React` があるかどうかで判断
// fetch した JS のなかに `@vue/` があるかどうかで判断
async fn is_react_vue(document: Document, url: &str) -> Result<Libs, reqwest::Error> {
    let mut libs = Libs {
        react: false,
        vue: false,
    };

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

    for js_url in js_urls {
        let js = reqwest::get(&js_url).await?.text().await?;
        if js.contains("@license React") {
            println!("found react in {}", url);
            libs.react = true;
        } else if js.contains("@vue/") {
            println!("found vue in {}", url);
            libs.vue = true;
        } else if js.contains("Vue.js v") {
            libs.vue = true;
        }
    }

    Ok(libs)
}

// <div id="___gatsby"> があるかどうかで判断
async fn is_gatsby(document: Document) -> Result<bool, reqwest::Error> {
    if document.find(Name("div")).any(|n| n.attr("id").unwrap_or("") == "___gatsby") {
        return Ok(true);
    }

    Ok(false)
}

// <meta name="generator" content="WordPress" /> だったら wordpress
// <meta name="generator" content="Vitepress" /> だったら vitepress
// <meta name="generator" content="VuePress" /> だったら vuepress
// <meta name="generator" content="Hugo" /> だったら hugo
async fn is_ssg(document: Document) -> Result<HashSet<String>, reqwest::Error> {
    let d = document    .find(Name("meta"));
    let mut technologies = HashSet::new();

    for n in d {
        if n.attr("name").unwrap_or("") == "generator" {
            if n.attr("content").unwrap().contains("WordPress") {
                technologies.insert("WordPress".to_string());
            }
            if n.attr("content").unwrap().contains("Vitepress") {
                technologies.insert("Vitepress".to_string());
            }
            if n.attr("content").unwrap().contains("VuePress") {
                technologies.insert("VuePress".to_string());
            }
            if n.attr("content").unwrap().contains("Hugo") {
                technologies.insert("Hugo".to_string());
            }
        }
    }

    Ok(technologies)
}


async fn is_nuxt(document: Document) -> Result<bool, reqwest::Error> {
    if document.find(Name("div")).any(|n| n.attr("id").unwrap_or("") == "__nuxt") {
        return Ok(true);
    }

    if document
        .find(Name("script"))
        .any(|n| n.attr("id").unwrap_or("") == "__NUXT_DATA__")
    {
        return Ok(true);
    }

    if document
        .find(Name("script"))
        .any(|n| n.attr("src").unwrap_or("").starts_with("/_nuxt/"))
    {
        return Ok(true);
    }

    Ok(false)
}

async fn get_technologies(url: &str) -> Result<HashSet<String>, reqwest::Error> {
    let resp = reqwest::get(url).await?.text().await?;
    let document = Document::from(resp.as_str());

    let mut technologies = HashSet::new();

    println!("processing {}", url);

    // Next.js
    if is_next_js(document.clone()).await? {
        technologies.insert("Next.js".to_string());
    }

    // Gatsby
    if is_gatsby(document.clone()).await? {
        technologies.insert("Gatsby".to_string());
    }

    // ssg libs
    let ssg_libs = is_ssg(document.clone()).await?;
    for lib in ssg_libs {
        technologies.insert(lib.to_string());
    }


    // Nuxt
    if is_nuxt(document.clone()).await? {
        technologies.insert("Nuxt.js".to_string());
    }

    // react or vue
    let libs = is_react_vue(document.clone(), url).await?;
    if libs.react {
        technologies.insert("React".to_string());
    }
    if libs.vue {
        technologies.insert("Vue.js".to_string());
    }

    // and more

    Ok(technologies)
}
