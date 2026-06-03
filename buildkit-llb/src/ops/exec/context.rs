use std::iter::once;
use std::path::PathBuf;

use buildkit_proto::pb::Meta;

#[derive(Debug, Clone)]
pub(crate) struct Context {
    pub name: String,
    pub args: Vec<String>,
    pub env: Vec<String>,

    pub cwd: PathBuf,
    pub user: String,
}

impl Context {
    pub fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),

            cwd: PathBuf::from("/"),
            user: "root".into(),

            args: vec![],
            env: vec![],
        }
    }
}

impl From<Context> for Meta {
    fn from(val: Context) -> Self {
        Meta {
            args: {
                once(val.name.clone())
                    .chain(val.args.iter().cloned())
                    .collect()
            },

            env: val.env,
            cwd: val.cwd.to_string_lossy().into(),
            user: val.user,

            ..Default::default()
        }
    }
}
