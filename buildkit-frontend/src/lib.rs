#![deny(warnings)]
#![deny(clippy::all)]

use buildkit_proto::pb;
use failure::{Error, ResultExt};
use log::*;
use serde::de::DeserializeOwned;
use tonic::transport::Endpoint;
use tower::service_fn;

mod bridge;
mod error;
mod stdio;
mod utils;

pub mod oci;
pub mod options;

use oci::ImageSpecification;

pub use self::bridge::Bridge;
pub use self::error::ErrorCode;
pub use self::options::Options;
pub use self::stdio::{stdio_connector, StdioSocket};
pub use self::utils::{ErrorWithCauses, OutputRef};

#[tonic::async_trait]
pub trait Frontend<O = Options>
where
    O: DeserializeOwned,
{
    async fn run(self, bridge: Bridge, options: O) -> Result<FrontendOutput, Error>;
}

/// Result returned by [`Frontend::run`]. Either a single output ref (the
/// classic single-platform case) or a list of per-platform refs that will
/// be assembled into an image index by BuildKit's image exporter.
pub struct FrontendOutput {
    pub(crate) inner: FrontendOutputInner,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum FrontendOutputInner {
    Single {
        output: OutputRef,
        image_spec: Option<ImageSpecification>,
    },
    MultiPlatform(Vec<MultiPlatformEntry>),
}

/// One platform-keyed entry in a multi-platform [`FrontendOutput`]. The
/// `id` is used both as the key in BuildKit's `RefMap` and as the suffix
/// of the `containerimage.config/<id>` metadata key; it defaults to the
/// canonical `<os>/<arch>[/<variant>]` form derived from `platform`.
pub struct MultiPlatformEntry {
    pub id: String,
    pub platform: pb::Platform,
    pub output: OutputRef,
    pub image_spec: Option<ImageSpecification>,
}

impl MultiPlatformEntry {
    pub fn new(platform: pb::Platform, output: OutputRef) -> Self {
        let id = buildkit_llb::ops::platform::platform_id(&platform);
        Self {
            id,
            platform,
            output,
            image_spec: None,
        }
    }

    pub fn with_spec(mut self, spec: ImageSpecification) -> Self {
        self.image_spec = Some(spec);
        self
    }

    /// Override the auto-derived id. Use this when integrating with a
    /// caller that expects a non-canonical key (rare).
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }
}

impl FrontendOutput {
    pub fn with_ref(output: OutputRef) -> Self {
        Self {
            inner: FrontendOutputInner::Single {
                output,
                image_spec: None,
            },
        }
    }

    pub fn with_spec_and_ref(spec: ImageSpecification, output: OutputRef) -> Self {
        Self {
            inner: FrontendOutputInner::Single {
                output,
                image_spec: Some(spec),
            },
        }
    }

    /// Build a multi-platform result. Each entry contributes a per-platform
    /// ref (under `RefMap`) and, if `image_spec` is set, a per-platform
    /// `containerimage.config/<id>` metadata entry. The bridge also emits
    /// the `refs.platforms` JSON blob that BuildKit's image exporter uses
    /// to assemble the final manifest list / OCI image index.
    pub fn with_multi_platform(entries: Vec<MultiPlatformEntry>) -> Self {
        Self {
            inner: FrontendOutputInner::MultiPlatform(entries),
        }
    }
}

pub async fn run_frontend<F, O>(frontend: F) -> Result<(), Error>
where
    F: Frontend<O>,
    O: DeserializeOwned,
{
    let channel = {
        Endpoint::from_static("http://[::]:50051")
            .connect_with_connector(service_fn(stdio_connector))
            .await?
    };

    let bridge = Bridge::new(channel);

    match frontend_entrypoint(&bridge, frontend).await {
        Ok(output) => {
            bridge
                .finish_with_success(output)
                .await
                .context("Unable to send a success result")?;
        }

        Err(error) => {
            let error = ErrorWithCauses::multi_line(error);

            error!("Frontend entrypoint failed: {}", error);

            // https://godoc.org/google.golang.org/grpc/codes#Code
            bridge
                .finish_with_error(
                    ErrorCode::Unknown,
                    ErrorWithCauses::single_line(error.into_inner()).to_string(),
                )
                .await
                .context("Unable to send an error result")?;
        }
    }

    // The HTTP/2 connection over stdio keeps tonic background tasks alive,
    // preventing the tokio runtime from shutting down. Force-exit now that
    // the result has been sent back to the daemon.
    std::process::exit(0);
}

async fn frontend_entrypoint<F, O>(bridge: &Bridge, frontend: F) -> Result<FrontendOutput, Error>
where
    F: Frontend<O>,
    O: DeserializeOwned,
{
    let options = options::from_env(std::env::vars()).context("Unable to parse options")?;

    debug!("running a frontend entrypoint");
    frontend.run(bridge.clone(), options).await
}
