use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use failure::{bail, format_err, Error, ResultExt};
use log::*;
use serde::Serialize;
use tokio::sync::Mutex;

use tonic::transport::channel::Channel;
use tonic::Request;

use buildkit_proto::google::rpc::Status;
use buildkit_proto::moby::buildkit::v1::frontend::llb_bridge_client::LlbBridgeClient;
use buildkit_proto::moby::buildkit::v1::frontend::{
    result::Result as RefResult, ReadFileRequest, Ref, RefMap, ResolveImageConfigRequest,
    Result as Output, ReturnRequest, SolveRequest,
};
use buildkit_proto::pb;

pub use buildkit_llb::ops::source::ImageSource;
pub use buildkit_llb::ops::Terminal;
pub use buildkit_proto::moby::buildkit::v1::frontend::FileRange;

use crate::error::ErrorCode;
use crate::oci::ImageSpecification;
use crate::options::common::CacheOptionsEntry;
use crate::utils::OutputRef;
use crate::{FrontendOutput, FrontendOutputInner, MultiPlatformEntry};

/// Metadata key under which the OCI image config (`ImageSpecification`) is
/// returned to BuildKit. For multi-platform results the per-platform
/// id (e.g. `linux/amd64`) is appended after a `/`.
const CONFIG_KEY: &str = "containerimage.config";

/// Metadata key under which BuildKit expects the JSON-encoded list of
/// platforms produced by a multi-platform frontend, in the format described
/// by [`buildkit/exporter/exptypes.Platforms`](https://github.com/moby/buildkit/blob/master/exporter/exptypes/types.go).
const PLATFORMS_KEY: &str = "refs.platforms";

#[derive(Clone)]
pub struct Bridge {
    client: Arc<Mutex<LlbBridgeClient<Channel>>>,
}

impl Bridge {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            client: Arc::new(Mutex::new(LlbBridgeClient::new(channel))),
        }
    }

    pub async fn resolve_image_config(
        &self,
        image: &ImageSource,
        log: Option<&str>,
    ) -> Result<(String, ImageSpecification), Error> {
        let request = ResolveImageConfigRequest {
            r#ref: image.canonical_name(),
            platform: image.platform().cloned(),
            resolve_mode: image.resolve_mode().unwrap_or_default().to_string(),
            log_name: log.unwrap_or_default().into(),

            ..Default::default()
        };

        debug!("requesting to resolve an image: {:?}", request);
        let response = {
            self.client
                .lock()
                .await
                .resolve_image_config(Request::new(request))
                .await
                .unwrap()
                .into_inner()
        };

        Ok((
            response.digest,
            serde_json::from_slice(&response.config)
                .context("Unable to parse image specification")?,
        ))
    }

    pub async fn solve<'a, 'b: 'a>(&'a self, graph: Terminal<'b>) -> Result<OutputRef, Error> {
        self.solve_with_cache(graph, &[]).await
    }

    pub async fn solve_with_cache<'a, 'b: 'a>(
        &'a self,
        graph: Terminal<'b>,
        cache: &[CacheOptionsEntry],
    ) -> Result<OutputRef, Error> {
        let inner = self.send_solve(graph, cache).await?;

        match inner {
            RefResult::Ref(Ref { id, .. }) => Ok(OutputRef(id)),
            other => bail!("Unexpected solve response: {:?}", other),
        }
    }

    /// Solve a graph that produces multiple per-platform refs and return
    /// them keyed by the canonical platform string (`linux/amd64`,
    /// `linux/arm/v7`, ...). Use this when the LLB graph itself is
    /// already multi-platform aware (e.g. produced by a delegating
    /// frontend); to assemble per-platform refs that the current frontend
    /// solved separately use [`FrontendOutput::with_multi_platform`].
    pub async fn solve_multi_platform<'a, 'b: 'a>(
        &'a self,
        graph: Terminal<'b>,
    ) -> Result<HashMap<String, OutputRef>, Error> {
        self.solve_multi_platform_with_cache(graph, &[]).await
    }

    pub async fn solve_multi_platform_with_cache<'a, 'b: 'a>(
        &'a self,
        graph: Terminal<'b>,
        cache: &[CacheOptionsEntry],
    ) -> Result<HashMap<String, OutputRef>, Error> {
        let inner = self.send_solve(graph, cache).await?;

        match inner {
            RefResult::Refs(RefMap { refs }) => Ok(refs
                .into_iter()
                .map(|(id, Ref { id: ref_id, .. })| (id, OutputRef(ref_id)))
                .collect()),
            RefResult::Ref(Ref { id, .. }) => {
                let mut map = HashMap::new();
                map.insert(String::new(), OutputRef(id));
                Ok(map)
            }
            other => bail!("Unexpected solve response: {:?}", other),
        }
    }

    async fn send_solve<'a, 'b: 'a>(
        &'a self,
        graph: Terminal<'b>,
        cache: &[CacheOptionsEntry],
    ) -> Result<RefResult, Error> {
        debug!("serializing a graph to request");
        let request = SolveRequest {
            definition: Some(graph.into_definition()),
            exporter_attr: vec![],
            allow_result_return: true,
            cache_imports: cache.iter().cloned().map(Into::into).collect(),

            ..Default::default()
        };

        debug!("solving with cache from: {:?}", cache);
        debug!("requesting to solve a graph");
        let response = {
            self.client
                .lock()
                .await
                .solve(Request::new(request))
                .await
                .context("Unable to solve the graph")?
                .into_inner()
                .result
                .ok_or_else(|| format_err!("Unable to extract solve result"))?
        };

        debug!("got response: {:#?}", response);

        response
            .result
            .ok_or_else(|| format_err!("Unable to extract solve result"))
    }

    pub async fn read_file<'a, 'b: 'a, P>(
        &'a self,
        layer: &'b OutputRef,
        path: P,
        range: Option<FileRange>,
    ) -> Result<Vec<u8>, Error>
    where
        P: Into<PathBuf>,
    {
        let file_path = path.into().display().to_string();
        debug!("requesting a file contents: {:#?}", file_path);

        let request = ReadFileRequest {
            r#ref: layer.0.clone(),
            file_path,
            range,

            ..Default::default()
        };

        let response = {
            self.client
                .lock()
                .await
                .read_file(Request::new(request))
                .await
                .context("Unable to read the file")?
                .into_inner()
                .data
        };

        Ok(response)
    }

    pub(crate) async fn finish_with_success(self, output: FrontendOutput) -> Result<(), Error> {
        let request = match output.inner {
            FrontendOutputInner::Single { output, image_spec } => {
                let mut metadata = HashMap::new();
                if let Some(config) = image_spec {
                    metadata.insert(CONFIG_KEY.into(), serde_json::to_vec(&config)?);
                }
                ReturnRequest {
                    error: None,
                    result: Some(Output {
                        result: Some(RefResult::Ref(Ref {
                            id: output.0,
                            def: None,
                        })),
                        metadata,

                        ..Default::default()
                    }),
                }
            }

            FrontendOutputInner::MultiPlatform(entries) => {
                build_multi_platform_return(&entries)?
            }
        };

        self.client
            .lock()
            .await
            .r#return(Request::new(request))
            .await?;

        // TODO: gracefully shutdown the HTTP/2 connection

        Ok(())
    }

    pub(crate) async fn finish_with_error<S>(self, code: ErrorCode, message: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let request = ReturnRequest {
            result: None,
            error: Some(Status {
                code: code as i32,
                message: message.into(),
                details: vec![],
            }),
        };

        debug!("sending an error result: {:#?}", request);
        self.client
            .lock()
            .await
            .r#return(Request::new(request))
            .await?;

        // TODO: gracefully shutdown the HTTP/2 connection

        Ok(())
    }
}

/// Build a [`ReturnRequest`] that ships per-platform refs along with the
/// `refs.platforms` metadata blob and per-platform
/// `containerimage.config/<id>` entries.
fn build_multi_platform_return(entries: &[MultiPlatformEntry]) -> Result<ReturnRequest, Error> {
    if entries.is_empty() {
        bail!("multi-platform output must contain at least one entry");
    }

    let mut refs = HashMap::with_capacity(entries.len());
    let mut metadata = HashMap::with_capacity(entries.len() + 1);

    for entry in entries {
        if refs.contains_key(&entry.id) {
            bail!("duplicate platform id in multi-platform output: {}", entry.id);
        }
        refs.insert(
            entry.id.clone(),
            Ref {
                id: entry.output.0.clone(),
                def: None,
            },
        );
        if let Some(config) = entry.image_spec.as_ref() {
            metadata.insert(
                format!("{}/{}", CONFIG_KEY, entry.id),
                serde_json::to_vec(config)?,
            );
        }
    }

    let platforms_payload = PlatformsJson {
        platforms: entries
            .iter()
            .map(|entry| PlatformEntryJson {
                id: &entry.id,
                platform: PlatformJson::from(&entry.platform),
            })
            .collect(),
    };
    metadata.insert(
        PLATFORMS_KEY.into(),
        serde_json::to_vec(&platforms_payload)?,
    );

    Ok(ReturnRequest {
        error: None,
        result: Some(Output {
            result: Some(RefResult::Refs(RefMap { refs })),
            metadata,

            ..Default::default()
        }),
    })
}

#[derive(Serialize)]
struct PlatformsJson<'a> {
    platforms: Vec<PlatformEntryJson<'a>>,
}

#[derive(Serialize)]
struct PlatformEntryJson<'a> {
    #[serde(rename = "ID")]
    id: &'a str,
    platform: PlatformJson<'a>,
}

#[derive(Serialize)]
struct PlatformJson<'a> {
    architecture: &'a str,
    os: &'a str,

    #[serde(skip_serializing_if = "str::is_empty")]
    variant: &'a str,

    #[serde(rename = "os.version", skip_serializing_if = "str::is_empty")]
    os_version: &'a str,

    #[serde(
        rename = "os.features",
        skip_serializing_if = "<[String]>::is_empty"
    )]
    os_features: &'a [String],
}

impl<'a> From<&'a pb::Platform> for PlatformJson<'a> {
    fn from(p: &'a pb::Platform) -> Self {
        Self {
            architecture: &p.architecture,
            os: &p.os,
            variant: &p.variant,
            os_version: &p.os_version,
            os_features: &p.os_features,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::OutputRef;
    use buildkit_llb::ops::platform;

    #[test]
    fn multi_platform_metadata_shape() {
        let entries = vec![
            MultiPlatformEntry::new(platform::linux_amd64(), OutputRef("amd64-id".into())),
            MultiPlatformEntry::new(platform::linux_arm_v7(), OutputRef("arm-id".into())),
        ];

        let request = build_multi_platform_return(&entries).unwrap();
        let result = request.result.unwrap();

        match result.result.unwrap() {
            RefResult::Refs(RefMap { refs }) => {
                assert_eq!(refs.len(), 2);
                assert_eq!(refs.get("linux/amd64").unwrap().id, "amd64-id");
                assert_eq!(refs.get("linux/arm/v7").unwrap().id, "arm-id");
            }
            other => panic!("expected RefResult::Refs, got {:?}", other),
        }

        let platforms_blob = result.metadata.get(PLATFORMS_KEY).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(platforms_blob).unwrap();
        let arr = parsed["platforms"].as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let amd = arr.iter().find(|e| e["ID"] == "linux/amd64").unwrap();
        assert_eq!(amd["platform"]["architecture"], "amd64");
        assert_eq!(amd["platform"]["os"], "linux");
        assert!(amd["platform"].get("variant").is_none());

        let arm = arr.iter().find(|e| e["ID"] == "linux/arm/v7").unwrap();
        assert_eq!(arm["platform"]["architecture"], "arm");
        assert_eq!(arm["platform"]["os"], "linux");
        assert_eq!(arm["platform"]["variant"], "v7");

        // No image_spec was attached, so no per-platform config keys.
        assert!(result
            .metadata
            .keys()
            .all(|k| !k.starts_with(&format!("{}/", CONFIG_KEY))));
    }

    #[test]
    fn multi_platform_rejects_duplicate_id() {
        let entries = vec![
            MultiPlatformEntry::new(platform::linux_amd64(), OutputRef("a".into())),
            MultiPlatformEntry::new(platform::linux_amd64(), OutputRef("b".into())),
        ];

        let err = build_multi_platform_return(&entries).unwrap_err();
        assert!(err.to_string().contains("duplicate platform id"));
    }

    #[test]
    fn multi_platform_rejects_empty() {
        let err = build_multi_platform_return(&[]).unwrap_err();
        assert!(err.to_string().contains("at least one entry"));
    }
}
