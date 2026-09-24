use std::net::SocketAddr;

use analytics_api::{connect_with_definition_version, router};
use anyhow::Context;
use clap::Parser;
use tracing::info;

#[derive(Debug, Parser)]
#[command(name = "analytics-api", version, about = "Query Page View aggregates")]
struct Cli {
    #[arg(long, env = "ANALYTICS_API_HOST", default_value = "0.0.0.0")]
    host: String,
    #[arg(long, env = "ANALYTICS_API_PORT", default_value_t = 4002)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be configured")?;
    let definitions_path = std::env::var("ANALYTICS_DEFINITIONS_FILE")
        .unwrap_or_else(|_| "config/analytics-definitions.json".to_owned());
    let definitions: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&definitions_path).context("failed to read analytics definitions")?,
    )?;
    let definition_schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read("config/analytics-definitions.schema.json")
            .context("failed to read analytics definitions schema")?,
    )?;
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&definition_schema)
        .map_err(|error| anyhow::anyhow!("invalid analytics definition schema: {error}"))?;
    validator.validate(&definitions).map_err(|errors| {
        anyhow::anyhow!(
            "invalid analytics definitions: {}",
            errors
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    validate_definition_privacy(&definitions)?;
    let definition_version = definitions
        .get("version")
        .and_then(serde_json::Value::as_str)
        .filter(|version| !version.trim().is_empty())
        .context("analytics definitions require a non-empty version")?
        .to_owned();
    let state = connect_with_definition_version(&database_url, definition_version)
        .context("failed to configure PostgreSQL pool")?;
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::new(cli.host.parse()?, cli.port)).await?;
    info!(service = "analytics-api", host = %cli.host, port = cli.port, "analytics API started");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

fn validate_definition_privacy(definitions: &serde_json::Value) -> anyhow::Result<()> {
    const FORBIDDEN: [&str; 21] = [
        "email",
        "emailaddress",
        "useremail",
        "phone",
        "phonenumber",
        "name",
        "firstname",
        "lastname",
        "fullname",
        "address",
        "homeaddress",
        "streetaddress",
        "ip",
        "ipaddress",
        "useragent",
        "cookie",
        "password",
        "passwd",
        "token",
        "userid",
        "useridentifier",
    ];
    let mut site_ids = std::collections::HashSet::new();
    for site in definitions["sites"].as_array().into_iter().flatten() {
        let site_id = site["site_id"].as_str().unwrap_or_default();
        anyhow::ensure!(
            site_ids.insert(site_id),
            "duplicate site_id in analytics definitions: {site_id}"
        );
        for category in ["conversions", "funnels"] {
            let mut definition_ids = std::collections::HashSet::new();
            for definition in site[category].as_array().into_iter().flatten() {
                let definition_id = definition["id"].as_str().unwrap_or_default();
                anyhow::ensure!(
                    definition_ids.insert(definition_id),
                    "duplicate {category} definition id for site {site_id}: {definition_id}"
                );
            }
        }
        for properties in site["conversions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|definition| &definition["properties"])
            .chain(
                site["funnels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|funnel| {
                        funnel["steps"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|step| &step["properties"])
                    }),
            )
        {
            if let Some(object) = properties.as_object() {
                for key in object.keys() {
                    let normalized: String = key
                        .to_ascii_lowercase()
                        .chars()
                        .filter(|character| !matches!(character, '_' | '.' | '-'))
                        .collect();
                    anyhow::ensure!(
                        !FORBIDDEN.contains(&normalized.as_str()),
                        "sensitive definition property key is prohibited: {key}"
                    );
                }
            }
        }
    }
    Ok(())
}
