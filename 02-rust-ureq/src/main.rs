use std::env;
use std::error::Error;
use std::time::Instant;
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
        .build();

    // This is call behind a proxy
    let resp = agent.get(&url).call()?.into_string()?;

    println!("HTML of the page: {resp}");
    println!("End of content\n");

    let links = scrape_search_result(&resp);
    println!("List of links: {links:?}");

    let start = Instant::now();
    dbg!(scrape_products(agent.clone(),links.clone())?.len());
    println!("Duration in sequential {:.3}s", Instant::now().duration_since(start).as_secs_f64());

    let start = Instant::now();
    dbg!(scrape_products_par(agent.clone(),links.clone())?.len());
    println!("Duration in parallel {:.3}s", Instant::now().duration_since(start).as_secs_f64());

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

// sequential
fn scrape_products(agent: Agent, list: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let responses = list.iter().map(|url| agent.get(url).call()).collect::<Vec<_>>();
    let mut contents = vec![];

    for response in responses {
        let res = response?.into_string()?;
        contents.push(res);
    }

    Ok(contents)
}

// parallel
fn scrape_products_par(agent: Agent, list: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let responses = list.par_iter().map(|url| agent.get(url).call()).collect::<Vec<_>>();
    let mut contents = vec![];

    for response in responses {
        let res = response?.into_string()?;
        contents.push(res);
    }

    Ok(contents)
}
