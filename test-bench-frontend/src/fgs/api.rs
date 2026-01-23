use gloo_net::http::Request;

pub use test_bench_shared::{FgsError, FgsServerClient};

/// Check if a URL returns a successful response (for image loading).
pub async fn check_url_ok(url: &str) -> bool {
    match Request::get(url).send().await {
        Ok(response) => response.ok(),
        Err(_) => false,
    }
}

/// Fetch text content from a URL, returning None on failure.
pub async fn fetch_text(url: &str) -> Option<String> {
    let response = Request::get(url).send().await.ok()?;
    response.text().await.ok()
}

/// Calculate exponential backoff delay for retry logic.
pub fn calculate_backoff_delay(failure_count: u32, base_delay: u32, max_delay: u32) -> u32 {
    if failure_count == 0 {
        base_delay
    } else {
        let exponential_delay = base_delay * 2_u32.pow(failure_count.min(10));
        exponential_delay.min(max_delay)
    }
}
