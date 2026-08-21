use crate::core::publish::{CapabilityDir, verify_current};
use cap_std::ambient_authority;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct InstallationIntegrityReport {
    pub code: &'static str,
    pub safe_action: &'static str,
}

pub fn verify_installation(root: &Path) -> InstallationIntegrityReport {
    if !root.is_dir() {
        return InstallationIntegrityReport {
            code: "DOCTOR_ROOT_INVALID",
            safe_action: "choose an existing installation directory",
        };
    }
    if !root.join(".clroom/out/current.json").exists() {
        return InstallationIntegrityReport {
            code: "DOCTOR_ARTIFACT_NOT_PUBLISHED",
            safe_action: "run prepare before checking a published artifact",
        };
    }
    let Ok(directory) = CapabilityDir::open_ambient_dir(root, ambient_authority()) else {
        return InstallationIntegrityReport {
            code: "DOCTOR_ARTIFACT_INVALID",
            safe_action: "restore the installation directory from a trusted copy",
        };
    };
    if verify_current(&directory).is_ok() {
        InstallationIntegrityReport {
            code: "DOCTOR_ARTIFACT_PASS",
            safe_action: "none",
        }
    } else {
        InstallationIntegrityReport {
            code: "DOCTOR_ARTIFACT_INVALID",
            safe_action: "restore the published artifact from a trusted copy",
        }
    }
}
