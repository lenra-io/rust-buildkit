use std::iter::once;

use async_trait::async_trait;
use failure::Error;

use buildkit_frontend::oci::*;
use buildkit_frontend::run_frontend;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput, Options, OutputRef};

use buildkit_llb::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    if run_frontend(ReverseFrontend).await.is_err() {
        std::process::exit(1);
    }
}

struct ReverseFrontend;

#[async_trait]
impl Frontend for ReverseFrontend {
    async fn run(self, bridge: Bridge, options: Options) -> Result<FrontendOutput, Error> {
        Ok(FrontendOutput::with_spec_and_ref(
            Self::image_spec(),
            Self::solve(&bridge, options.get("filename").unwrap()).await?,
        ))
    }
}

const OUTPUT_FILENAME: &str = "/reverse.dockerfile";

impl ReverseFrontend {
    fn image_spec() -> ImageSpecification {
        ImageSpecification {
            created: None,
            author: None,

            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,
            os_version: None,
            os_features: None,
            variant: None,

            config: Some(ImageConfig {
                cmd: Some(vec!["/bin/cat".into(), OUTPUT_FILENAME.into()]),
                ..Default::default()
            }),

            rootfs: None,
            history: None,
        }
    }

    async fn solve(bridge: &Bridge, dockerfile_path: &str) -> Result<OutputRef, Error> {
        let dockerfile_source = Source::local("dockerfile");
        let dockerfile_layer = bridge
            .solve(Terminal::with(dockerfile_source.output()))
            .await?;

        let dockerfile_contents = bridge
            .read_file(&dockerfile_layer, dockerfile_path, None)
            .await?;

        let transformed_contents: String = {
            String::from_utf8_lossy(&dockerfile_contents)
                .lines()
                .map(|line| {
                    line.trim()
                        .chars()
                        .rev()
                        .chain(once('\n'))
                        .collect::<String>()
                })
                .collect()
        };

        let llb = {
            let alpine = Source::image("alpine:latest").ref_counted();
            let destination = LayerPath::Other(alpine.output(), OUTPUT_FILENAME);

            FileSystem::mkfile(OutputIdx(0), destination)
                .data(transformed_contents.into_bytes())
                .into_operation()
                .ref_counted()
                .output(0)
        };

        bridge.solve(Terminal::with(llb)).await
    }
}
