#!/bin/sh
set -e

export BUILDKIT_VERSION="v0.18"

# Create all required directories
mkdir -p proto/github.com/moby/buildkit/api/types
mkdir -p proto/github.com/moby/buildkit/frontend/gateway/pb
mkdir -p proto/github.com/moby/buildkit/solver/pb
mkdir -p proto/github.com/moby/buildkit/util/apicaps/pb
mkdir -p proto/github.com/moby/buildkit/sourcepolicy/pb
mkdir -p proto/github.com/gogo/googleapis/google/rpc
mkdir -p proto/google/rpc
mkdir -p proto/github.com/gogo/protobuf/gogoproto
mkdir -p proto/github.com/tonistiigi/fsutil/types
mkdir -p proto/github.com/planetscale/vtprotobuf/vtproto
mkdir -p proto/google/protobuf

# Download BuildKit protos
curl "https://raw.githubusercontent.com/moby/buildkit/$BUILDKIT_VERSION/api/types/worker.proto" > proto/github.com/moby/buildkit/api/types/worker.proto
curl "https://raw.githubusercontent.com/moby/buildkit/$BUILDKIT_VERSION/frontend/gateway/pb/gateway.proto" > proto/github.com/moby/buildkit/frontend/gateway/pb/gateway.proto
curl "https://raw.githubusercontent.com/moby/buildkit/$BUILDKIT_VERSION/solver/pb/ops.proto" > proto/github.com/moby/buildkit/solver/pb/ops.proto
curl "https://raw.githubusercontent.com/moby/buildkit/$BUILDKIT_VERSION/util/apicaps/pb/caps.proto" > proto/github.com/moby/buildkit/util/apicaps/pb/caps.proto
curl "https://raw.githubusercontent.com/moby/buildkit/$BUILDKIT_VERSION/sourcepolicy/pb/policy.proto" > proto/github.com/moby/buildkit/sourcepolicy/pb/policy.proto

# Download Google APIs
curl "https://raw.githubusercontent.com/googleapis/googleapis/master/google/rpc/status.proto" > proto/google/rpc/status.proto

# Download other dependencies
curl "https://raw.githubusercontent.com/gogo/protobuf/master/gogoproto/gogo.proto" > proto/github.com/gogo/protobuf/gogoproto/gogo.proto
curl "https://raw.githubusercontent.com/tonistiigi/fsutil/master/types/stat.proto" > proto/github.com/tonistiigi/fsutil/types/stat.proto
curl "https://raw.githubusercontent.com/planetscale/vtprotobuf/refs/heads/main/include/github.com/planetscale/vtprotobuf/vtproto/ext.proto" > proto/github.com/planetscale/vtprotobuf/vtproto/ext.proto

# Download protobuf standard files
curl "https://raw.githubusercontent.com/protocolbuffers/protobuf/main/src/google/protobuf/descriptor.proto" > proto/google/protobuf/descriptor.proto
curl "https://raw.githubusercontent.com/protocolbuffers/protobuf/main/src/google/protobuf/any.proto" > proto/google/protobuf/any.proto
