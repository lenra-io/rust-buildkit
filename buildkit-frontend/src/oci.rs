use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::time::Duration;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// (De)serializer for `Option<Duration>` fields that follow Go's
/// `time.Duration` JSON convention (integer count of nanoseconds). Used
/// by the Docker [`Healthcheck`] extension where buildkitd, dockerd and
/// containerd all read/write durations as nanos.
mod opt_duration_nanos {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::convert::TryFrom;
    use std::time::Duration;

    pub fn serialize<S>(d: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match d {
            // `skip_serializing_if = "Option::is_none"` filters None
            // before this is reached, but handle it defensively.
            Some(d) => s.serialize_u64(u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n: Option<u64> = Option::deserialize(d)?;
        Ok(n.map(Duration::from_nanos))
    }
}

// https://github.com/opencontainers/image-spec/blob/v1.0.1/config.md

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSpecification {
    /// An combined date and time at which the image was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// Gives the name and/or email address of the person or entity which created and is responsible for maintaining the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The CPU architecture which the binaries in this image are built to run on.
    pub architecture: Architecture,

    /// The name of the operating system which the image is built to run on.
    pub os: OperatingSystem,

    /// Optional version of the operating system, used to differentiate
    /// between Windows builds (`10.0.17763.1234`, ...) or specific kernel
    /// constraints. Serialized under the dotted JSON key `os.version`
    /// per the OCI image-spec.
    #[serde(rename = "os.version", skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,

    /// Optional list of OS features required by the image (Windows uses
    /// `win32k` for example). Serialized under the dotted JSON key
    /// `os.features` per the OCI image-spec.
    #[serde(rename = "os.features", skip_serializing_if = "Option::is_none")]
    pub os_features: Option<Vec<String>>,

    /// CPU sub-architecture / variant (`v7`, `v8`, ...). Useful to
    /// differentiate `linux/arm/v6` from `linux/arm/v7` configs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,

    /// The execution parameters which should be used as a base when running a container using the image.
    /// This field can be `None`, in which case any execution parameters should be specified at creation of the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ImageConfig>,

    /// The rootfs key references the layer content addresses used by the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<ImageRootfs>,

    /// Describes the history of each layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<LayerHistoryItem>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    /// 64-bit x86, the most mature port
    Amd64,

    /// 32-bit x86
    I386,

    /// 32-bit ARM
    ARM,

    /// 64-bit ARM
    ARM64,

    /// PowerPC 64-bit, little-endian
    PPC64le,

    /// PowerPC 64-bit, big-endian
    PPC64,

    /// MIPS 64-bit, little-endian
    Mips64le,

    /// MIPS 64-bit, big-endian
    Mips64,

    /// MIPS 32-bit, little-endian
    Mipsle,

    /// MIPS 32-bit, big-endian
    Mips,

    /// IBM System z 64-bit, big-endian
    S390x,

    /// 64-bit RISC-V
    Riscv64,

    /// 64-bit LoongArch
    Loong64,

    /// WebAssembly
    Wasm,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingSystem {
    Aix,
    Android,
    Darwin,
    Dragonfly,
    Freebsd,
    Hurd,
    Illumos,
    Ios,
    Js,
    Linux,
    Netbsd,
    Openbsd,
    Plan9,
    Solaris,
    Windows,
    Zos,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawImageConfig")]
#[serde(into = "RawImageConfig")]
pub struct ImageConfig {
    /// The username or UID which is a platform-specific structure that allows specific control over which user the process run as.
    pub user: Option<String>,

    /// A set of ports to expose from a container running this image.
    pub exposed_ports: Option<Vec<ExposedPort>>,

    /// Environment variables for the process to run with.
    pub env: Option<BTreeMap<String, String>>,

    /// A list of arguments to use as the command to execute when the container starts.
    pub entrypoint: Option<Vec<String>>,

    /// Default arguments to the entrypoint of the container.
    pub cmd: Option<Vec<String>>,

    /// A set of directories describing where the process is likely write data specific to a container instance.
    pub volumes: Option<Vec<PathBuf>>,

    /// Sets the current working directory of the entrypoint process in the container.
    pub working_dir: Option<PathBuf>,

    /// The field contains arbitrary metadata for the container.
    pub labels: Option<BTreeMap<String, String>>,

    /// The field contains the system call signal that will be sent to the container to exit.
    pub stop_signal: Option<Signal>,

    /// Docker-extension healthcheck definition (`HEALTHCHECK` directive in
    /// a Dockerfile). Read by `docker run`, `podman run`, BuildKit and
    /// the OCI runtime; absent for OCI-only images that don't opt in.
    pub healthcheck: Option<Healthcheck>,

    /// Docker-extension default shell used by the shell-form of `RUN`,
    /// `CMD` and `ENTRYPOINT`. Defaults to `["/bin/sh", "-c"]` on Linux
    /// and `["cmd", "/S", "/C"]` on Windows when omitted.
    pub shell: Option<Vec<String>>,

    /// Windows-only Docker-extension flag indicating that command
    /// arguments are already escaped for `cmd.exe` and should not be
    /// re-escaped. Deprecated by Docker but still emitted by older
    /// pipelines, so kept here for round-trip fidelity.
    pub args_escaped: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawImageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    exposed_ports: Option<BTreeMap<ExposedPort, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    entrypoint: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    cmd: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    volumes: Option<BTreeMap<PathBuf, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    working_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<BTreeMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stop_signal: Option<Signal>,

    #[serde(skip_serializing_if = "Option::is_none")]
    healthcheck: Option<Healthcheck>,

    #[serde(skip_serializing_if = "Option::is_none")]
    shell: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    args_escaped: Option<bool>,
}

/// Docker `HEALTHCHECK` payload as carried inside the OCI image config.
/// All durations follow Go's `time.Duration` JSON convention - an
/// integer count of nanoseconds - which is what dockerd, BuildKit and
/// containerd all read and write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Healthcheck {
    /// The probe to run. Conventional shapes:
    /// - `["NONE"]` to disable an inherited healthcheck;
    /// - `["CMD", "<arg>", ...]` to exec the args directly;
    /// - `["CMD-SHELL", "<command line>"]` to run inside the image's
    ///   default shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<Vec<String>>,

    /// Time between the end of one check and the start of the next.
    #[serde(
        with = "opt_duration_nanos",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub interval: Option<Duration>,

    /// Maximum time a single probe can take before being considered to
    /// have failed.
    #[serde(
        with = "opt_duration_nanos",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub timeout: Option<Duration>,

    /// Initial grace period during which probe failures don't count
    /// against `retries`. Useful for slow-starting services.
    #[serde(
        with = "opt_duration_nanos",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub start_period: Option<Duration>,

    /// During the start period, run probes this often (defaults to
    /// `interval` if unset). Added in newer Docker / containerd.
    #[serde(
        with = "opt_duration_nanos",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub start_interval: Option<Duration>,

    /// Number of consecutive probe failures required to mark the
    /// container unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageRootfs {
    /// Must be set to `RootfsType::Layers`.
    #[serde(rename = "type")]
    pub diff_type: RootfsType,

    /// An array of layer content hashes (DiffIDs), in order from first to last.
    pub diff_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerHistoryItem {
    /// A combined date and time at which the layer was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// The author of the build point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The command which created the layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// A custom message set when creating the layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// This field is used to mark if the history item created a filesystem diff.
    /// It is set to true if this history item doesn't correspond to an actual layer in the rootfs section
    /// (for example, Dockerfile's ENV command results in no change to the filesystem).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_layer: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub enum ExposedPort {
    Tcp(u16),
    Udp(u16),
}

impl TryFrom<String> for ExposedPort {
    type Error = std::num::ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let postfix_len = value.len() - 4;

        match &value[postfix_len..] {
            "/tcp" => Ok(ExposedPort::Tcp(value[..postfix_len].parse()?)),
            "/udp" => Ok(ExposedPort::Udp(value[..postfix_len].parse()?)),

            _ => Ok(ExposedPort::Tcp(value.parse()?)),
        }
    }
}

impl From<ExposedPort> for String {
    fn from(val: ExposedPort) -> Self {
        match val {
            ExposedPort::Tcp(port) => format!("{}/tcp", port),
            ExposedPort::Udp(port) => format!("{}/udp", port),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RootfsType {
    Layers,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Signal {
    SIGHUP,
    SIGINT,
    SIGQUIT,
    SIGILL,
    SIGTRAP,
    SIGABRT,
    SIGBUS,
    SIGFPE,
    SIGKILL,
    SIGUSR1,
    SIGSEGV,
    SIGUSR2,
    SIGPIPE,
    SIGALRM,
    SIGTERM,
    SIGSTKFLT,
    SIGCHLD,
    SIGCONT,
    SIGSTOP,
    SIGTSTP,
    SIGTTIN,
    SIGTTOU,
    SIGURG,
    SIGXCPU,
    SIGXFSZ,
    SIGVTALRM,
    SIGPROF,
    SIGWINCH,
    SIGIO,
    SIGPWR,
    SIGSYS,
    SIGEMT,
    SIGINFO,
}

impl From<RawImageConfig> for ImageConfig {
    fn from(raw: RawImageConfig) -> Self {
        Self {
            user: raw.user,
            entrypoint: raw.entrypoint,
            cmd: raw.cmd,
            working_dir: raw.working_dir,
            labels: raw.labels,
            stop_signal: raw.stop_signal,
            healthcheck: raw.healthcheck,
            shell: raw.shell,
            args_escaped: raw.args_escaped,

            env: raw.env.map(|inner| {
                inner
                    .into_iter()
                    .map(|mut pair| match pair.find('=') {
                        Some(pos) => {
                            let value = pair.split_off(pos + 1);
                            let mut name = pair;
                            name.pop();

                            (name, value)
                        }

                        None => (pair, String::with_capacity(0)),
                    })
                    .collect()
            }),

            exposed_ports: raw
                .exposed_ports
                .map(|inner| inner.into_keys().collect()),

            volumes: raw
                .volumes
                .map(|inner| inner.into_keys().collect()),
        }
    }
}

impl From<ImageConfig> for RawImageConfig {
    fn from(val: ImageConfig) -> Self {
        RawImageConfig {
            user: val.user,
            entrypoint: val.entrypoint,
            cmd: val.cmd,
            working_dir: val.working_dir,
            labels: val.labels,
            stop_signal: val.stop_signal,
            healthcheck: val.healthcheck,
            shell: val.shell,
            args_escaped: val.args_escaped,

            env: val.env.map(|inner| {
                inner
                    .into_iter()
                    .map(|(key, value)| format!("{}={}", key, value))
                    .collect()
            }),

            exposed_ports: val.exposed_ports.map(|inner| {
                inner
                    .into_iter()
                    .map(|port| (port, Value::Object(Default::default())))
                    .collect()
            }),

            volumes: val.volumes.map(|inner| {
                inner
                    .into_iter()
                    .map(|volume| (volume, Value::Object(Default::default())))
                    .collect()
            }),
        }
    }
}

#[test]
fn serialization() {
    use pretty_assertions::assert_eq;

    let ref_json = include_str!("../tests/oci-image-spec.json");
    let ref_spec = ImageSpecification {
        created: Some("2015-10-31T22:22:56.015925234Z".parse().unwrap()),
        author: Some("Alyssa P. Hacker <alyspdev@example.com>".into()),
        architecture: Architecture::Amd64,
        os: OperatingSystem::Linux,
        os_version: None,
        os_features: None,
        variant: None,
        rootfs: Some(ImageRootfs {
            diff_type: RootfsType::Layers,
            diff_ids: vec![
                "sha256:c6f988f4874bb0add23a778f753c65efe992244e148a1d2ec2a8b664fb66bbd1".into(),
                "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef".into(),
            ],
        }),
        history: Some(vec![
            LayerHistoryItem {
                created: Some("2015-10-31T22:22:54.690851953Z".parse().unwrap()),
                created_by: Some("/bin/sh -c #(nop) ADD file in /".into()),
                author: None,
                comment: None,
                empty_layer: None,
            },
            LayerHistoryItem {
                created: Some("2015-10-31T22:22:55.613815829Z".parse().unwrap()),
                created_by: Some("/bin/sh -c #(nop) CMD [\"sh\"]".into()),
                author: None,
                comment: None,
                empty_layer: Some(true),
            },
        ]),

        config: Some(ImageConfig {
            user: Some("alice".into()),
            exposed_ports: Some(vec![ExposedPort::Tcp(8080), ExposedPort::Udp(8081)]),
            env: Some(
                vec![(
                    String::from("PATH"),
                    String::from("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"),
                )]
                .into_iter()
                .collect(),
            ),
            entrypoint: Some(vec!["/bin/my-app-binary".into()]),
            cmd: Some(vec![
                "--foreground".into(),
                "--config".into(),
                "/etc/my-app.d/default.cfg".into(),
            ]),
            volumes: Some(vec![
                "/var/job-result-data".into(),
                "/var/log/my-app-logs".into(),
            ]),
            working_dir: Some("/home/alice".into()),
            labels: Some(
                vec![(
                    String::from("com.example.project.git.url"),
                    String::from("https://example.com/project.git"),
                )]
                .into_iter()
                .collect(),
            ),
            stop_signal: Some(Signal::SIGKILL),
            healthcheck: None,
            shell: None,
            args_escaped: None,
        }),
    };

    assert_eq!(serde_json::to_string_pretty(&ref_spec).unwrap(), ref_json);
    assert_eq!(
        serde_json::from_str::<ImageSpecification>(ref_json).unwrap(),
        ref_spec
    );
}

#[test]
fn min_serialization() {
    use pretty_assertions::assert_eq;

    let ref_json = include_str!("../tests/oci-image-spec-min.json");
    let ref_spec = ImageSpecification {
        created: None,
        author: None,

        architecture: Architecture::Amd64,
        os: OperatingSystem::Linux,
        os_version: None,
        os_features: None,
        variant: None,
        rootfs: Some(ImageRootfs {
            diff_type: RootfsType::Layers,
            diff_ids: vec![
                "sha256:c6f988f4874bb0add23a778f753c65efe992244e148a1d2ec2a8b664fb66bbd1".into(),
                "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef".into(),
            ],
        }),

        history: None,
        config: None,
    };

    assert_eq!(serde_json::to_string_pretty(&ref_spec).unwrap(), ref_json);
    assert_eq!(
        serde_json::from_str::<ImageSpecification>(ref_json).unwrap(),
        ref_spec
    );
}

// https://github.com/opencontainers/image-spec/blob/v1.0.1/image-index.md
// https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md

/// OCI Image Index - the JSON structure used to describe a multi-platform
/// (a.k.a. manifest list) image. BuildKit's image exporter assembles one
/// of these from the per-platform refs returned by a frontend, but it can
/// also be useful to consume an existing index from disk or to attach one
/// to a return result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIndex {
    pub schema_version: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// OCI v1.1 - declares this index as an artifact rather than a
    /// regular image, e.g. `application/vnd.in-toto+json` for an
    /// attestation index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,

    pub manifests: Vec<Descriptor>,

    /// OCI v1.1 - reference to another descriptor this index is "about",
    /// used by the referrers API to attach attestations / SBOMs to an
    /// existing image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<BTreeMap<String, String>>,
}

/// OCI image manifest (per-platform - what an [`ImageIndex`] entry
/// resolves to). Carries the descriptors of the image config blob and
/// the layer blobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageManifest {
    pub schema_version: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// OCI v1.1 - declares the manifest as an artifact rather than a
    /// regular image (used for SBOMs, attestations, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,

    pub config: Descriptor,
    pub layers: Vec<Descriptor>,

    /// OCI v1.1 - reference to another descriptor this manifest is
    /// "about" (referrers API).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<BTreeMap<String, String>>,
}

/// OCI content descriptor pointing at a per-platform manifest inside an
/// [`ImageIndex`], or at the config / layer blobs inside an
/// [`ImageManifest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub media_type: String,
    pub digest: String,
    pub size: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<BTreeMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,

    /// OCI v1.1 - inline blob payload, base64-encoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    /// OCI v1.1 - declares the referenced blob as an artifact type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
}

/// OCI Platform descriptor (per-manifest target). Mirrors the JSON shape
/// of `ocispecs.Platform` from the image-spec, including the dotted
/// `os.version` / `os.features` keys, so the round-trip with BuildKit's
/// `refs.platforms` metadata stays stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Platform {
    pub architecture: Architecture,
    pub os: OperatingSystem,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,

    #[serde(rename = "os.version", skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,

    #[serde(rename = "os.features", skip_serializing_if = "Option::is_none")]
    pub os_features: Option<Vec<String>>,
}

#[test]
fn image_index_roundtrip() {
    let json = r#"{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.index.v1+json",
  "manifests": [
    {
      "mediaType": "application/vnd.oci.image.manifest.v1+json",
      "digest": "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
      "size": 7143,
      "platform": {
        "architecture": "amd64",
        "os": "linux"
      }
    },
    {
      "mediaType": "application/vnd.oci.image.manifest.v1+json",
      "digest": "sha256:5b0bcabd1ed22e9fb1310cf6c2dec7cdef19f0ad69efa1f392e94a4333501270",
      "size": 7682,
      "platform": {
        "architecture": "arm",
        "os": "linux",
        "variant": "v7"
      }
    }
  ]
}"#;

    let parsed: ImageIndex = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.schema_version, 2);
    assert_eq!(parsed.manifests.len(), 2);
    assert_eq!(
        parsed.manifests[0].platform.as_ref().unwrap().architecture,
        Architecture::Amd64
    );
    assert_eq!(
        parsed.manifests[1].platform.as_ref().unwrap().variant,
        Some("v7".into())
    );

    // Re-serializing produces the same JSON.
    assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
}

#[cfg(test)]
mod modern_fields_tests {
    use super::*;
    use std::time::Duration;

    /// Image config carrying every Docker extension we care about
    /// (healthcheck, shell, args_escaped) on top of the OCI base.
    #[test]
    fn image_config_with_healthcheck_shell_args_escaped() {
        let json = r#"{
  "Cmd": [
    "nginx",
    "-g",
    "daemon off;"
  ],
  "Healthcheck": {
    "Test": [
      "CMD-SHELL",
      "curl -f http://localhost/ || exit 1"
    ],
    "Interval": 30000000000,
    "Timeout": 5000000000,
    "StartPeriod": 60000000000,
    "StartInterval": 1000000000,
    "Retries": 3
  },
  "Shell": [
    "/bin/bash",
    "-c"
  ],
  "ArgsEscaped": true
}"#;

        let parsed: ImageConfig = serde_json::from_str(json).unwrap();
        let hc = parsed.healthcheck.as_ref().unwrap();
        assert_eq!(
            hc.test.as_ref().unwrap(),
            &vec![
                "CMD-SHELL".to_string(),
                "curl -f http://localhost/ || exit 1".to_string(),
            ]
        );
        assert_eq!(hc.interval, Some(Duration::from_secs(30)));
        assert_eq!(hc.timeout, Some(Duration::from_secs(5)));
        assert_eq!(hc.start_period, Some(Duration::from_secs(60)));
        assert_eq!(hc.start_interval, Some(Duration::from_secs(1)));
        assert_eq!(hc.retries, Some(3));
        assert_eq!(parsed.shell.as_deref(), Some(&["/bin/bash".into(), "-c".into()][..]));
        assert_eq!(parsed.args_escaped, Some(true));

        // Round-trip back to the same JSON.
        assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
    }

    /// `ImageSpecification` with the OCI v1.0.2+ top-level optional fields
    /// (`variant`, `os.version`, `os.features`).
    #[test]
    fn image_spec_with_variant_and_os_version() {
        let json = r#"{
  "architecture": "arm",
  "os": "linux",
  "os.version": "5.10",
  "os.features": [
    "vfp",
    "neon"
  ],
  "variant": "v7"
}"#;

        let parsed: ImageSpecification = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.architecture, Architecture::ARM);
        assert_eq!(parsed.variant.as_deref(), Some("v7"));
        assert_eq!(parsed.os_version.as_deref(), Some("5.10"));
        assert_eq!(
            parsed.os_features.as_ref().unwrap(),
            &vec!["vfp".to_string(), "neon".to_string()]
        );

        assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
    }

    /// New `Architecture` enum members survive a round-trip.
    #[test]
    fn arch_riscv_wasm_loong() {
        for (arch, wire) in [
            (Architecture::Riscv64, "riscv64"),
            (Architecture::Wasm, "wasm"),
            (Architecture::Loong64, "loong64"),
        ] {
            let json = serde_json::to_string(&arch).unwrap();
            assert_eq!(json, format!("\"{}\"", wire));
            let back: Architecture = serde_json::from_str(&json).unwrap();
            assert_eq!(back, arch);
        }
    }

    /// New `OperatingSystem` enum members survive a round-trip.
    #[test]
    fn os_modern_variants() {
        for (os, wire) in [
            (OperatingSystem::Aix, "aix"),
            (OperatingSystem::Android, "android"),
            (OperatingSystem::Illumos, "illumos"),
            (OperatingSystem::Ios, "ios"),
            (OperatingSystem::Js, "js"),
            (OperatingSystem::Zos, "zos"),
        ] {
            let json = serde_json::to_string(&os).unwrap();
            assert_eq!(json, format!("\"{}\"", wire));
            let back: OperatingSystem = serde_json::from_str(&json).unwrap();
            assert_eq!(back, os);
        }
    }

    /// `ImageManifest` round-trips a minimal-but-realistic OCI v1.1
    /// manifest with `subject`, `artifactType` and inline blob `data`.
    #[test]
    fn image_manifest_with_oci_v1_1_fields() {
        let json = r#"{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "artifactType": "application/vnd.example.sbom.v1+json",
  "config": {
    "mediaType": "application/vnd.oci.image.config.v1+json",
    "digest": "sha256:b5b2b2c507a0944348e0303114d8d93aaaa081732b86451d9bce1f432a537bc7",
    "size": 1234
  },
  "layers": [
    {
      "mediaType": "application/vnd.oci.image.layer.v1.tar+gzip",
      "digest": "sha256:9834876dcfb05cb167a5c24953eba58c4ac89b1adf57f28f2f9d09af107ee8f0",
      "size": 32654,
      "data": "aGVsbG8="
    }
  ],
  "subject": {
    "mediaType": "application/vnd.oci.image.manifest.v1+json",
    "digest": "sha256:5b0bcabd1ed22e9fb1310cf6c2dec7cdef19f0ad69efa1f392e94a4333501270",
    "size": 7682
  }
}"#;

        let parsed: ImageManifest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.schema_version, 2);
        assert_eq!(
            parsed.artifact_type.as_deref(),
            Some("application/vnd.example.sbom.v1+json")
        );
        assert_eq!(parsed.layers.len(), 1);
        assert_eq!(parsed.layers[0].data.as_deref(), Some("aGVsbG8="));
        assert!(parsed.subject.is_some());

        assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
    }

    /// `ImageIndex` round-trips with `subject` (referrers API) and
    /// `artifactType` (OCI v1.1).
    #[test]
    fn image_index_with_oci_v1_1_fields() {
        let json = r#"{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.index.v1+json",
  "artifactType": "application/vnd.example.attestation+json",
  "manifests": [
    {
      "mediaType": "application/vnd.oci.image.manifest.v1+json",
      "digest": "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
      "size": 7143
    }
  ],
  "subject": {
    "mediaType": "application/vnd.oci.image.manifest.v1+json",
    "digest": "sha256:5b0bcabd1ed22e9fb1310cf6c2dec7cdef19f0ad69efa1f392e94a4333501270",
    "size": 7682
  }
}"#;

        let parsed: ImageIndex = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.artifact_type.as_deref(),
            Some("application/vnd.example.attestation+json")
        );
        assert!(parsed.subject.is_some());

        assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
    }

    /// Healthcheck `Test: ["NONE"]` (disable inherited healthcheck) with
    /// no other fields set is a common Dockerfile case.
    #[test]
    fn healthcheck_none_only_serializes_test() {
        let hc = Healthcheck {
            test: Some(vec!["NONE".into()]),
            interval: None,
            timeout: None,
            start_period: None,
            start_interval: None,
            retries: None,
        };
        assert_eq!(
            serde_json::to_string(&hc).unwrap(),
            r#"{"Test":["NONE"]}"#
        );
    }

    /// `ImageConfig::default()` is empty - useful so callers can populate
    /// only the fields they care about via `..Default::default()`.
    #[test]
    fn image_config_default_is_empty() {
        let cfg = ImageConfig::default();
        assert_eq!(serde_json::to_string(&cfg).unwrap(), "{}");
    }
}
