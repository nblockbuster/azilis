use destiny_pkg::{PackageManager, TagHash, TagHash64};
use eframe::epaint::mutex::RwLock;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    static ref PACKAGE_MANAGER: RwLock<Option<Arc<PackageManager>>> = RwLock::new(None);
}

pub fn initialize_package_manager(pm: PackageManager) {
    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));
}

pub fn package_manager_checked() -> anyhow::Result<Arc<PackageManager>> {
    PACKAGE_MANAGER
        .read()
        .as_ref()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Package manager is not initialized!"))
}

pub fn package_manager() -> Arc<PackageManager> {
    package_manager_checked().unwrap()
}
