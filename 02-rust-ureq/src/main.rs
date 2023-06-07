use std::env;
use std::error::Error;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};

fn main() -> Result<(), Box<dyn Error>> {
    let keyword = "boxing gloves";
    let url = format!("https://www.walmart.com/search?q={keyword}");
    let proxy_endpoint = env::var("PROXY_ENDPOINT").expect("`PROXY_ENDPOINT` env var should be specified");
    println!("Target URL: {url}");

    let proxy = ureq::Proxy::new(proxy_endpoint)?;
    let agent = ureq::AgentBuilder::new()
        .proxy(proxy)
        .build();

    // This is call behind a proxy
    let resp = agent.get(&url).call()?.into_string()?;

    println!("HTML of the page: {resp}");
    println!("End of content\n");

    let links = scrape_search_result(&resp);
    println!("List of links: {links:?}");

    Ok(())
}

fn scrape_search_result(resp: &str) -> Vec<String> {
    let mut cpt = 0;
    let mut links = vec![];
    let document = Document::from(resp);

    // This is the path we would like to select: `.pb1-xl a.absolute`
    for node in document.find(Class("pb1-xl")) {
        for link in node.find(Name("a").and(Class("absolute"))) {
            cpt += 1;

            println!("#{cpt} {} ({:?})\n", link.text(), link.attr("href").unwrap());
            let href = link.attr("href").unwrap();

            if href.contains("walmart.com/") {
                links.push(href.to_owned());
            } else {
                // some url may not contain the full url, so we add the prefix
                links.push(format!("https://www.walmart.com{href})"));
            }
        }
    }

    links
}
