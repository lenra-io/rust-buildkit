use std::collections::HashMap;
use std::path::{Path, PathBuf};

use buildkit_proto::pb;

use super::path::LayerPath;
use super::FileOperation;

use crate::serialization::{Context, Result};
use crate::utils::OutputIdx;

#[derive(Debug)]
pub struct MakeDirOperation<'a> {
    path: LayerPath<'a, PathBuf>,
    output: OutputIdx,

    make_parents: bool,
    mode: i32,
    owner: Option<pb::ChownOpt>,
    // description: HashMap<String, String>,
    // caps: HashMap<String, bool>,
}

impl<'a> MakeDirOperation<'a> {
    pub(crate) fn new<P>(output: OutputIdx, path: LayerPath<'a, P>) -> Self
    where
        P: AsRef<Path>,
    {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        MakeDirOperation {
            path: path.into_owned(),
            output,

            make_parents: false,
            mode: -1,
            owner: None,
            // caps,
            // description: Default::default(),
        }
    }

    pub fn make_parents(mut self, value: bool) -> Self {
        self.make_parents = value;
        self
    }

    /// Override the permission bits of the created directory. Pass the mode as
    /// an integer (e.g. `0o755`); `-1` uses the BuildKit default.
    pub fn chmod(mut self, mode: i32) -> Self {
        self.mode = mode;
        self
    }

    /// Override the owner of the created directory.
    pub fn chown(mut self, owner: pb::ChownOpt) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn into_operation(self) -> super::sequence::SequenceOperation<'a> {
        super::sequence::SequenceOperation::new().append(self)
    }
}

impl<'a> FileOperation for MakeDirOperation<'a> {
    fn output(&self) -> i32 {
        self.output.into()
    }

    fn serialize_inputs(&self, cx: &mut Context) -> Result<Vec<pb::Input>> {
        if let LayerPath::Other(ref op, ..) = self.path {
            let serialized_from_head = cx.register(op.operation())?;

            let inputs = vec![pb::Input {
                digest: serialized_from_head.digest.clone(),
                index: op.output().into(),
            }];

            Ok(inputs)
        } else {
            Ok(Vec::with_capacity(0))
        }
    }

    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> Result<pb::FileAction> {
        let (src_idx, path) = match self.path {
            LayerPath::Scratch(ref path) => (-1, path.to_string_lossy().into()),
            LayerPath::Other(_, ref path) => (inputs_offset as i64, path.to_string_lossy().into()),

            LayerPath::Own(ref output, ref path) => {
                let output: i64 = output.into();

                (inputs_count as i64 + output, path.to_string_lossy().into())
            }
        };

        Ok(pb::FileAction {
            input: src_idx,
            secondary_input: -1,

            output: i64::from(self.output()),

            action: Some(pb::file_action::Action::Mkdir(pb::FileActionMkDir {
                path,

                make_parents: self.make_parents,

                mode: self.mode,
                owner: self.owner.clone(),

                // TODO: make this configurable
                timestamp: -1,
            })),
        })
    }
}
