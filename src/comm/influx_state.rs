use crate::comm::{influx_error::InfluxErrorCode, influxdb::*, message::*};
use std::{
    fmt,
    time::{Duration, SystemTime},
};
pub struct DeviceState {
    total_bytes: usize,
    total_packet: usize,
    time_start: SystemTime,
    last_msg_count: Option<usize>,
    packet_lost: usize,
    last_sync: TimeStampCount,
    count_offset: u64,
}

impl DeviceState {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            total_packet: 0,
            time_start: SystemTime::now(),
            last_msg_count: None,
            packet_lost: 0,
            last_sync: TimeStampCount::new(),
            count_offset: 0,
        }
    }
    // Function to follow the data packet received by the app.
    // Check wether the packet has been received correctly or not.
    pub fn add_data_packet(
        &mut self,
        len: usize,
        count_id: usize,
    ) -> Result<usize, InfluxErrorCode> {
        let mut current_data_lost: usize = 0;
        match self.last_msg_count {
            Some(mut last_count) => {
                // Posible overflow
                if last_count > count_id {
                    if count_id == 0 {
                        current_data_lost = 0;
                    } else {
                        current_data_lost = count_id;
                        self.packet_lost += current_data_lost;
                    }
                } else if last_count == count_id {
                    // Suposse Duplicated
                    return Err(InfluxErrorCode::DataIdDuplicated);
                } else if last_count + 1 == count_id {
                    //Correct count_id
                    current_data_lost = 0;
                } else {
                    current_data_lost = count_id.checked_sub(last_count).unwrap_or(0);
                    self.packet_lost += current_data_lost;
                }
            }
            None => {}
        }
        self.total_packet += 1;
        self.total_bytes += len;

        self.last_msg_count = Some(count_id);
        return Ok(current_data_lost);
    }
    fn get_total_packet(&self) -> usize {
        self.total_packet
    }

    fn get_packet_lost(&self) -> usize {
        self.packet_lost
    }
    pub fn sync_state(
        &mut self,
        last_sync: TimeStampCount,
        server_time: Duration,
        sample_time: u64,
    ) -> Result<(), InfluxErrorCode> {
        self.last_sync = last_sync;
        //self.count_offset = self.calculate_count_offset(sample_time, server_time)?;
        Ok(())
    }
    pub fn get_count_offset(&self, current_buffer_id: u64) -> u64 {
        self.count_offset
    }
    pub fn get_last_sync(&self) -> TimeStampCount {
        self.last_sync.clone()
    }
    fn calculate_count_offset(
        &self,
        sample_time: u64,
        server_time: Duration,
    ) -> Result<u64, InfluxErrorCode> {
        if server_time.as_nanos() >= self.last_sync.timestamp as u128 {
            return Err(InfluxErrorCode::TimeError);
        }
        let diff_count = ((self.last_sync.timestamp as u128)
            .checked_sub(server_time.as_nanos())
            .unwrap_or(1)
            / (sample_time as u128 * 100)) as u64;

        if diff_count < self.last_sync.count {
            return Err(InfluxErrorCode::TimeError);
        }

        println!(
            "last_timestamp: {}, server_time: {}, diff: {}, diff_count: {}, last_sync_count: {}, sample freq {}",
            self.last_sync.timestamp,
            server_time.as_nanos(),
            (self.last_sync.timestamp as u128).checked_sub(server_time.as_nanos()).unwrap_or(0) ,
            diff_count,
            self.last_sync.count
            ,sample_time
        );
        Ok(diff_count.checked_sub(self.last_sync.count).unwrap_or(0))
    }
}

struct InfluxDbState {
    last_sync: TimeStampCount,
    count_offset: u32,
}
impl fmt::Display for DeviceState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time_elapsed = (self.time_start.elapsed().unwrap().as_millis() as f64) * 1000.0;
        write!(
            f,
            "Bitrate: {:.3}KB, Packet Lost: {}, Total Bytes {}, Total Packets {:.2}, Time Elapsed: {}",
            ((self.total_bytes as f64) * 1000.0) / time_elapsed,
            self.packet_lost,
            self.total_bytes,
            self.total_packet,
            time_elapsed
        )
    }
}
