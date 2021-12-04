use crate::comm::config::*;
use crate::comm::devices::traits::*;
use crate::comm::influx_error::InfluxErrorCode;
use crate::comm::influx_state::*;
use crate::comm::influxdb::{InfluxDbContext, InfluxDbId, InfluxDbPrecision};
use crate::comm::message::*;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashMap, fmt, mem};

use chrono::{DateTime, Local, TimeZone};
use std::{error::Error, fs};
static CC32XX_TOPIC_STR: &str = "CC32xx";

static CC32XX_TYPE_OF_MEASUREMENT: &str = "test_phase_3";
static CC32XX_CATEFORY: &str = "device";
static CC32XX_MEASURE: &str = "plhp-sensor";
const CC32XX_SAMPLE_RATE: u64 = 128000;
const CC32XX_TIME_PRECISION: InfluxDbPrecision = InfluxDbPrecision::MicroSeconds;

pub struct CC32xxDevice<'a> {
    id_device_state: Arc<RwLock<HashMap<InfluxDbId, DeviceState>>>,
    id_db_context: Arc<RwLock<HashMap<InfluxDbId, InfluxDbContext<'a>>>>,
    mac_id: Arc<RwLock<HashMap<CC32xxUniqueIdType, InfluxDbId>>>,
    path_config: String,
}
impl<'a> CC32xxDevice<'a> {
    fn next_id(current_lenght: usize) -> InfluxDbId {
        InfluxDbId {
            id: (current_lenght + 1) as u32,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CC32xxUniqueIdType {
    pub id: [u8; 6],
}
impl CC32xxUniqueIdType {
    pub fn get_id_string(&self) -> String {
        format!("{}", self)
    }
}
impl fmt::Display for CC32xxUniqueIdType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.id[0], self.id[1], self.id[2], self.id[3], self.id[4], self.id[5]
        )
    }
}

// Trait mandatory for sensor devices
impl<'b> Devices for CC32xxDevice<'b> {
    type UniqueIdType = CC32xxUniqueIdType;
    type DataBufferType = CC32xxDataBuffer;
    fn new(path_config: String) -> Self {
        CC32xxDevice {
            id_device_state: Arc::new(RwLock::new(HashMap::<InfluxDbId, DeviceState>::new())),
            mac_id: Arc::new(RwLock::new(HashMap::<Self::UniqueIdType, InfluxDbId>::new())),
            id_db_context: Arc::new(RwLock::new(HashMap::<InfluxDbId, InfluxDbContext>::new())),
            path_config,
        }
    }
    /// Add Unique Id to the system and return a local id
    ///
    /// #Errors
    ///  WriteLockFailed: Failed when try to read from mutex
    ///
    fn add_unique_id(
        &self,
        unique_id: Self::UniqueIdType,
        server_time: std::time::Duration,
    ) -> Result<InfluxDbId, InfluxErrorCode> {
        // Check if the MAC is present in the config device file
        let parsed = ConfigFile::load_config(self.path_config.clone())
            .map_err(|_| InfluxErrorCode::NotValidConfigFile)?;
        let device_name = parsed.devices.get(&unique_id.get_id_string());
        if let None = device_name {
            return Err(InfluxErrorCode::UniqueIdNotRegistered);
        }
        let device_name = device_name.unwrap().to_owned();

        let mut write_mac_id = self.mac_id.write();
        match write_mac_id {
            Err(_error) => return Err(InfluxErrorCode::WriteLockFailed),
            Ok(mut mac_id) => {
                let next_id = Self::next_id(mac_id.len());
                let id = mac_id.entry(unique_id).or_insert(next_id);

                // Add entry to db_context
                let write_id_db_context = self.id_db_context.write();
                match write_id_db_context {
                    Err(_) => {
                        return Err(InfluxErrorCode::WriteLockFailed);
                    }
                    Ok(mut id_db_context) => {
                        id_db_context.insert(*id, Self::create_db_context(device_name));
                    }
                }

                // Add entry to device_state
                let write_id_device_state = self.id_device_state.write();

                match write_id_device_state {
                    Err(_) => return Err(InfluxErrorCode::WriteLockFailed),
                    Ok(mut id_device_state) => {
                        id_device_state.insert(*id, self.create_device_state(*id, server_time)?);
                    }
                }

                Ok(*id)
            }
        }
    }
    fn create_device_state(
        &self,
        id: InfluxDbId,
        server_time: std::time::Duration,
    ) -> Result<DeviceState, InfluxErrorCode> {
        let mut device_state = DeviceState::new();
        device_state.sync_state(
            TimeStampCount::new(),
            server_time,
            self.get_sample_period(id)?,
        );
        Ok(device_state)
    }
    fn add_data_packet(
        &self,
        id: InfluxDbId,
        len: usize,
        count_id: usize,
    ) -> Result<usize, InfluxErrorCode> {
        let id_device_state = self.id_device_state.write();
        if let Err(_error) = id_device_state {
            return Err(InfluxErrorCode::WriteLockFailed);
        }

        match id_device_state.unwrap().get_mut(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(mut id_device_state) => {
                let count_id = id_device_state.add_data_packet(len, count_id)?;
                Ok(count_id)
            }
        }
    }
    fn get_sample_period(&self, id: InfluxDbId) -> Result<u64, InfluxErrorCode> {
        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        match id_db_context.unwrap().get(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(db_context) => {
                let precision = db_context.get_sample_period()?;

                Ok(precision)
            }
        }
    }
    fn get_precision(&self, id: InfluxDbId) -> Result<InfluxDbPrecision, InfluxErrorCode> {
        self.check_id(id)?;

        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        match id_db_context.unwrap().get(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(db_context) => {
                let precision = db_context.get_precision()?;
                Ok(precision)
            }
        }
    }
    fn get_name(&self, id: InfluxDbId) -> Result<String, InfluxErrorCode> {
        self.check_id(id)?;

        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        match id_db_context.unwrap().get(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(db_context) => {
                let name = db_context.get_name()?;
                Ok(name)
            }
        }
    }
    fn check_id(&self, id: InfluxDbId) -> Result<bool, InfluxErrorCode> {
        let mac_id = self.mac_id.read();
        if let Err(error) = mac_id {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        let mut coincidences: Vec<u32> = mac_id
            .unwrap()
            .iter()
            .filter_map(|(key, &val)| if val == id { Some(1) } else { None })
            .collect();
        match coincidences.len() {
            0 => return Err(InfluxErrorCode::IdDuplicated),
            1 => return Ok(true),
            _ => return Err(InfluxErrorCode::IdNotValid),
        }
    }
    fn create_db_context<'a>(name: String) -> InfluxDbContext<'a> {
        InfluxDbContext::new(
            name,
            CC32XX_TYPE_OF_MEASUREMENT,
            CC32XX_CATEFORY,
            CC32XX_MEASURE,
            CC32XX_SAMPLE_RATE,
            CC32XX_TIME_PRECISION,
        )
    }
    fn get_device_topic() -> String {
        format!("{}", CC32XX_TOPIC_STR)
    }
    fn get_maximum_n_devices() -> u32 {
        40
    }
    fn data_buffer_from_bytes(bytes: bytes::Bytes) -> Self::DataBufferType {
        CC32xxDataBuffer::new(bytes)
    }
    fn get_data_header(&self, id: InfluxDbId) -> Result<String, InfluxErrorCode> {
        self.check_id(id)?;

        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }

        match id_db_context.unwrap().get(&id) {
            None => Err(InfluxErrorCode::IdDbContext),
            Some(db_context) => db_context.gen_data_hedaer(),
        }
    }
    fn get_offset(&self, id: InfluxDbId, current_buffer_id: u128) -> Result<u128, InfluxErrorCode> {
        self.check_id(id)?;

        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        match id_db_context.unwrap().get(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(db_context) => {
                let sample_period = db_context.get_sample_period()?;

                let id_device_state = self.id_device_state.read();
                if let Err(_error) = id_device_state {
                    return Err(InfluxErrorCode::RxLockFailed);
                }
                match id_device_state.unwrap().get(&id) {
                    None => Err(InfluxErrorCode::IdNotValid),
                    Some(device_state) => {
                        let last_sync = device_state.get_last_sync();
                        let count_offset = current_buffer_id
                            .checked_sub(last_sync.count as u128 * 100)
                            .unwrap_or(current_buffer_id);
                        let timestamp =
                            last_sync.timestamp as u128 + (count_offset * sample_period as u128);
                        Ok(timestamp)
                    }
                }
            }
        }
    }

    fn sync_device(
        &self,
        id: InfluxDbId,
        last_sync: TimeStampCount,
        server_time: std::time::Duration,
    ) -> Result<(), InfluxErrorCode> {
        self.check_id(id)?;

        let id_db_context = self.id_db_context.read();
        if let Err(_error) = id_db_context {
            return Err(InfluxErrorCode::RxLockFailed);
        }
        match id_db_context.unwrap().get(&id) {
            None => {
                return Err(InfluxErrorCode::IdNotValid);
            }
            Some(db_context) => {
                let sample_period = db_context.get_sample_period()?;
                let id_device_state = self.id_device_state.write();
                if let Err(_error) = id_device_state {
                    return Err(InfluxErrorCode::WriteLockFailed);
                }

                match id_device_state.unwrap().get_mut(&id) {
                    None => {
                        return Err(InfluxErrorCode::IdNotValid);
                    }
                    Some(mut id_device_state) => {
                        id_device_state.sync_state(last_sync, server_time, sample_period)?;
                        Ok(())
                    }
                }
            }
        }
    }
}

pub struct CC32xxDataBuffer {
    pub header: CC32xxDataBufferHeader,
    pub buffer: Bytes,
    pub next_value: Option<u16>,
    last_time: Option<u128>,
    last_buffer_id: Option<u128>,
}
impl CC32xxDataBuffer {
    pub fn new(mut buffer: Bytes) -> CC32xxDataBuffer {
        let header = buffer.split_to(mem::size_of::<CC32xxDataBufferHeader>());
        let data_header: Result<CC32xxDataBufferHeader, _> = bincode::deserialize(&header[..]);

        CC32xxDataBuffer {
            header: data_header.unwrap(),
            buffer,
            next_value: None,
            last_time: None,
            last_buffer_id: None,
        }
    }
    pub fn id(&self) -> u64 {
        self.header.buffer_id
    }
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}
impl std::iter::Iterator for CC32xxDataBuffer {
    type Item = u16;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_value {
            Some(next_value) => {
                self.next_value = None;
                Some(next_value)
            }
            None => {
                if self.buffer.is_empty() == true {
                    return None;
                };

                let mut new_iter = if self.buffer.len() > 3 {
                    self.buffer.split_to(3).into_iter()
                } else {
                    let temp = self.buffer.slice(0..3).into_iter();
                    self.buffer.clear();
                    temp
                };

                let first_byte = new_iter.next().unwrap() as u16;
                let second_byte = new_iter.next().unwrap() as u16;
                let third_byte = new_iter.next().unwrap() as u16;
                self.next_value = Some(((second_byte & 0x000F) << 8) + ((third_byte) & 0x00FF));

                Some(((first_byte & 0x00FF) << 4) + ((second_byte & 0x00F0) >> 4))
            }
        }
    }
}

impl DataBuffer for CC32xxDataBuffer
where
    CC32xxDataBuffer: std::iter::Iterator,
{
    type DataBufferHeaderType = CC32xxDataBufferHeader;
    fn from_bytes(bytes: bytes::Bytes) -> Self {
        CC32xxDataBuffer::new(bytes)
    }
    fn db_create_body<T: Devices>(
        &mut self,
        id: InfluxDbId,
        device: &T,
        server_time: std::time::Duration,
        last_sync: TimeStampCount,
    ) -> Result<(String, u128), InfluxErrorCode> {
        let mut string_out = String::new();

        let mut last_value: u128 = 0;
        let mut last_buffer_id = 0;
        // ToDo get the number of buffers
        let start_count_id = self.header.buffer_id as u128 * 100 as u128;
        let mut index = 0;

        let device_precision = device.get_precision(id)?;
        let sample_period = device.get_sample_period(id)?;
        while let Some(value) = self.next() {
            let current_count_id = start_count_id as u128 + index as u128;
            index = index + 1;

            // To calculate the current timestamp:
            // 1. Substract last sync from server time.
            let interval_time = (last_sync.timestamp as u128)
                .checked_sub(server_time.as_nanos())
                .ok_or(InfluxErrorCode::TimeBeforeServerTime)?;
            // 2. Calculate the number of samples between these two times.
            let diff_count_last_sync = interval_time
                .checked_div(sample_period as u128)
                .ok_or(InfluxErrorCode::SamplePeriodNotDiv)?;

            // 3. Substract current count from last sync count.
            let diff_count = (current_count_id)
                .checked_sub(last_sync.count as u128 * 100)
                .ok_or(InfluxErrorCode::TimeBeforeLastSync)?;

            // 4. Server Time + (2, 3) * sample period.
            let current_timestamp = server_time.as_nanos()
                + ((diff_count + (diff_count_last_sync)) * sample_period as u128);
            let time_value = ((current_timestamp * device_precision.get_multiplier() as u128)
                / InfluxDbPrecision::NanoSeconds.get_multiplier() as u128)
                as u128;
            last_value = time_value;
            last_buffer_id = self.header.buffer_id;
            string_out.push_str(
                &(format!("{}={} {}\n", device.get_data_header(id)?, value, time_value).to_owned()),
            );
        }
        self.last_time = Some(last_value);

        // Construct a datetime from epoch:
        let dt = Local.timestamp_millis(last_value as i64 / (1000 as i64));
        let dt_diff = Local::now() - dt;

        let size_iterator = string_out.split("\n").count();
        let mut iterator_string = string_out.split("\n");

        // println!("{}", iterator_string.nth(size_iterator - 1).unwrap());
        Ok((string_out, last_value))
    }
    fn get_last_time(&self) -> Option<u128> {
        self.last_time
    }
    fn get_buffer_id(&self) -> u64 {
        self.header.buffer_id
    }
}

#[derive(Serialize, Deserialize)]
pub struct CC32xxDataBufferHeader {
    buffer_id: u64,
}
impl fmt::Display for CC32xxDataBufferHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Buffer id = {}", self.buffer_id)
    }
}
impl DataBufferHeader for CC32xxDataBufferHeader {
    fn get_buffer_id(&self) -> u64 {
        self.buffer_id
    }
}
