use crate::comm::devices::traits::{DataBuffer, Devices};
use crate::comm::influx_error::InfluxErrorCode;
use crate::comm::influx_info::*;
use crate::comm::influx_state::*;

use crate::comm::message::*;
use chrono::{DateTime, Local, TimeZone};
use reqwest::Client;
use std::time::SystemTime;
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct InfluxDbId {
    pub id: u32,
}

impl std::fmt::Display for InfluxDbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
pub struct InfluxDbClient {
    http_client: Client,
    url: String,
    port: String,
    org_id: String,
    bucket: String,
    token: String,
    last_db_timestamp: std::time::Duration,
}
impl InfluxDbClient {
    pub fn new(
        url: String,
        port: String,
        org_id: String,
        bucket: String,
        token: String,
    ) -> InfluxDbClient {
        InfluxDbClient {
            http_client: Client::new(),
            last_db_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            url,
            port,
            org_id,
            bucket,
            token,
        }
    }
    /// Function to write a Inlfux Info message in the database
    pub async fn log_info<T: InfluxLogInfo>(
        &self,
        device_name: String,
        id: InfluxDbId,
        influx_info: T,
    ) -> Result<String, InfluxErrorCode> {
        let start = SystemTime::now();
        let since_epoch = start
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let (key_header, value_header) = self.gen_header();
        let string_body = influx_info.gen_body(device_name, since_epoch);
        let req = self
            .http_client
            .post(&self.gen_url(InfluxDbPrecision::NanoSeconds))
            .header(&key_header[..], &value_header[..])
            .body(string_body);

        let res = req.send().await;
        match res {
            Ok(res) => Ok(res.status().to_string()),

            _ => Err(InfluxErrorCode::IdDbContext),
        }
    }

    fn gen_url(&self, precision: InfluxDbPrecision) -> String {
        let out = format!(
            "{}:{}/api/v2/write?orgID={}&bucket={}&precision={}",
            self.url, self.port, self.org_id, self.bucket, precision,
        );
        out
    }

    fn gen_header(&self) -> (String, String) {
        (
            String::from("Authorization"),
            format!("Token {}", self.token),
        )
    }
    pub async fn write_data<TypeDevice: Devices>(
        &self,
        id: InfluxDbId,
        device: &TypeDevice,
        last_sync: TimeStampCount,
        data: Vec<TypeDevice::DataBufferType>,
    ) -> Result<String, InfluxErrorCode> {
        let precision = device.get_precision(id)?;
        let (key_header, value_header) = self.gen_header();
        let string_body = self.gen_body(data, last_sync, id, device, self.last_db_timestamp)?;

        println!("BODY: {}", string_body);
        let req = self
            .http_client
            .post(&self.gen_url(precision))
            .header(&key_header[..], &value_header[..])
            .body(string_body.clone());
        println!("Header: {}", &self.gen_url(precision));
        println!("Header_2: {} - {}", &key_header[..], &value_header[..]);
        let res = req.send().await;
        match res {
            Ok(res) => Ok(res.status().to_string()),

            _ => Err(InfluxErrorCode::IdDbContext),
        }
    }
    fn gen_body<TypeDevice: Devices>(
        &self,
        data: Vec<TypeDevice::DataBufferType>,
        last_sync: TimeStampCount,
        id: InfluxDbId,
        device: &TypeDevice,
        duration: std::time::Duration,
    ) -> Result<String, InfluxErrorCode> {
        let last_data = data.last().unwrap().clone();
        let last_value = last_data.get_buffer_id();
        drop(last_data);
        let mut last_epoch = 0;
        let out = data
            .into_iter()
            .filter_map(|mut pkt| -> Option<String> {
                let value = pkt.db_create_body(id, device, duration, last_sync);
                match value {
                    Ok((body, last_time)) => {
                        last_epoch = last_time;
                        Some(body)
                    }
                    Err(error) => None,
                }
            }) // NOT VALID
            .collect::<Vec<String>>()
            .join("");

        //println!("{}", out.to_string());

        let dt = Local.timestamp_millis(last_epoch as i64 / (1000 as i64));
        let dt_diff = Local::now() - dt;
        println!(
            "TimeStamp: {}, Count: {}, Device: {} Diff {}, last_buffer_count {}, last epoch: {}",
            last_sync.timestamp,
            last_sync.count,
            dt.to_rfc3339(),
            dt_diff.num_milliseconds(),
            last_value,
            last_epoch
        );
        Ok(out)
    }
}

pub struct InfluxDbContext<'a> {
    name: String,
    type_of_measurement: &'a str,
    category: &'a str,
    measure: &'a str,
    sample_period: u64,
    precision: InfluxDbPrecision,
}
impl<'a> InfluxDbContext<'a> {
    pub fn new(
        name: String,
        type_of_measurement: &'a str,
        category: &'a str,
        measure: &'a str,
        sample_period: u64,
        precision: InfluxDbPrecision,
    ) -> InfluxDbContext<'a> {
        InfluxDbContext {
            name,
            precision,
            sample_period,
            type_of_measurement,
            category,
            measure,
        }
    }

    pub fn gen_data_hedaer(&self) -> Result<String, InfluxErrorCode> {
        let data_header_str = format!(
            "{},{}={} {}",
            self.type_of_measurement, self.category, self.name, self.measure
        );
        Ok(data_header_str)
    }

    pub fn get_precision_multiplier(&self) -> u32 {
        self.precision.get_multiplier()
    }
    pub fn get_precision(&self) -> Result<InfluxDbPrecision, InfluxErrorCode> {
        Ok(self.precision)
    }
    pub fn get_name(&self) -> Result<String, InfluxErrorCode> {
        Ok(self.name.clone())
    }
    pub fn get_sample_period(&self) -> Result<u64, InfluxErrorCode> {
        Ok(self.sample_period)
    }
}

#[derive(Copy, Clone)]
pub enum InfluxDbPrecision {
    Seconds,
    MiliSeconds,
    MicroSeconds,
    NanoSeconds,
}
impl InfluxDbPrecision {
    pub fn get_multiplier(&self) -> u32 {
        match self {
            InfluxDbPrecision::Seconds => 1,
            InfluxDbPrecision::MiliSeconds => 1000,
            InfluxDbPrecision::MicroSeconds => 1000000,
            InfluxDbPrecision::NanoSeconds => 1000000000,
        }
    }
    pub fn convert_duration(&self, duration: std::time::Duration) -> u128 {
        match self {
            InfluxDbPrecision::Seconds => duration.as_secs() as u128,
            InfluxDbPrecision::MiliSeconds => duration.as_millis(),
            InfluxDbPrecision::MicroSeconds => duration.as_micros(),
            InfluxDbPrecision::NanoSeconds => duration.as_nanos(),
        }
    }
    pub fn precision_db_url(&self) -> String {
        format!("{}", self)
    }
}
impl std::fmt::Display for InfluxDbPrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InfluxDbPrecision::Seconds => write!(f, "s"),
            InfluxDbPrecision::MiliSeconds => write!(f, "ms"),
            InfluxDbPrecision::MicroSeconds => write!(f, "us"),
            InfluxDbPrecision::NanoSeconds => write!(f, "ns"),
        }
    }
}
