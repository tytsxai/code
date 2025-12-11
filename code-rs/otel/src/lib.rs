pub mod config;

pub mod otel_event_manager;
#[cfg(feature = "otel")]
pub mod otel_provider;

#[cfg(not(feature = "otel"))]
mod imp {
    use std::error::Error;
    use tracing_subscriber::layer::Identity;

    pub struct OtelProvider;

    impl OtelProvider {
        pub fn from(
            _settings: &crate::config::OtelSettings,
        ) -> Result<Option<Self>, Box<dyn Error>> {
            Ok(None)
        }

        pub fn layer(&self) -> Identity {
            Identity::default()
        }

        pub fn shutdown(&self) {
            // no-op when OTEL is disabled
        }
    }
}

#[cfg(not(feature = "otel"))]
pub use imp::OtelProvider;
