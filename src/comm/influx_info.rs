use crate::comm::influxdb::*;
use crate::*;
/// Type of Info message to write in InfluxDb
///
pub enum InfluxInfo {
    Error(InfoError),
    Rssi(InfoRssi),
}

/// Info asociated with the strenght of the device
///
pub struct InfoRssi {
    pub data_rssi: i16,
    pub ctrl_rssi: i16,
    pub error_pkt_num: u32,
}
impl InfoRssi {
    pub fn from_cmd(cmd_rssi: CmdRssi) -> Self {
        InfoRssi {
            data_rssi: cmd_rssi.data_rssi,
            ctrl_rssi: cmd_rssi.ctrl_rssi,
            error_pkt_num: cmd_rssi.error_pkt_num,
        }
    }
}
impl InfluxLogInfo for InfoRssi {
    fn gen_body(&self, name: String, epoch_nanos: u128) -> String {
        format!(
            "rssi,device={} data_rssi={},ctrl_rssi={},error_pkt_num={} {}",
            name, self.data_rssi, self.ctrl_rssi, self.error_pkt_num, epoch_nanos
        )
    }
}

/// Info associated with asyncronous error in server or device
///
pub struct InfoError {
    pub error: InfluxErrorCode,
    pub error_info: String,
}
impl InfluxLogInfo for InfoError {
    fn gen_body(&self, name: String, epoch_nanos: u128) -> String {
        format!(
            "error,device={} type_error=\"{:?}\",info_error=\"{}\" {}",
            name, self.error, self.error_info, epoch_nanos
        )
    }
}
/// Each info struct MUST implemented this trait
pub trait InfluxLogInfo {
    fn gen_body(&self, name: String, epoch_nanos: u128) -> String;
}
