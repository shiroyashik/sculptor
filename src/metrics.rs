
use std::{sync::LazyLock, time::Instant};

use axum::{body::Body, extract::State, http::{Request, Response}, middleware::Next, routing::get, Router};
use prometheus::{proto::{Metric, MetricType}, register_histogram_vec, register_int_counter};
use reqwest::StatusCode;

use crate::state::AppState;

pub fn metrics_router(enabled: bool) -> Router<AppState> {
    if !enabled { return Router::new(); }
    tracing::info!("Metrics enabled! You can access them on /metrics");
    Router::new()
        .route("/metrics", get(metrics))
}

async fn metrics(State(state): State<AppState>) -> String {
    let mut metric_families = prometheus::gather();
    
    // Add new custom metrics
    let players = {
        let mut metric = prometheus::proto::Metric::default();
        metric.set_gauge(prometheus::proto::Gauge::default());
        metric.mut_gauge().set_value(state.session.len() as f64);
        create_mf("sculptor_players_count".to_string(), "Number of players".to_string(), MetricType::GAUGE, metric)
    };

    metric_families.push(players);

    prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .unwrap()
}

#[inline]
fn create_mf(name: String, help: String, field_type: MetricType, metric: Metric) -> prometheus::proto::MetricFamily {
    let mut mf = prometheus::proto::MetricFamily::default();
    mf.set_name(name);
    mf.set_help(help);
    mf.set_field_type(field_type);
    mf.mut_metric().push(metric);
    mf
}

pub async fn track_metrics(req: Request<Body>, next: Next) -> Result<Response<Body>, StatusCode> {
    let start = Instant::now();
    let uri = req.uri().path().to_string();

    // Call the next middleware or handler
    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();

    REQUESTS
        .with_label_values(&[&uri, response.status().as_str()])
        .observe(latency);

    Ok(response)
}

pub static REQUESTS: LazyLock<prometheus::HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!("sculptor_requests_count", "Number of requests", &["uri", "code"], vec![0.025, 0.250, 0.500]).unwrap()
});

pub static PINGS: LazyLock<prometheus::HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!("sculptor_pings_count", "Number of pings", &["type"], vec![0.000003, 0.00002, 0.0002]).unwrap()
});

pub static PINGS_ERROR: LazyLock<prometheus::IntCounter> = LazyLock::new(|| {
    register_int_counter!("sculptor_pings_error", "Number of ping decoding errors").unwrap()
});