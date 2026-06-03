use std::collections::HashMap;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op, MergeInput, MergeOp, OpMetadata};

use crate::ops::{OperationBuilder, SingleBorrowedOutput, SingleOwnedOutput};
use crate::serialization::{Context, Node, Operation, OperationId, Result};
use crate::utils::{OperationOutput, OutputIdx};

/// Merges the layers of several operations into a single output. This is what a
/// Dockerfile's `COPY --link` is translated to: the copy is performed on an
/// independent layer which is then merged on top of the previous state, so the
/// resulting layer stays cacheable independently of the base.
#[derive(Debug)]
pub struct MergeOperation<'a> {
    id: OperationId,
    inputs: Vec<OperationOutput<'a>>,
    description: HashMap<String, String>,
    ignore_cache: bool,
}

impl<'a> MergeOperation<'a> {
    /// Create a merge of the given operation outputs, in order (later inputs
    /// are layered on top of earlier ones).
    pub fn new(inputs: Vec<OperationOutput<'a>>) -> Self {
        Self {
            id: OperationId::default(),
            inputs,
            description: Default::default(),
            ignore_cache: false,
        }
    }
}

impl<'a> SingleBorrowedOutput<'a> for MergeOperation<'a> {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutput<'a> for Arc<MergeOperation<'a>> {
    fn output(&self) -> OperationOutput<'a> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}

impl<'a> OperationBuilder<'a> for MergeOperation<'a> {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }

    fn ignore_cache(mut self, ignore: bool) -> Self {
        self.ignore_cache = ignore;
        self
    }
}

impl<'a> Operation for MergeOperation<'a> {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, cx: &mut Context) -> Result<Node> {
        let mut inputs = Vec::with_capacity(self.inputs.len());
        let mut merge_inputs = Vec::with_capacity(self.inputs.len());

        for (index, input) in self.inputs.iter().enumerate() {
            let serialized = cx.register(input.operation())?;
            inputs.push(pb::Input {
                digest: serialized.digest.clone(),
                index: input.output().into(),
            });
            merge_inputs.push(MergeInput {
                input: index as i64,
            });
        }

        let head = pb::Op {
            inputs,
            op: Some(Op::Merge(MergeOp {
                inputs: merge_inputs,
            })),

            ..Default::default()
        };

        let mut caps = HashMap::new();
        caps.insert("mergeop".into(), true);

        let metadata = OpMetadata {
            description: self.description.clone(),
            ignore_cache: self.ignore_cache,
            caps,

            ..Default::default()
        };

        Ok(Node::new(head, metadata))
    }
}
