use chrono::Utc;

pub mod local;

pub enum StorageBackendType {
    Local,
}

#[async_trait::async_trait]
pub trait StorageHandler {
    async fn healthcheck(&self) -> Result<(), String>;
    async fn quota(&self) -> Result<u64, String>;
    async fn store(&self, backup_path: &str) -> Result<(), String>;
    async fn list(&self, vm_uuid: &str) -> Result<Vec<String>, String>;
    async fn rotate(&self, vm_uuid: &str, max_backups: u32) -> Result<(), String>;
    async fn generate_backup_name(
        &self,
        vm_name: &str,
        time_stamp: &str,
        xcp_host: &str,
        backup_type: &str,
    ) -> String;
}
