use std::{env, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub scylla_uri: Option<String>,
    pub scylla_keyspace: String,
    pub request_timeout: Duration,
    pub node_id: u64,
    pub cluster_nodes: Vec<String>,
    pub port: u16,
    pub db_path: String,
    pub auth_token: Option<String>,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub schema_validation_hard_fail: bool,
    pub require_tls: bool,
    pub require_auth: bool,
    pub otel_enabled: bool,
    pub otel_fail_fast: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let scylla_uri = env::var("SCYLLA_URI").ok();

        let scylla_keyspace =
            env::var("SCYLLA_KEYSPACE").map_err(|_| "SCYLLA_KEYSPACE is undefined")?;

        let request_timeout = env::var("REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(3));

        let node_id = env::var("NODE_ID")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let cluster_nodes = env::var("CLUSTER_NODES")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec!["127.0.0.1:50051".to_string()]); // Default single node

        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(50051);

        // Allow configurable DB path for multi-node local run
        let db_path = env::var("DB_PATH").unwrap_or_else(|_| "data/rocksdb".to_string());

        let auth_token = env::var("AUTH_TOKEN").ok();
        let tls_cert_path = env::var("TLS_CERT_PATH").ok();
        let tls_key_path = env::var("TLS_KEY_PATH").ok();
        let schema_validation_hard_fail = parse_env_bool("SCHEMA_VALIDATION_HARD_FAIL", false)?;
        let require_tls = parse_env_bool("REQUIRE_TLS", false)?;
        let require_auth = parse_env_bool("REQUIRE_AUTH", false)?;
        let otel_enabled = parse_env_bool("OTEL_ENABLED", false)?;
        let otel_fail_fast = parse_env_bool("OTEL_FAIL_FAST", false)?;

        if require_tls && (tls_cert_path.is_none() || tls_key_path.is_none()) {
            return Err(
                "REQUIRE_TLS=true but TLS_CERT_PATH and TLS_KEY_PATH are not both configured"
                    .to_string(),
            );
        }

        if require_auth && auth_token.is_none() {
            return Err("REQUIRE_AUTH=true but AUTH_TOKEN is not configured".to_string());
        }

        Ok(Self {
            scylla_uri,
            scylla_keyspace,
            request_timeout,
            node_id,
            cluster_nodes,
            port,
            db_path,
            auth_token,
            tls_cert_path,
            tls_key_path,
            schema_validation_hard_fail,
            require_tls,
            require_auth,
            otel_enabled,
            otel_fail_fast,
        })
    }
}

fn parse_env_bool(key: &str, default: bool) -> Result<bool, String> {
    match env::var(key) {
        Ok(value) => value
            .parse::<bool>()
            .map_err(|_| format!("{} must be true or false", key)),
        Err(_) => Ok(default),
    }
}
