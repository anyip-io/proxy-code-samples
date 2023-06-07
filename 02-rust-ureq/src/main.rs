use std::env;
use std::error::Error;
use std::io::ErrorKind;
use std::time::{Duration, Instant};
use select::document::Document;
use select::predicate::{Class, Name, Predicate};
use rayon::prelude::*;
use ureq::Agent;

fn main() -> Result<(), Box<dyn Error>> {
    let keyword = "boxing gloves";
    let url = format!("https://www.walmart.com/search?q={keyword}");
    let proxy_endpoint = env::var("PROXY_ENDPOINT").expect("`PROXY_ENDPOINT` env var should be specified");
    println!("Target URL: {url}");

    let proxy = ureq::Proxy::new(proxy_endpoint)?;
    let agent = ureq::AgentBuilder::new()
        .proxy(proxy)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        .timeout(Duration::from_secs(10))
        .build();

    // This is call behind a proxy
    let resp = agent.get(&url).call()?.into_string()?;

    // println!("HTML of the page: {resp}");
    // println!("End of content\n");

    let mut links = scrape_search_result(&resp);
    println!("List of links: {links:?}");

    let mut retries = 0;

    while retries < 3 {
        let products = scrape_products_par(agent.clone(), links.clone());
        println!("success: {}/{}", products.iter()
            .filter_map(|(url, res)| res.as_ref().ok())
            .count(), products.len()
        );

        println!("Error details:");
        links = products.iter()
            .filter(|(x, res)| res.is_err())
            .map(|(url, res)| url.to_owned())
            .collect::<Vec<_>>();

        if links.len() == 0 {
            // if there's no link's left, we have scraped the entire list of products
            break;
        }

        retries += 1;
    }


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

// parallel
fn scrape_products_par(agent: Agent, list: Vec<String>) -> Vec<(String, Result<String, std::io::Error>)> {
    let responses = list.par_iter().map(|url| (url.clone(), agent.get(url).call())).collect::<Vec<_>>();
    let mut contents = vec![];

    for (url, response) in responses {
        let res = response
            .map_err(|e| std::io::Error::new(ErrorKind::Other,e))
            .and_then(|r| r.into_string());

        contents.push((url, res));
    }

    contents
}
