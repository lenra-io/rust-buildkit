use std::io::Result;
fn main() -> Result<()> {
    tonic_build::configure().compile_protos(
        &[
            "proto/github.com/moby/buildkit/api/types/worker.proto",
            "proto/github.com/moby/buildkit/frontend/gateway/pb/gateway.proto",
            "proto/github.com/moby/buildkit/solver/pb/ops.proto",
            "proto/github.com/moby/buildkit/util/apicaps/pb/caps.proto",
            "proto/github.com/moby/buildkit/sourcepolicy/pb/policy.proto",
            "proto/google/rpc/status.proto",
            "proto/github.com/gogo/protobuf/gogoproto/gogo.proto",
            "proto/github.com/tonistiigi/fsutil/types/stat.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
