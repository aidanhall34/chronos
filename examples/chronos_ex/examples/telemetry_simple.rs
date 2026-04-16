use opentelemetry::trace::TracerProvider as _;

use opentelemetry_sdk::{runtime::Tokio, trace::TracerProvider};
use tracing::{info_span, instrument};

use tokio::time::Duration;

struct SubRunner {}

impl SubRunner {
    pub fn run(&self) {
        let span_sub_runer = info_span!("sub_hello_world 2");
        let _guard = span_sub_runer.enter();
        println!("Hello, under world!");
        let sub_runner_one = SubRunnerOne {};
        sub_runner_one.run();
    }

    #[instrument(name = "run_db2", skip(self))]
    pub async fn run_db(&self) {
        // let span_sub_runer = info_span!("run_db 2");
        // let _guard = span_sub_runer.enter();
        let sub_runner_one = SubRunnerOne {};
        sub_runner_one.db().await;
    }
}
struct SubRunnerOne {}

impl SubRunnerOne {
    pub fn run(&self) {
        let span_sub_runer = info_span!("subone_hello_world 3");
        let _guard = span_sub_runer.enter();
        println!("Hello, sub one under world!");
    }

    #[instrument(name = "db3", skip(self))]
    pub async fn db(&self) {
        // let span_sub_runer = info_span!("db 3");
        // let _guard = span_sub_runer.enter();
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

struct Runner {}

impl Runner {
    pub fn run(&self) {
        let span_runer = info_span!("Run Runner 1");
        let _guard = span_runer.enter();
        println!("Hello, world!");
        let sub_runner = SubRunner {};
        sub_runner.run();

        // let sub_runner_one = SubRunnerOne {};
        // sub_runner_one.run();
    }

    #[instrument(name = "db runner1", skip(self))]
    pub async fn run_sub_db(&self) {
        // let span_runer = info_span!("db Runner 1");
        // let _guard = span_runer.enter();
        let sub_runner = SubRunner {};
        sub_runner.run_db().await;

        // let sub_runner_one = SubRunnerOne {};
        // sub_runner_one.run();
    }
}

#[tokio::main]
async fn main() {
    // env_logger::init();
    dotenv::dotenv().ok();

    // let no_op = NoopTracerProvider::new();

    // let tracer = opentelemetry_jaeger::new_agent_pipeline()
    //     .with_service_name("test_tel")
    //     .install_simple()
    //     .expect("failed to install tracing");

    // let otel_exporter = opentelemetry_otlp::SpanExporter::Tonic { timeout: (), metadata: (), trace_exporter: () }
    let otel_exporter = opentelemetry_otlp::SpanExporter::Http {
        timeout: Duration::from_secs(0),
        headers: None,
        collector_endpoint: "http://localhost:4318/v1/traces".parse().unwrap(),
        trace_exporter: None,
    };

    // 1. builder pattern for tracer provider
    let provider = TracerProvider::builder().with_batch_exporter(otel_exporter, Tokio).build();

    let _tracer = provider.tracer("Simple_telemetry_test");

    // //creating a layer for Otel
    // let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // 2. pipeline pattern for tracer provider - best way to stich tracing with opentelemetry
    // can mention all exporter configs here
    // batching and tracing configs curried
    // let otel_new = opentelemetry_otlp::new_pipeline().tracing().install_batch(Tokio).unwrap();

    // let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_new);

    // let subscriber = Registry::default().with(otel_layer);

    //subscribing to tracing with opentelemetry
    // match tracing_subscriber::registry().with(otel_layer).try_init() {
    //     Ok(_) => {}
    //     Err(e) => {
    //         println!("error while initializing tracing {}", e);
    //     }
    // }

    #[instrument]
    fn worker() {
        // let span = info_span!("worker");
        // let _guard = span.enter();
        println!("Hello, world!");
    }

    let handler = tokio::task::spawn(async {
        println!("this is spawning");
        // let runner = Runner {};
        // runner.run();
        // runner.run_sub_db().await;
        let mut count = 0;
        loop {
            count += 1;
            if count / 2 == 0 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    });
    // let handler_one = tokio::task::spawn(async {
    //     let runner = Runner {};
    //     runner.run().await;
    // });

    futures::future::join_all([handler]).await;

    // let runner = Runner {};
    // runner.run();
    tokio::time::sleep(Duration::from_secs(30)).await;
}
