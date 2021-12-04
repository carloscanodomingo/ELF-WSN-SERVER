use crate::comm::influx_error::InfluxErrorCode;
use crate::comm::influx_state::*;
use crate::comm::influxdb::{InfluxDbContext, InfluxDbId, InfluxDbPrecision};
use crate::comm::message::*;
pub trait Devices {
    type UniqueIdType;
    type DataBufferType: DataBuffer;
    fn new(path_config: String) -> Self;

    fn add_data_packet(
        &self,
        id: InfluxDbId,
        len: usize,
        count_id: usize,
    ) -> Result<usize, InfluxErrorCode>;

    fn get_data_header(&self, id: InfluxDbId) -> Result<String, InfluxErrorCode>;
    fn check_id(&self, id: InfluxDbId) -> Result<bool, InfluxErrorCode>;
    fn add_unique_id(
        &self,
        unique_id: Self::UniqueIdType,
        server_time: std::time::Duration,
    ) -> Result<InfluxDbId, InfluxErrorCode>;
    fn create_db_context<'a>(name: String) -> InfluxDbContext<'a>;
    fn get_device_topic() -> String;
    fn get_maximum_n_devices() -> u32;

    fn get_sample_period(&self, id: InfluxDbId) -> Result<u64, InfluxErrorCode>;

    fn create_device_state(
        &self,
        id: InfluxDbId,
        server_time: std::time::Duration,
    ) -> Result<DeviceState, InfluxErrorCode>;

    fn get_precision(&self, id: InfluxDbId) -> Result<InfluxDbPrecision, InfluxErrorCode>;

    fn get_name(&self, id: InfluxDbId) -> Result<String, InfluxErrorCode>;

    fn get_offset(&self, id: InfluxDbId, current_buffer_id: u128) -> Result<u128, InfluxErrorCode>;
    fn data_buffer_from_bytes(bytes: bytes::Bytes) -> Self::DataBufferType;

    fn sync_device(
        &self,
        id: InfluxDbId,
        last_sync: TimeStampCount,
        server_time: std::time::Duration,
    ) -> Result<(), InfluxErrorCode>;
}
pub trait DataBuffer {
    type DataBufferHeaderType: DataBufferHeader;
    fn from_bytes(bytes: bytes::Bytes) -> Self;
    fn get_buffer_id(&self) -> u64;

    fn get_last_time(&self) -> Option<u128>;
    fn db_create_body<T: Devices>(
        &mut self,
        id: InfluxDbId,
        device: &T,
        db_time: std::time::Duration,

        tscount_received: TimeStampCount,
    ) -> Result<(String, u128), InfluxErrorCode>;
}
pub trait DataBufferHeader {
    fn get_buffer_id(&self) -> u64;
}

pub trait WriteDb {
    fn log_db(&self, id: InfluxDbId) -> u64;
}
