use std::path::{Path, PathBuf};

use buildkit_proto::pb;

use crate::utils::{OperationOutput, OutputIdx};

/// Operand of *command execution operation* that specifies how are input sources mounted.
#[derive(Debug, Clone)]
pub enum Mount<'a, P: AsRef<Path>> {
    /// Read-only output of another operation.
    ReadOnlyLayer(OperationOutput<'a>, P),

    /// Read-only output of another operation with a selector.
    ReadOnlySelector(OperationOutput<'a>, P, P),

    /// Writable bind mount of another operation's output. Changes are not
    /// persisted into the resulting layer (`--mount=type=bind,rw`).
    ReadWriteLayer(OperationOutput<'a>, P),

    /// Writable bind mount of another operation's output with a selector.
    ReadWriteSelector(OperationOutput<'a>, P, P),

    /// Empty layer that produces an output.
    Scratch(OutputIdx, P),

    /// Writable output of another operation.
    Layer(OutputIdx, OperationOutput<'a>, P),

    /// Writable persistent cache.
    SharedCache(P),

    /// Writable persistent cache with explicit options.
    /// The boolean is the read-only flag.
    Cache(P, pb::CacheOpt, bool),

    /// Persistent cache seeded from another operation's output.
    /// Arguments: input layer, destination, source selector (empty for root),
    /// cache options and the read-only flag.
    CacheFrom(OperationOutput<'a>, P, P, pb::CacheOpt, bool),

    /// tmpfs mount with options (e.g. a size limit).
    Tmpfs(P, pb::TmpfsOpt),

    /// Secret mount.
    Secret(P, pb::SecretOpt),

    /// Optional SSH agent socket at the specified path.
    OptionalSshAgent(P),

    /// SSH agent socket mount with explicit options.
    Ssh(P, pb::SshOpt),
}

impl<'a, P: AsRef<Path>> Mount<'a, P> {
    /// Transform the mount into owned variant (basically, with `PathBuf` as the path).
    pub fn into_owned(self) -> Mount<'a, PathBuf> {
        use Mount::*;

        match self {
            ReadOnlySelector(op, path, selector) => {
                ReadOnlySelector(op, path.as_ref().into(), selector.as_ref().into())
            }

            ReadOnlyLayer(op, path) => ReadOnlyLayer(op, path.as_ref().into()),
            ReadWriteSelector(op, path, selector) => {
                ReadWriteSelector(op, path.as_ref().into(), selector.as_ref().into())
            }
            ReadWriteLayer(op, path) => ReadWriteLayer(op, path.as_ref().into()),
            Scratch(output, path) => Scratch(output, path.as_ref().into()),
            Layer(output, input, path) => Layer(output, input, path.as_ref().into()),
            SharedCache(path) => SharedCache(path.as_ref().into()),
            Cache(path, opt, readonly) => Cache(path.as_ref().into(), opt, readonly),
            CacheFrom(op, path, selector, opt, readonly) => CacheFrom(
                op,
                path.as_ref().into(),
                selector.as_ref().into(),
                opt,
                readonly,
            ),
            Tmpfs(path, opt) => Tmpfs(path.as_ref().into(), opt),
            Secret(path, opt) => Secret(path.as_ref().into(), opt),
            OptionalSshAgent(path) => OptionalSshAgent(path.as_ref().into()),
            Ssh(path, opt) => Ssh(path.as_ref().into(), opt),
        }
    }

    pub fn is_root(&self) -> bool {
        use Mount::*;

        let path = match self {
            ReadOnlySelector(_, path, ..) => path,
            ReadOnlyLayer(_, path) => path,
            ReadWriteSelector(_, path, ..) => path,
            ReadWriteLayer(_, path) => path,
            Scratch(_, path) => path,
            Layer(_, _, path) => path,
            SharedCache(path) => path,
            Cache(path, ..) => path,
            CacheFrom(_, path, ..) => path,
            Tmpfs(path, ..) => path,
            Secret(_, _) => return false,
            OptionalSshAgent(_) => return false,
            Ssh(_, _) => return false,
        };

        path.as_ref() == Path::new("/")
    }
}
