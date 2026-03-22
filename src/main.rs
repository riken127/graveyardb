use graveyar_db::{
    api::event_store_server::EventStoreServer,
    config,
    grpc::GrpcService,
    pipeline::EventPipeline,
    storage::{
        event_store::EventStore, hybrid::HybridEventStore, rocksdb::event_store::RocksEventStore,
        scylla::session::ScyllaStore,
    },
};
use std::sync::Arc;
use tonic::transport::{Identity, Server, ServerTlsConfig};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::settings::Config::from_env()?;
    init_tracing(&config)?;

    println!("bootstrap OK");
    println!("Loaded config: {:?}", config);

    // 1. Storage Initialization
    let rocks_store = Arc::new(RocksEventStore::new(&config.db_path)?);

    let storage: Arc<dyn EventStore> = if let Some(scylla_uri) = &config.scylla_uri {
        println!("Initializing ScyllaDB at {}...", scylla_uri);
        match ScyllaStore::new(scylla_uri, &config.scylla_keyspace).await {
            Ok(scylla) => {
                println!("ScyllaDB connected. Using Hybrid Storage (Primary: Scylla, Fallback: RocksDB).");
                Arc::new(HybridEventStore::new(Arc::new(scylla), rocks_store))
            }
            Err(e) => {
                eprintln!(
                    "Failed to connect to ScyllaDB: {}. Falling back to RocksDB only.",
                    e
                );
                rocks_store
            }
        }
    } else {
        println!("No SCYLLA_URI configured. Using RocksDB only.");
        rocks_store
    };

    // 2. Pipeline
    let pipeline = Arc::new(EventPipeline::new_with_transport(
        storage,
        config.cluster_nodes.clone(),
        config.node_id,
        config.auth_token.clone(),
        config.schema_validation_hard_fail,
        config.request_timeout,
        config.tls_cert_path.is_some() && config.tls_key_path.is_some(),
    ));

    // 3. Snapshot Store (Local RocksDB)
    let snapshot_db_path = format!("{}_snapshots", config.db_path);
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    let snapshot_db =
        Arc::new(rocksdb::DB::open(&opts, &snapshot_db_path).expect("Failed to open Snapshot DB"));
    let snapshot_store = Arc::new(
        graveyar_db::storage::rocksdb::snapshot_store::RocksSnapshotStore::new(snapshot_db),
    );

    // 4. gRPC Service
    let service = GrpcService::new(pipeline, snapshot_store);
    let addr = format!("0.0.0.0:{}", config.port).parse()?;

    println!("Server listening on {}", addr);

    let mut builder = Server::builder();

    if let (Some(cert_path), Some(key_path)) = (config.tls_cert_path, config.tls_key_path) {
        println!(
            "TLS enabled. Loading cert from {} and key from {}",
            cert_path, key_path
        );
        let cert = tokio::fs::read(cert_path).await?;
        let key = tokio::fs::read(key_path).await?;
        let identity = Identity::from_pem(cert, key);

        builder = builder.tls_config(ServerTlsConfig::new().identity(identity))?;
    } else {
        println!("TLS DISABLED (missing TLS_CERT_PATH or TLS_KEY_PATH).");
    }

    if let Some(token) = config.auth_token.clone() {
        println!("Authentication enabled with Bearer Token.");
        let interceptor = graveyar_db::grpc::auth::AuthInterceptor::new(token);
        builder
            .add_service(EventStoreServer::with_interceptor(service, interceptor))
            .serve(addr)
            .await?;
    } else {
        println!("Authentication DISABLED (no AUTH_TOKEN configured).");
        builder
            .add_service(EventStoreServer::new(service))
            .serve(addr)
            .await?;
    }

    Ok(())
}

fn init_tracing(config: &config::settings::Config) -> Result<(), Box<dyn std::error::Error>> {
    if !config.otel_enabled {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .init();
        return Ok(());
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build();
    match exporter {
        Ok(exporter) => {
            let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
                .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                .build();
            let tracer =
                opentelemetry::trace::TracerProvider::tracer(&tracer_provider, "graveyar_db");
            opentelemetry::global::set_tracer_provider(tracer_provider);
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(telemetry)
                .init();
            Ok(())
        }
        Err(err) => {
            if config.otel_fail_fast {
                return Err(Box::new(err));
            }

            eprintln!(
                "Failed to initialize OpenTelemetry exporter (continuing with local tracing only): {}",
                err
            );
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .init();
            Ok(())
        }
    }
}
