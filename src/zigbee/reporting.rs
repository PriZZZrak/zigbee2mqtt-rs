use crate::coordinator::CoordinatorHandle;
use crate::error::Result;

pub async fn bind(
    endpoint: u8,
    target_ieee: [u8; 8],
    coord: &CoordinatorHandle,
    nwk: u16,
    cluster: u16,
) -> Result<()> {
    coord
        .request_bind(nwk, target_ieee, endpoint, cluster, [0; 8], 1)
        .await
}
pub fn payload(attribute: &str, min: u16, max: u16, change: u16) -> serde_json::Value {
    serde_json::json!([{"attribute": attribute, "minimumReportInterval": min, "maximumReportInterval": max, "reportableChange": change}])
}
