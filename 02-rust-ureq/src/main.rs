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

    let links = scrape_search_result(&resp);
    println!("List of links: {links:?}");

    let start = Instant::now();
    // dbg!(scrape_products(agent.clone(),links.clone())?.len());
    println!("Duration in sequential {:.3}s", Instant::now().duration_since(start).as_secs_f64());

    let start = Instant::now();
    let res = scrape_products_par(agent.clone(),links.clone());
    println!("Duration in parallel {:.3}s", Instant::now().duration_since(start).as_secs_f64());
    println!("success: {}/{}", res.iter().filter_map(|x| x.as_ref().ok()).count(), res.len());

    println!("Error details:");
    res.iter().filter_map(|x| x.as_ref().err()).for_each(|e| println!("{}", e));

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
fn scrape_products_par(agent: Agent, list: Vec<String>) -> Vec<Result<String, std::io::Error>> {
    let responses = list.par_iter().map(|url| agent.get(url).call()).collect::<Vec<_>>();
    let mut contents = vec![];

    for response in responses {
        let res = response
            .map_err(|e| std::io::Error::new(ErrorKind::Other,e))
            .and_then(|r|r.into_string());

        contents.push(res);
    }

    contents
}
