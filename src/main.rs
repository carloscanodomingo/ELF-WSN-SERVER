#![allow(unused_imports)]
#![allow(unused_mut)]
use crate::comm::influx_state::*;
use chrono::prelude::*;
use comm::devices::traits::{DataBuffer, Devices};
use comm::devices::CC32xx::*;
use comm::influx_error::InfluxErrorCode;
use comm::influx_info::*;
use comm::influxdb::{InfluxDbClient, InfluxDbContext, InfluxDbId, InfluxDbPrecision};
use comm::{config::*, message::*};
use core::convert::TryInto;
use fmt::format;
use std::sync::{Arc, Mutex};
use tokio::{sync::mpsc, task, time};
mod comm;
#[macro_use]
extern crate num_derive;

use rumqttc::{self, AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime};

static CC32XX: &str = "CC32xx";
static ESP32: &str = "ESP32";

const DEVICES: [&'static str; 2] = ["CC32xx", "ESP32"];
const CHANNELS: [&'static str; 2] = ["data", "start"];
static DATA_TOPIC_SUFFIX: &str = "/data";
static CMD_TOPIC_SUFFIX: &str = "/cmd";
static TOPIC_DATA: &str = "data";
static PATH_CONFIG: &str = "PATH_CONFIG_INFLUXDB_SERVER";
static TOPIC_RACH: &str = "cmd";
const MAX_DATA_NUM: u32 = 2;
#[derive(Debug)]
struct MqttSender {
    device_topic: String,
    device_id: InfluxDbId,
    msg_cmd: MsgCmdType,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    mqtt_main().await
}
//#[tokio::main(worker_threads = 2)]
async fn mqtt_main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    color_backtrace::install();

    let path_config = env::var(PATH_CONFIG);
    if let Err(_error) = path_config {
        println!(
            "Configuration file path has to be set in the environmental variable {}",
            PATH_CONFIG
        );
        return Err(Box::new(InfluxErrorCode::ConfigPathDoesntExits));
    }
    let path_config = path_config.unwrap();

    let mut mqttoptions = MqttOptions::new("test-1", "localhost", 1883);
    mqttoptions.set_keep_alive(5);
    mqttoptions.set_max_packet_size(20 * 1024, 50 * 1024);
    mqttoptions.set_clean_session(false);
    let server_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let (tx, rx) = mpsc::channel::<MqttSender>(100);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    task::spawn(async move {
        requests(&client, rx).await;
        time::sleep(Duration::from_secs(3)).await;
    });
    let device_cc32xx_2 = CC32xxDevice::new(path_config.clone());
    let config_file = ConfigFile::load_config(path_config);
    if let Err(error) = config_file {
        return Err(Box::new(error));
    }
    let config_file = config_file.unwrap();
    println!("The token is: {}", config_file.token);

    let arc_db_client = Arc::new(Mutex::new(InfluxDbClient::new(
        config_file.url,
        config_file.port,
        config_file.org_id,
        config_file.bucket,
        config_file.token,
    )));
    let mac_test: CC32xxUniqueIdType = CC32xxUniqueIdType {
        id: [1, 2, 3, 4, 5, 6],
    };

    device_cc32xx_2.add_unique_id(mac_test, server_time);
    loop {
        let mut msg_response: Option<bytes::Bytes> = None;
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                let mut data = p.payload;
                let header = comm::message::decode_msg(&mut data);
                if let Err(x) = header {
                    println!("Msg header error - {:?}", x);
                    continue;
                };
                let header = header.unwrap();
                let mut topic_iter = p.topic.split("/");
                match topic_iter.next() {
                    Some(str_topic_device) if DEVICES.contains(&str_topic_device) => {
                        match topic_iter.next() {
                            // If the topic is for command
                            Some(str_op) if str_op == TOPIC_RACH => {
                                if str_topic_device == CC32xxDevice::get_device_topic() {
                                    let devices_sender = &device_cc32xx_2;
                                    let rx_id = InfluxDbId {
                                        id: header.device_id as u32,
                                    };
                                    if header.msg_type == (MsgType::MqttCmd as u8) {
                                        let cmd_decode = comm::message::decode_msg_cmd(&mut data);
                                        if let Err(error) = cmd_decode {
                                            println!("decode msg cmd Error {:?}", error);
                                            continue;
                                        }
                                        let cmd_type = cmd_decode.unwrap();

                                        match cmd_type {
                                            // Device sent ready message -> Add or find unique ID
                                            // and return local id to device.
                                            MsgCmdType::Ready(cmd_ready) => {
                                                let res_id = devices_sender.add_unique_id(
                                                    CC32xxUniqueIdType { id: cmd_ready.mac },
                                                    server_time,
                                                );
                                                // If an error occurs -> Register in DDBB.

                                                if let Err(error) = res_id {
                                                    let mut dst: CC32xxUniqueIdType =
                                                        CC32xxUniqueIdType { id: cmd_ready.mac };

                                                    println!("Problem after here");
                                                    let info_error = InfoError {
                                                        error,
                                                        error_info: dst.get_id_string(),
                                                    };
                                                    let db_client = arc_db_client.clone();
                                                    let res = db_client
                                                        .lock()
                                                        .unwrap()
                                                        //ToDo Fix name
                                                        .log_info(
                                                            dst.get_id_string(),
                                                            rx_id,
                                                            info_error,
                                                        )
                                                        .await;

                                                    println!("Log info Response {:?}", res);
                                                    continue;
                                                }
                                                // If correct, unwrap id and send it back
                                                let id = res_id.unwrap();
                                                let tx_res = tx
                                                    .send(MqttSender {
                                                        device_id: InfluxDbId { id: 0 },
                                                        device_topic:
                                                            CC32xxDevice::get_device_topic(),
                                                        msg_cmd: MsgCmdType::RespId(CmdRespId {
                                                            mac: cmd_ready.mac,
                                                            device_id: id.id as u16,
                                                        }),
                                                    })
                                                    .await;
                                                if let Err(error) = tx_res {
                                                    println!(
                                                        "Problem in channel: Send fail - {:?}",
                                                        error
                                                    );
                                                }
                                            }
                                            MsgCmdType::Rssi(cmd_rssi) => {
                                                let db_client = arc_db_client.clone();
                                                let device_name_res =
                                                    devices_sender.get_name(rx_id);
                                                if let Err(error) = device_name_res {
                                                    println!("Error: {:?}", error);
                                                    continue;
                                                }
                                                let device_name = device_name_res.unwrap();

                                                let info_rssi = InfoRssi::from_cmd(cmd_rssi);
                                                let rssi_to_print = info_rssi.data_rssi.clone();
                                                println!("Enter in Rssi");
                                                let res = db_client
                                                    .lock()
                                                    .unwrap()
                                                    .log_info(device_name, rx_id, info_rssi)
                                                    .await;

                                                println!(
                                                    "Received Rssi: {} - Response HTTP: {:?}",
                                                    rssi_to_print, res
                                                );
                                            }

                                            // Depreated in recent version this functionality is
                                            // done in every data packet.
                                            MsgCmdType::SyncChrono(cmd_sync) => {
                                                let db_client = arc_db_client.clone();
                                                let sync_res = devices_sender.sync_device(
                                                    rx_id,
                                                    TimeStampCount {
                                                        count: cmd_sync.buffer_count,
                                                        timestamp: cmd_sync.timestamp_ns as u64,
                                                    },
                                                    server_time,
                                                );
                                                if let Err(error) = sync_res {
                                                    println!("Error: {:?}", error);
                                                    continue;
                                                }
                                                println!(
                                                    "Received TimeSync: count: {} timestamp_ns: {} - Response HTTP: {:?}",
                                                    cmd_sync.buffer_count, cmd_sync.timestamp_ns, sync_res
                                                );
                                            }

                                            _ => {
                                                println!("NOT IMPLEMENTED in MAIN");
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            // Check if the TOPIC correspond with the topic for receiving data
                            Some(str_op) if str_op == TOPIC_DATA => match topic_iter.next() {
                                // Check if it is possible to convert the id to a num
                                Some(num) if num.parse::<u8>().is_ok() => match str_topic_device {
                                    // Check if the topic str device is CC32xx
                                    str_device
                                        if str_device == CC32xxDevice::get_device_topic() =>
                                    {
                                        // Convert ID from the topic
                                        let id = InfluxDbId {
                                            id: num.parse::<u8>().unwrap() as u32,
                                        };
                                        // Use CC32xx as device sender struct
                                        let devices_sender = &device_cc32xx_2;
                                        //  Decode the header of the message data
                                        let res_header_data =
                                            comm::message::decode_msg_data(&mut data);
                                        // Check if the header failed
                                        if let Err(error) = res_header_data {
                                            println!("Header data Error:  {:?}", error);
                                            continue;
                                        }
                                        let header = res_header_data.unwrap();

                                        // Get buffer size
                                        let mut buffer_size = header.buffer_size;
                                        // Get total len of the data
                                        let data_len =
                                            header.n_buffers as usize * header.buffer_size as usize;
                                        // Count new packet
                                        let res = devices_sender.add_data_packet(
                                            id,
                                            data_len,
                                            header.data_id as usize,
                                        );
                                        // Register if there is an error adding a packet
                                        if let Err(error) = res {
                                            println!("{:?}", error);
                                            let temp_db_client = arc_db_client.clone();
                                            let db_client = temp_db_client.lock().unwrap();
                                            let info_error = InfoError {
                                                error,
                                                error_info: format!("{}", header.data_id),
                                            };

                                            let res = db_client
                                                //ToDO change server
                                                .log_info("Server".to_string(), id, info_error)
                                                .await;
                                            if let Err(log_error) = res {
                                                println!("Failed to log error: {:?}", log_error);
                                            }
                                        }
                                        // Decode packet of data
                                        let rest_buffer = comm::message::decode_data::<
                                            CC32xxDataBuffer,
                                        >(
                                            &mut data, buffer_size as usize
                                        );
                                        match rest_buffer {
                                            Ok(buffers) => {
                                                // Check if we lost any data packet.
                                                let temp_db_client = arc_db_client.clone();
                                                let db_client = temp_db_client.lock().unwrap();

                                                async {
                                                    let res = db_client
                                                        .write_data(
                                                            id,
                                                            devices_sender,
                                                            header.timestamp,
                                                            buffers,
                                                        )
                                                        .await;

                                                    let local: DateTime<Local> = Local::now();
                                                    println!(
                                                        "{} - {}/{} - {:?}",
                                                        local, str_topic_device, id, res
                                                    );
                                                }
                                                .await
                                            }
                                            Err(x) => {
                                                println!("Buffer {:?}", x);
                                                continue;
                                            }
                                        }
                                    }
                                    str_device if str_device == ESP32 => {}
                                    _ => println!("ESP32 Not developped"),
                                },
                                Some(_) => println!("Not a valid num"),
                                None => println!("Is needed a ID for data topic"),
                            },
                            Some(x) => println!("Not valid topic DEVICE/{}/...", x),
                            None => println!("Not valid struct of topic"),
                        }
                    }
                    Some(x) => {
                        println!("Not valid topic {}", x)
                    }
                    None => println!("Not / found in Topic"),
                }
            }
            Ok(Event::Incoming(_)) => {
                //println!("Incoming = {:?}", i);
            }
            Ok(Event::Outgoing(_)) => {} //println!("Outgoing = {:?}", o),
            Err(e) => {
                println!("Connection Error = {:?}", e);
                std::thread::sleep(time::Duration::from_millis(3000));
                continue;
            }
        }
    }
}

async fn requests(client: &AsyncClient, mut rx: mpsc::Receiver<MqttSender>) {
    client
        .subscribe("CC32xx/cmd/d2s/255", QoS::AtLeastOnce)
        .await
        .unwrap();
    client
        .subscribe("CC32xx/cmd/255", QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish(
            "CC32xx/cmd/90",
            QoS::ExactlyOnce,
            false,
            "Start Sending Packet",
        )
        .await
        .unwrap();

    for device_str in DEVICES.iter() {
        for x in 1..10 {
            println!("TOPIC: {}/{}/{}", device_str, TOPIC_DATA, x);
            client
                .subscribe(
                    format!("{}/{}/{}", device_str, TOPIC_DATA, x),
                    QoS::AtLeastOnce,
                )
                .await
                .unwrap();
        }
    }

    while let Some(msg_cmd) = rx.recv().await {
        let _topic = format!("{}/cmd/d2s/{}", msg_cmd.device_topic, msg_cmd.device_id);
        match &msg_cmd.msg_cmd {
            MsgCmdType::RespId(msg) => {
                client
                    .subscribe(
                        format!("{}/cmd/d2s/{}", msg_cmd.device_topic, msg.device_id),
                        QoS::AtLeastOnce,
                    )
                    .await
                    .unwrap();
            }
            _ => {}
        }
        let bytes_resp = comm::message::encode_msg_cmd(msg_cmd.msg_cmd);
        match bytes_resp {
            Ok(resp) => {
                if let Err(error) = client
                    .publish(
                        format!("CC32xx/cmd/s2d/{}", msg_cmd.device_id),
                        QoS::ExactlyOnce,
                        false,
                        &resp[..],
                    )
                    .await
                {
                    println!("Error Sending in Start - error: {}", error);
                }
            }
            Err(error) => {
                println!("Error {:?}", error);
            }
        }
    }
}
#[test]
fn bytes_test() {
    use serde::{Deserialize, Serialize};
    use std::fs::File;

    #[derive(Serialize, Deserialize)]
    struct DataTestTest {
        id: u32,
        len: u32,
        count: u32,
        sync: u16,
    }
    struct Packet {
        header: DataTestTest,
        payload: Vec<u8>,
    }
    let a = DataTestTest {
        id: 2,
        len: 30,
        count: 50,
        sync: 33,
    };
    let pkt = Packet {
        header: DataTestTest {
            id: 12222,
            len: 10,
            count: 9888,
            sync: 333,
        },
        payload: vec![1, 1, 1, 1],
    };
    let mut f = File::create("/tmp/output.bin").unwrap();
    bincode::serialize_into(&mut f, &a).unwrap();

    let bytes = bincode::serialize(&a).unwrap();
    println!("{:?}", bytes);
    let data: DataTestTest = bincode::deserialize(&bytes[..]).unwrap();
    println!("{:?}", data.sync);
}
/*
#[test]
fn hash_map_basic() {
    let devices = Arc::new(Mutex::new(HashMap::<u32, DataInfo>::new()));

    let num = 0;
    let mutex = Arc::clone(&devices);
    let mut array = mutex.lock().unwrap();

    println!("we are still learning");
    let ret = array.get(&num);
    assert_eq!(ret.is_none(), true);
    let create_and_add = || -> DataInfo {
        let mut x = DataInfo::new();
        x.add_data_packet(100, 9);
        x
    };
    array.insert(num, create_and_add());
    assert_eq!(array.get(&num).unwrap().total_bytes, 100);
    drop(array);
}

#[test]
fn hash_map_entry() {
    let num = 0;

    let devices = Arc::new(Mutex::new(HashMap::<u32, DataInfo>::new()));

    let mutex = Arc::clone(&devices);
    let mut array = mutex.lock().unwrap();

    let current_count_data = array.entry(num).or_insert(DataInfo::new());

    assert_eq!(current_count_data.total_bytes, 0);
    assert_eq!(current_count_data.total_packet, 0);

    current_count_data.add_data_packet(100, 5);

    assert_eq!(current_count_data.total_bytes, 100);

    drop(array);
    let mutex = Arc::clone(&devices);
    let mut array = mutex.lock().unwrap();

    let num = 1;

    let current_count_data = array.entry(num).or_insert(DataInfo::new());
    current_count_data.add_data_packet(200, 5);
    assert_eq!(current_count_data.total_bytes, 200);
    drop(array);

    let mutex = Arc::clone(&devices);
    let mut array = mutex.lock().unwrap();

    let num = 0;

    let current_count_data = array.entry(num).or_insert(DataInfo::new());
    current_count_data.add_data_packet(200, 6);
    assert_eq!(current_count_data.total_bytes, 300);
    drop(array);
}

#[test]
fn array_data_count_test() {
    let sys_time_start = SystemTime::now();
    let count_data_array = Arc::new(Mutex::new(vec![
        DataInfo {
            total_bytes: 0,
            total_packet: 0,
            time_start: sys_time_start.clone(),
            last_msg_count: None,
            packet_lost: 0,
        };
        MAX_DATA_NUM as usize
    ]));

    let mutex = Arc::clone(&count_data_array);
    let array = mutex.lock().unwrap();

    println!("we are still learning");
    assert_eq!(array[0].total_packet, 0);
    drop(array);
    let mutex = Arc::clone(&count_data_array);
    let array = mutex.lock().unwrap();

    println!("we are still learning");
    assert_eq!(array[1].total_packet, 0);
    drop(array);

    let mutex = Arc::clone(&count_data_array);
    let mut array = mutex.lock().unwrap();

    array[0].add_data_packet(100, 5);
    assert_eq!(array[0].total_packet, 1);
    assert_eq!(array[0].total_bytes, 100);

    drop(array);

    let mutex = Arc::clone(&count_data_array);
    let array = mutex.lock().unwrap();

    assert_eq!(array[1].total_packet, 0);
}

#[test]
fn test_data() {
    use reqwest::Client;
    use std::time::SystemTime;
    let data_code = comm::message::create_data_code();
    // Device sent
    let (header, msg_container) = comm::message::decode::<CC32xxDataBuffer>(data_code);
    match msg_container {
        comm::message::MsgContainer::MsgDataContainer(count_id, data_buffers) => {
            println!("{} NO PROBLEM", count_id);

            let start_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 300;

            let mut http_body = String::new();
            //let buffer = &data_buffers.first().unwrap().payload;
            let len_data_buffer = data_buffers.len() as u64;
            for (index_buffer, data_buffer) in data_buffers.iter().enumerate() {
                for (index, value) in data_buffer.buffer.iter().enumerate() {
                    http_body.push_str(
                        &(format!(
                            "rust_test,host=second_device print_seno={:.5} {}\n",
                            value,
                            start_time + index_buffer as u64 * len_data_buffer + index as u64
                        )
                        .to_owned()),
                    );
                }
            }
            println!("{}", http_body);
            let client = Client::new();
            let req = client
                // or use .post, etc.
                .post("http://localhost:8086/api/v2/write?orgID=972309793c223d17&bucket=Test&precision=s")
                .header("Authorization", "Token -EC-3F-ANf5z7D3yiJZPiygWG1GTwxfVkC0FHb2al4vJIrPAGn7lEUJL5ZA5X7QUJMyF2jfEOP9zFifbKh3tWA==")
                .body(http_body);
            //.body("mem,host=test5 used_percent=23.43234543 1617411484");

            tokio_test::block_on(async {
                let res = req.send().await.unwrap();

                println!("{}", res.status());
            });
        }

        _ => println!("problem with msg"),
    }
    println!("{}", header);
}
#[test]
fn test_data_trait() {
    /*
    use std::time::SystemTime;
    let data_code = comm::message::create_data_code();
    // Device sent
    let (header, msg_container) = comm::message::decode::<CC32xxDataBuffer>(data_code);
    let arc_db_client = Arc::new(Mutex::new(InfluxDbClient::new(
        "http://localhost".to_string(),
        "8086".to_string(),
        "972309793c223d17".to_string(),
        "Test".to_string(),
        "ZvqgmZ8WXY_pFjAc9NhU3qwubX02pVAAn-Gj_MAoqBDrFdw41AhWXQR_G9HuGbodWt6Aq_MPghl0QZGKNcIeVg=="
            .to_string(),
    )));
    let db_context = InfluxDbContext::new(
        "test_3".to_string(),
        "host".to_string(),
        "third_device".to_string(),
        "print_data".to_string(),
        1.0 / 10.0,
        InfluxDbPrecision::MicroSeconds,
    );

    match msg_container {
        comm::message::MsgContainer::MsgDataContainer(count_id, data_buffers) => {
            println!("{} NO PROBLEM", count_id);
            //let buffer = &data_buffers.first().unwrap().payload;
            let _len_data_buffer = data_buffers.len() as u64;
            for (_index_buffer, data_buffer) in data_buffers.into_iter().enumerate() {
                let temp_db_client = arc_db_client.clone();
                let db_client = temp_db_client.lock().unwrap();

                tokio_test::block_on(async {
                    let res = db_client.write_data(&db_context, data_buffer).await;
                    match res {
                        Some(x) => println!("{}", x),
                        _ => {}
                    }
                })
            }
        }

        _ => println!("problem with msg"),
    }
    println!("{}", header);
    */
}
*/
#[test]
fn test_json() {
    use serde_json::{json, Map, Value};

    use serde::{Deserialize, Serialize};
    use serde_json::Result;
    use std::{error::Error, fs};

    #[derive(Serialize, Deserialize)]
    struct config_file {
        devices: HashMap<String, String>,
    }
    let path = "config_server.json";
    match fs::read_to_string(path) {
        Ok(valid_fs) => {
            let config = valid_fs.to_string();
            println!("string read {}", config);
            let parsed: config_file = serde_json::from_str(&config).unwrap();
            //let obj: config_file = parsed.as_object().unwrap().clone();

            for (key, value) in parsed.devices.iter() {
                println!("key: \"{}\" Value {}", key, value);
            }
        }
        Err(error) => {
            println!("Error to Read");
        }
    }
    let john = json!({
        "name": "John Doe",
        "age": 43,
        "phones": [
            "+44 1234567",
            "+44 2345678"
        ]
    });

    println!("first phone number: {}", john["phones"][0]);

    // Convert to a string of JSON and print it out
    println!("{}", john.to_string());
}
#[test]
fn test_pattern_topic() {
    let str_topic = "esp32/data/cmd/32".to_owned();
    let mut pattern = str_topic.split("/");
    match pattern.next().unwrap() {
        "esp32" => match pattern.next() {
            Some("data") => match pattern.next() {
                Some("cmd") => match pattern.next() {
                    Some(num) if num.parse::<u8>().is_ok() => {
                        println!("El numero del topic es:{}", num);
                    }
                    Some(_) => println!("Not a valid num"),
                    None => panic!(),
                },
                Some(_) => {
                    panic!()
                }
                None => panic!(),
            },
            Some(_) => panic!(),
            None => panic!(),
        },
        _ => panic!(),
    }
}
