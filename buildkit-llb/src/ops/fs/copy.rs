use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use buildkit_proto::pb;

use super::path::{LayerPath, UnsetPath};
use super::FileOperation;

use crate::serialization::{Context, Result};
use crate::utils::OutputIdx;

#[derive(Debug)]
pub struct CopyOperation<From: Debug, To: Debug> {
    source: From,
    destination: To,

    follow_symlinks: bool,
    recursive: bool,
    create_path: bool,
    wildcard: bool,

    mode: i32,
    mode_str: String,
    owner: Option<pb::ChownOpt>,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    unpack: bool,

    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
}

type OpWithoutSource = CopyOperation<UnsetPath, UnsetPath>;
type OpWithSource<'a> = CopyOperation<LayerPath<'a, PathBuf>, UnsetPath>;
type OpWithDestination<'a> =
    CopyOperation<LayerPath<'a, PathBuf>, (OutputIdx, LayerPath<'a, PathBuf>)>;

impl OpWithoutSource {
    pub(crate) fn new() -> OpWithoutSource {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        CopyOperation {
            source: UnsetPath,
            destination: UnsetPath,

            follow_symlinks: false,
            recursive: false,
            create_path: false,
            wildcard: false,

            mode: -1,
            mode_str: String::new(),
            owner: None,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            unpack: false,

            caps,
            description: Default::default(),
        }
    }

    pub fn from<P>(self, source: LayerPath<'_, P>) -> OpWithSource<'_>
    where
        P: AsRef<Path>,
    {
        CopyOperation {
            source: source.into_owned(),
            destination: UnsetPath,

            follow_symlinks: self.follow_symlinks,
            recursive: self.recursive,
            create_path: self.create_path,
            wildcard: self.wildcard,

            mode: self.mode,
            mode_str: self.mode_str,
            owner: self.owner,
            include_patterns: self.include_patterns,
            exclude_patterns: self.exclude_patterns,
            unpack: self.unpack,

            description: self.description,
            caps: self.caps,
        }
    }
}

impl<'a> OpWithSource<'a> {
    pub fn to<P>(self, output: OutputIdx, destination: LayerPath<'a, P>) -> OpWithDestination<'a>
    where
        P: AsRef<Path>,
    {
        CopyOperation {
            source: self.source,
            destination: (output, destination.into_owned()),

            follow_symlinks: self.follow_symlinks,
            recursive: self.recursive,
            create_path: self.create_path,
            wildcard: self.wildcard,

            mode: self.mode,
            mode_str: self.mode_str,
            owner: self.owner,
            include_patterns: self.include_patterns,
            exclude_patterns: self.exclude_patterns,
            unpack: self.unpack,

            description: self.description,
            caps: self.caps,
        }
    }
}

impl<'a> OpWithDestination<'a> {
    pub fn into_operation(self) -> super::sequence::SequenceOperation<'a> {
        super::sequence::SequenceOperation::new().append(self)
    }
}

impl<From, To> CopyOperation<From, To>
where
    From: Debug,
    To: Debug,
{
    pub fn follow_symlinks(mut self, value: bool) -> Self {
        self.follow_symlinks = value;
        self
    }

    pub fn recursive(mut self, value: bool) -> Self {
        self.recursive = value;
        self
    }

    pub fn create_path(mut self, value: bool) -> Self {
        self.create_path = value;
        self
    }

    pub fn wildcard(mut self, value: bool) -> Self {
        self.wildcard = value;
        self
    }

    /// Override the permission bits of the copied files (`COPY --chmod`).
    /// Pass the mode as an integer (e.g. `0o755`); `-1` keeps the source mode.
    pub fn chmod(mut self, mode: i32) -> Self {
        self.mode = mode;
        self
    }

    /// Override the permissions of the copied files using a non-octal mode
    /// string (used when the value can't be represented as octal bits).
    pub fn chmod_str<S>(mut self, mode: S) -> Self
    where
        S: Into<String>,
    {
        self.mode_str = mode.into();
        self
    }

    /// Override the owner of the copied files (`COPY --chown`).
    pub fn chown(mut self, owner: pb::ChownOpt) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Only copy files/directories matching at least one of these patterns.
    pub fn include_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.include_patterns = patterns.into_iter().map(Into::into).collect();
        self
    }

    /// Exclude files/directories matching any of these patterns (`COPY --exclude`).
    pub fn exclude_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.exclude_patterns = patterns.into_iter().map(Into::into).collect();
        self
    }

    /// Automatically unpack a source archive into the destination (`ADD` archive
    /// behaviour).
    pub fn unpack(mut self, value: bool) -> Self {
        self.unpack = value;
        self
    }
}

impl<'a> FileOperation for OpWithDestination<'a> {
    fn output(&self) -> i32 {
        self.destination.0.into()
    }

    fn serialize_inputs(&self, cx: &mut Context) -> Result<Vec<pb::Input>> {
        let mut inputs = if let LayerPath::Other(ref op, ..) = self.source {
            let serialized_from_head = cx.register(op.operation())?;

            vec![pb::Input {
                digest: serialized_from_head.digest.clone(),
                index: op.output().into(),
            }]
        } else {
            vec![]
        };

        if let LayerPath::Other(ref op, ..) = self.destination.1 {
            let serialized_to_head = cx.register(op.operation())?;

            inputs.push(pb::Input {
                digest: serialized_to_head.digest.clone(),
                index: op.output().into(),
            });
        }

        Ok(inputs)
    }

    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> Result<pb::FileAction> {
        let (src_idx, src_offset, src) = match self.source {
            LayerPath::Scratch(ref path) => (-1, 0, path.to_string_lossy().into()),

            LayerPath::Other(_, ref path) => {
                (inputs_offset as i64, 1, path.to_string_lossy().into())
            }

            LayerPath::Own(ref output, ref path) => {
                let output: i64 = output.into();

                (
                    inputs_count as i64 + output,
                    0,
                    path.to_string_lossy().into(),
                )
            }
        };

        let (dest_idx, dest) = match self.destination.1 {
            LayerPath::Scratch(ref path) => (-1, path.to_string_lossy().into()),

            LayerPath::Other(_, ref path) => (
                inputs_offset as i32 + src_offset,
                path.to_string_lossy().into(),
            ),

            LayerPath::Own(ref output, ref path) => {
                let output: i32 = output.into();

                (inputs_count as i32 + output, path.to_string_lossy().into())
            }
        };

        Ok(pb::FileAction {
            input: i64::from(dest_idx),
            secondary_input: src_idx,

            output: i64::from(self.output()),

            action: Some(pb::file_action::Action::Copy(pb::FileActionCopy {
                src,
                dest,

                follow_symlink: self.follow_symlinks,
                dir_copy_contents: self.recursive,
                create_dest_path: self.create_path,
                allow_wildcard: self.wildcard,

                owner: self.owner.clone(),
                mode: self.mode,
                mode_str: self.mode_str.clone(),

                attempt_unpack_docker_compatibility: self.unpack,
                include_patterns: self.include_patterns.clone(),
                exclude_patterns: self.exclude_patterns.clone(),

                // TODO: make this configurable
                timestamp: -1,

                ..Default::default()
            })),
        })
    }
}
