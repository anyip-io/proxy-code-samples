use reqwest::header::CONTENT_TYPE;
use reqwest::{header, Client};
use scraper::{Html, Selector};
use std::error::Error;
use std::time::{Duration, Instant};

/// The entry point, we create a tokio runtime and tell rust the function contains async functions
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let keyword = "bean bags";
    let url = format!("https://www.amazon.com/s?k={keyword}&ref=nb_sb_noss");

    /// build the http client
    let client = build_http_client(
        "http://portal.anyip.io:1080",
        "user-XXXX,country-fr",
        "yourpassword",
    );

    // should get the products links from the search result
    let links = paginate(&client, &url).await?;
    let mut products = vec![];

    for link in links {
        if let Some(target_url) = link {
            // extract products details concurrently by spawning 1 task per product
            products.push(extract_product_detail(client.clone(), target_url));
        }
    }

    let start = Instant::now();
    let products: Vec<Result<Product, Box<dyn Error>>> = futures::future::join_all(products).await;
    let entire_time_elapsed = Instant::now() - start;
    let mut duration_per_product = vec![];

    for product in products {
        match product {
            Ok(res) => {
                println!("Product name {}", res.name);
                duration_per_product.push(res.elapsed);
            }
            Err(err) => eprintln!("Error: {}", err), // just print the error out
        }
        println!("----------------------");
    }
    println!("Finished in {}s", entire_time_elapsed.as_secs());
    let average = duration_per_product.iter().sum::<f64>() / duration_per_product.len() as f64;
    println!("Average time / req: {:.2}s", average);

    // exit the program
    Ok(())
}

/// Go through the listing and get all the links
async fn paginate(client: &Client, url: &str) -> Result<Vec<Option<String>>, Box<dyn Error>> {
    let mut links = vec![];
    println!("Paginating...");

    // send a HTTP GET request
    let res = client
        .get(url)
        // .get("https://httpbin.org/anything")
        .send()
        .await? // this make the request asynchronous
        .error_for_status()? // will trigger an error if the HTTP status code is >= 400
        .text() // return the text in order to print it out to the console
        .await?; // this will aggregate the body chunks into the text content, all asynchronously.
                 // dbg!(&res);

    // Parse the document's DOM
    let document = Html::parse_document(&res);
    // select the items within' the listing
    let selector = Selector::parse(r#"div[data-component-type="s-search-result"] div.s-widget-container a.a-link-normal.s-no-outline"#).unwrap();

    // iterate over each links of the listing
    for link in document.select(&selector) {
        // extract the product link from it
        let href = link.value().attr("href");
        links.push(href.map(|m| m.to_owned()));
    }

    println!("{} products found within' the listing", links.len());

    Ok(links)
}

async fn extract_product_detail(client: Client, url: String) -> Result<Product, Box<dyn Error>> {
    let url = format!("https://www.amazon.com{}", url);
    let start = Instant::now();

    // send a HTTP GET request
    let res = client
        .get(&url)
        .send()
        .await? // this make the request asynchronous
        .error_for_status()? // will trigger an error if the HTTP status code is >= 400
        .text() // return the text in order to print it out to the console
        .await?; // this will aggregate the body chunks into the text content, all asynchronously.

    let elapsed = (Instant::now() - start).as_secs_f64();

    // Parse the document's DOM
    let document = Html::parse_document(&res);

    // Then scrape the Product
    let product = Product::scrape(document, url, elapsed);

    Ok(product)
}

struct Product {
    url: String,
    name: String,
    comments: Vec<String>,
    elapsed: f64,
}

impl Product {
    pub fn scrape(document: Html, url: String, elapsed: f64) -> Self {
        let mut comments = vec![];

        // select the items within' the listing
        let selector =
            Selector::parse(r#"div.a-section.review div.a-spacing-small.review-data"#).unwrap();

        // iterate over each comments about the product
        for comment in document.select(&selector) {
            // get the text of that element, then join the pieces together
            // to make a single String per comment
            let comment_txt = comment.text().collect::<Vec<_>>().join("");
            comments.push(comment_txt.trim().to_owned());
        }

        // find the title
        let name = document
            .select(&Selector::parse(r#"span#productTitle"#).unwrap())
            .next()
            .expect("product name is missing")
            .text()
            .collect::<Vec<_>>()
            .join("");

        Self {
            name,
            comments,
            url,
            elapsed,
        }
    }
}

/// Build the reqwest http client with the proxy info setup
fn build_http_client(proxy_endpoint: &str, username: &str, password: &str) -> Client {
    // Add the `content-type` on every requests
    let mut headers = header::HeaderMap::new();
    headers.insert(CONTENT_TYPE, header::HeaderValue::from_static("text/html"));

    reqwest::Client::builder()
        .pool_idle_timeout(None)
        .http1_only()
        .proxy(
            reqwest::Proxy::all(proxy_endpoint)
                .expect("wrong proxy endpoint")
                .basic_auth(&username, &password),
        )
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Safari/605.1.1",
        )
        .build()
        .expect("http client build error")
}
