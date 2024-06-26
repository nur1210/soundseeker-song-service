use metrics::{describe_counter, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn start_prometheus_exporter() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()
        .expect("failed to install Prometheus recorder");

    describe_counter!("grpc_requests_total", "grpc requests");
    describe_histogram!(
        "grpc_request_latency_seconds_bucket",
        "grpc request latency"
    )
}
