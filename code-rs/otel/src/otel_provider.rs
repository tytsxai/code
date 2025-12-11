use crate::config::OtelExporter;
use crate::config::OtelHttpProtocol;
use crate::config::OtelSettings;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::LogExporter;
use opentelemetry_otlp::Protocol;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithHttpConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLogger;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_semantic_conventions as semconv;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use std::error::Error;
use tonic::metadata::MetadataMap;
use tracing::debug;
use tracing::warn;

const ENV_ATTRIBUTE: &str = "env";

pub struct OtelProvider {
    pub logger: SdkLoggerProvider,
}

impl OtelProvider {
    pub fn shutdown(&self) {
        let _ = self.logger.shutdown();
    }

    /// Expose a tracing layer that bridges tracing records into OTLP logs.
    /// Consumers should keep this provider alive for the lifetime of tracing.
    pub fn layer(&self) -> OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger> {
        OpenTelemetryTracingBridge::new(&self.logger)
    }

    pub fn from(settings: &OtelSettings) -> Result<Option<Self>, Box<dyn Error>> {
        let resource = Resource::builder()
            .with_service_name(settings.service_name.clone())
            .with_attributes(vec![
                KeyValue::new(
                    semconv::attribute::SERVICE_VERSION,
                    settings.service_version.clone(),
                ),
                KeyValue::new(ENV_ATTRIBUTE, settings.environment.clone()),
            ])
            .build();

        let mut builder = SdkLoggerProvider::builder().with_resource(resource);

        match &settings.exporter {
            OtelExporter::None => {
                debug!("No exporter enabled in OTLP settings.");
                return Ok(None);
            }
            OtelExporter::OtlpGrpc { endpoint, headers } => {
                debug!("Using OTLP Grpc exporter: {}", endpoint);

                let mut header_map = HeaderMap::new();
                for (key, value) in headers {
                    match (
                        HeaderName::from_bytes(key.as_bytes()),
                        HeaderValue::from_str(value),
                    ) {
                        (Ok(name), Ok(val)) => {
                            header_map.insert(name, val);
                        }
                        _ => warn!("Invalid OTLP gRPC header dropped: key='{}'", key),
                    }
                }

                let exporter = LogExporter::builder()
                    .with_tonic()
                    .with_endpoint(endpoint)
                    .with_metadata(MetadataMap::from_headers(header_map))
                    .build()?;

                builder = builder.with_batch_exporter(exporter);
            }
            OtelExporter::OtlpHttp {
                endpoint,
                headers,
                protocol,
            } => {
                debug!("Using OTLP Http exporter: {}", endpoint);

                let protocol = match protocol {
                    OtelHttpProtocol::Binary => Protocol::HttpBinary,
                    OtelHttpProtocol::Json => Protocol::HttpJson,
                };

                let exporter = LogExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .with_protocol(protocol)
                    .with_headers(headers.clone())
                    .build()?;

                builder = builder.with_batch_exporter(exporter);
            }
        }

        Ok(Some(Self {
            logger: builder.build(),
        }))
    }
}

impl Drop for OtelProvider {
    fn drop(&mut self) {
        let _ = self.logger.shutdown();
    }
}
