use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct UserInterfaceCfgCluster;

impl ClusterHandler for UserInterfaceCfgCluster {
    fn process_reports(&self, _reports: &[AttributeReport]) -> Vec<(String, serde_json::Value)> {
        Vec::new()
    }
}
