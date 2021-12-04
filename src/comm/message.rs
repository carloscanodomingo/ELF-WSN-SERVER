use crate::comm::devices::traits::{DataBuffer, Devices};
use crate::comm::influx_error::InfluxErrorCode;
use crate::comm::influxdb::*;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::{fmt, mem, time::SystemTime};

static SERVER_ID: u8 = 0;
#[path = "devices/mod.rs"]
mod devices;
#[derive(PartialEq)]
enum CmdId {
    DeviceReady = 1,
    RespId = 2,
    DeviceSync = 3,
    Rssi = 4,
}
#[derive(Debug)]
pub enum MsgCmdType {
    Ready(CmdReady),
    SyncChrono(CmdSync),
    RespId(CmdRespId),
    Rssi(CmdRssi),
}
#[derive(Serialize, Deserialize)]
pub struct MsgCmdHeader {
    id: u16,
}
pub struct MsgCmd {
    id: MsgCmdHeader,
    payload: Bytes,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(C, packed(1))]
pub struct CmdReady {
    pub mac: [u8; 6],
    pub sample_frequency: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdSync {
    pub buffer_count: u64,
    pub timestamp_ns: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdRespId {
    pub mac: [u8; 6],
    pub device_id: u16,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CmdRssi {
    pub data_rssi: i16,
    pub ctrl_rssi: i16,
    pub error_pkt_num: u32,
}

pub struct Msg {
    header: MsgHeader,
    payload: Bytes,
}
#[derive(Serialize, Deserialize)]
pub struct MsgHeader {
    pub device_id: u8,
    pub msg_type: u8,
    pub len: u16,
}

#[derive(Serialize, Deserialize)]
pub struct MsgDataHeader {
    pub data_id: u32,
    pub buffer_size: u16,
    pub n_buffers: u16,
    pub timestamp: TimeStampCount,
}
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct TimeStampCount {
    pub timestamp: u64,
    pub count: u64,
}
impl TimeStampCount {
    pub fn new() -> TimeStampCount {
        TimeStampCount {
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            count: 0,
        }
    }
}
impl fmt::Display for MsgDataHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Data ID: {}, Buffer size: {}, n buffers: {}",
            self.data_id, self.buffer_size, self.n_buffers
        )
    }
}

pub struct MsgData {
    header: MsgDataHeader,
    payload: Bytes,
}
impl fmt::Display for MsgHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Devide ID: {}, MSG_TYPE: {}, len: {}",
            self.device_id, self.msg_type, self.len
        )
    }
}

pub enum MsgContainer<T> {
    MsgDataContainer(u32, Vec<T>),
    MsgCmdContainer(u32),
    MsgError,
    MsgEmpty,
}

#[derive(PartialEq)]
pub enum MsgType {
    MqttCmd = 0x0001,
    MqttData,
}
impl fmt::Display for MsgType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MsgType::MqttCmd => {
                write!(f, "MQTT COMMAND")
            }
            MsgType::MqttData => {
                write!(f, "MQTT DATA")
            }
        }
    }
}
pub fn encode_msg_cmd(cmd: MsgCmdType) -> Result<Bytes, InfluxErrorCode> {
    match cmd {
        MsgCmdType::RespId(cmd_resp_id) => {
            let res_data = bincode::serialize(&cmd_resp_id);
            match res_data {
                Ok(mut data) => {
                    let mut send_cmd =
                        create_resp_header(CmdId::RespId, mem::size_of::<CmdRespId>());
                    send_cmd.append(&mut data);
                    return Ok(Bytes::from(send_cmd));
                }
                Err(_) => return Err(InfluxErrorCode::EncodeCmdError),
            }
        }

        MsgCmdType::SyncChrono(CmdSync) => {
            println!("SyncChrono Error");
            return Err(InfluxErrorCode::CmdTypeNotImplemented);
        }
        x => {
            println!("Enter here");
            return Err(InfluxErrorCode::RespCmdNotImplemented);
        }
    }
}
fn create_resp_header(id: CmdId, cmd_len: usize) -> Vec<u8> {
    let header = MsgHeader {
        device_id: SERVER_ID,
        msg_type: MsgType::MqttCmd as u8,
        len: (mem::size_of::<MsgHeader>() + mem::size_of::<MsgCmdHeader>() + cmd_len) as u16,
    };
    let mut output = bincode::serialize(&header).unwrap();
    let cmd_header = MsgCmdHeader { id: id as u16 };
    println!("cmd id: {}", cmd_header.id);
    output.append(&mut bincode::serialize(&cmd_header).unwrap());
    output
}

pub fn decode_msg(data: &mut Bytes) -> Result<MsgHeader, InfluxErrorCode> {
    let total_len = data.len() as u16;
    let bytes_header = data.split_to(mem::size_of::<MsgHeader>());
    let header_msg: Result<MsgHeader, _> = bincode::deserialize(&bytes_header[..]);
    match header_msg {
        Ok(header) => {
            println!("{}", header);
            if header.len != total_len {
                return Err(InfluxErrorCode::HeaderLengthIncorrect);
            } else {
                return Ok(header);
            }
        }
        Err(_) => {
            return Err(InfluxErrorCode::DecodeHeaderError);
        }
    }
}
pub fn decode_msg_cmd(data: &mut Bytes) -> Result<MsgCmdType, InfluxErrorCode> {
    let header = data.split_to(mem::size_of::<MsgCmdHeader>());
    let data_lenght = data.len();
    let cmd_header: Result<MsgCmdHeader, _> = bincode::deserialize(&header[..]);

    match cmd_header {
        Ok(header) => match header.id {
            id if id == (CmdId::DeviceReady as u16) => {
                println!(
                    "ID: {}, LEN: {}, expected: {}",
                    id,
                    data.len(),
                    mem::size_of::<CmdReady>()
                );
                check_cmd_type::<CmdReady>(&data)?;
                let res: Result<CmdReady, _> = bincode::deserialize(&data[..]);
                match res {
                    Ok(cmd_ready) => {
                        return Ok(MsgCmdType::Ready(cmd_ready));
                    }
                    Err(_) => return Err(InfluxErrorCode::CmdTypeDoesntMatch),
                }
            }
            id if id == (CmdId::Rssi as u16) => {
                check_cmd_type::<CmdRssi>(&data)?;
                let res: Result<CmdRssi, _> = bincode::deserialize(&data[..]);
                match res {
                    Ok(cmd) => {
                        return Ok(MsgCmdType::Rssi(cmd));
                    }
                    Err(_) => return Err(InfluxErrorCode::CmdTypeDoesntMatch),
                }
            }
            id if id == (CmdId::DeviceSync as u16) => {
                check_cmd_type::<CmdSync>(&data)?;
                let res: Result<CmdSync, _> = bincode::deserialize(&data[..]);
                match res {
                    Ok(cmd) => {
                        return Ok(MsgCmdType::SyncChrono(cmd));
                    }
                    Err(_) => return Err(InfluxErrorCode::CmdTypeDoesntMatch),
                }
            }
            _ => return Err(InfluxErrorCode::CmdTypeNotImplemented),
        },
        Err(x) => return Err(InfluxErrorCode::DecodeHeaderCmdError),
    }
}
fn check_cmd_type<T>(data: &Bytes) -> Result<u8, InfluxErrorCode> {
    if data.len() != std::mem::size_of::<T>() {
        return Err(InfluxErrorCode::CmdIncorrectLength);
    } else {
        Ok(0)
    }
}

pub fn decode_msg_data(data: &mut Bytes) -> Result<MsgDataHeader, InfluxErrorCode> {
    let bytes_header_data = data.split_to(mem::size_of::<MsgDataHeader>());
    let data_lenght = data.len();
    let data_header: Result<MsgDataHeader, _> = bincode::deserialize(&bytes_header_data[..]);
    match data_header {
        Ok(header) => {
            let num_buffers = (data_lenght as f32 / header.buffer_size as f32) as f32;
            if num_buffers.fract() != 0.0 {
                return Err(InfluxErrorCode::NumBuffersFrac);
            }
            if num_buffers as u32 != header.n_buffers as u32 {
                return Err(InfluxErrorCode::IncorrectNumBuffers);
            }
            return Ok(header);
        }
        Err(_) => {
            return Err(InfluxErrorCode::DecodeHeaderDataError);
        }
    }
}
pub fn decode_data<T: DataBuffer>(
    data: &mut Bytes,
    buffer_size: usize,
) -> Result<Vec<T>, InfluxErrorCode> {
    let total_len = data.len();
    let num_buffers = data.len() as f32 / buffer_size as f32;
    if num_buffers.fract() != 0.0 {
        return Err(InfluxErrorCode::NumBuffersFrac);
    }
    let mut output_vec: Vec<T> = Vec::new();
    for _ in 0..num_buffers as u32 {
        let current_buffer = data.split_to(buffer_size);
        output_vec.push(T::from_bytes(current_buffer));
    }
    return Ok(output_vec);
}
pub fn decode<T: DataBuffer>(mut data_in: Bytes) -> (MsgHeader, MsgContainer<T>) {
    let total_len = data_in.len();
    let bytes_header = data_in.split_to(mem::size_of::<MsgHeader>());
    let header_msg: Result<MsgHeader, _> = bincode::deserialize(&bytes_header[..]);
    let mut msg_output = MsgContainer::MsgEmpty;
    match &header_msg {
        Ok(header) => {
            if total_len != header.len as usize {
                println!(
                    "LEN NOT CORRECT- total len{}, header {}",
                    total_len, header.len
                );
            } else {
                //assert_eq!(total_len, header.len as usize);
                match header.msg_type {
                    cmd_type if cmd_type == MsgType::MqttCmd as u8 => {}
                    data_type if data_type == MsgType::MqttData as u8 => {
                        let bytes_header = data_in.split_to(mem::size_of::<MsgDataHeader>());
                        let data_header: Result<MsgDataHeader, _> =
                            bincode::deserialize(&bytes_header[..]);
                        match &data_header {
                            Ok(header) => {
                                let num_buffers =
                                    (data_in.len() as f32 / header.buffer_size as f32) as f32;
                                if num_buffers.fract() != 0.0
                                    || num_buffers as u16 != header.n_buffers
                                {
                                    println!(
                                        "Not valid buffer size - {} Data header: {}",
                                        data_in.len(),
                                        header
                                    );
                                } else {
                                    let mut data: Vec<T> = Vec::new();
                                    for i in 0..header.n_buffers {
                                        let current_buffer =
                                            data_in.split_to(header.buffer_size as usize);
                                        data.push(T::from_bytes(current_buffer));
                                    }
                                    msg_output =
                                        MsgContainer::MsgDataContainer(header.data_id, data);
                                    //let data: Vec<DataBuffer> = bytes_payload.spl
                                }
                            }
                            Err(_) => {
                                println!("Data header not valid")
                            }
                        }
                    }
                    _ => {
                        println!("Not contemplated data type")
                    }
                }
            }
        }
        Err(_) => {
            println!("Not a valid message")
        }
    }
    (header_msg.unwrap(), msg_output)
}
const DATA_BUFFER_LEN_TEST: usize = 32;
const DEVICE_ID: u8 = 7;
pub fn create_data_code() -> Bytes {
    #[derive(Serialize, Deserialize, Copy, Clone)]
    struct TestBuffer {
        buffer_id: u32,
        data: [u8; DATA_BUFFER_LEN_TEST],
    }
    impl TestBuffer {
        fn new() -> TestBuffer {
            TestBuffer {
                buffer_id: 0,

                data: [0; DATA_BUFFER_LEN_TEST],
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    struct TestPacket {
        header: MsgDataHeader,
        payload: [TestBuffer; DATA_BUFFER_LEN_TEST],
    }
    impl TestPacket {
        fn new() -> TestPacket {
            TestPacket {
                header: MsgDataHeader {
                    data_id: 0,
                    buffer_size: (DATA_BUFFER_LEN_TEST + 4) as u16,
                    n_buffers: DATA_BUFFER_LEN_TEST as u16,
                    timestamp: TimeStampCount {
                        count: 0,
                        timestamp: 0,
                    },
                },
                payload: [TestBuffer::new(); DATA_BUFFER_LEN_TEST],
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    struct TestMsg {
        header: MsgHeader,
        payload: TestPacket,
    }
    impl TestMsg {
        fn new() -> TestMsg {
            TestMsg {
                header: MsgHeader {
                    device_id: DEVICE_ID,
                    msg_type: MsgType::MqttData as u8,
                    len: mem::size_of::<MsgHeader>() as u16 + mem::size_of::<TestPacket>() as u16,
                },
                payload: TestPacket::new(),
            }
        }
    }
    let mut cnt_buffer: u32 = 0;
    let mut msg = TestMsg::new();

    for (_index, packet) in msg.payload.payload.iter_mut().enumerate() {
        packet.buffer_id = cnt_buffer;
        cnt_buffer += 1;
        for (index, data) in packet.data.iter_mut().enumerate() {
            let val: f32 = (2.0 * 3.1415 * (index as f32)) / DATA_BUFFER_LEN_TEST as f32;
            *data = ((val.sin() + 1.0) / 2.0 * 256.0) as u8;
        }
    }

    let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
    Bytes::from(encoded)
}
