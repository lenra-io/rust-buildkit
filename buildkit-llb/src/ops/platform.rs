use buildkit_proto::pb;

/// Platform descriptor as used by BuildKit's `pb::Op.platform`. Re-exported
/// from `buildkit-proto` so callers can construct one directly when they
/// need a knob the helpers below don't expose.
pub use buildkit_proto::pb::Platform;

/// Constants reused by all helpers.
const LINUX: &str = "linux";
const WINDOWS: &str = "windows";
const DARWIN: &str = "darwin";

/// `linux/amd64`.
pub fn linux_amd64() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "amd64".into(),
        ..pb::Platform::default()
    }
}

/// `linux/arm64`.
pub fn linux_arm64() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "arm64".into(),
        ..pb::Platform::default()
    }
}

/// `linux/arm/v7`.
pub fn linux_arm_v7() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "arm".into(),
        variant: "v7".into(),
        ..pb::Platform::default()
    }
}

/// `linux/arm/v6`.
pub fn linux_arm_v6() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "arm".into(),
        variant: "v6".into(),
        ..pb::Platform::default()
    }
}

/// `linux/386`.
pub fn linux_386() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "386".into(),
        ..pb::Platform::default()
    }
}

/// `linux/ppc64le`.
pub fn linux_ppc64le() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "ppc64le".into(),
        ..pb::Platform::default()
    }
}

/// `linux/s390x`.
pub fn linux_s390x() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "s390x".into(),
        ..pb::Platform::default()
    }
}

/// `linux/riscv64`.
pub fn linux_riscv64() -> Platform {
    Platform {
        os: LINUX.into(),
        architecture: "riscv64".into(),
        ..pb::Platform::default()
    }
}

/// `windows/amd64`.
pub fn windows_amd64() -> Platform {
    Platform {
        os: WINDOWS.into(),
        architecture: "amd64".into(),
        ..pb::Platform::default()
    }
}

/// `darwin/amd64`.
pub fn darwin_amd64() -> Platform {
    Platform {
        os: DARWIN.into(),
        architecture: "amd64".into(),
        ..pb::Platform::default()
    }
}

/// `darwin/arm64`.
pub fn darwin_arm64() -> Platform {
    Platform {
        os: DARWIN.into(),
        architecture: "arm64".into(),
        ..pb::Platform::default()
    }
}

/// Canonical platform identifier as used as a key in BuildKit's
/// `RefMap` and as the suffix on the `containerimage.config/<id>`
/// metadata key. Examples: `linux/amd64`, `linux/arm/v7`.
pub fn platform_id(platform: &Platform) -> String {
    platform.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(platform_id(&linux_amd64()), "linux/amd64");
        assert_eq!(platform_id(&linux_arm64()), "linux/arm64");
        assert_eq!(platform_id(&linux_arm_v7()), "linux/arm/v7");
        assert_eq!(platform_id(&linux_arm_v6()), "linux/arm/v6");
        assert_eq!(platform_id(&windows_amd64()), "windows/amd64");
        assert_eq!(platform_id(&darwin_arm64()), "darwin/arm64");
    }
}
