#![allow(non_local_definitions)]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use clap::{arg, command, ArgAction};
use imu_message::IMUMessage;
use zenoh::config::Config;
use zenoh::prelude::sync::*;

use hdf5::types::VarLenArray;
use hdf5::{File, H5Type};

#[derive(Clone)] // register with HDF5
struct RecordVec {
    timestamp_micro: u64, // タイムスタンプ
    data: Vec<u8>,        // 可変長バイナリデータ
}

#[derive(H5Type, Clone, PartialEq, Debug)] // register with HDF5
#[repr(C)]
struct Record {
    timestamp_micro: u64,  // タイムスタンプ
    data: VarLenArray<u8>, // 可変長バイナリデータ
}

impl Record {
    fn from(record: RecordVec) -> Self {
        Record {
            timestamp_micro: record.timestamp_micro,
            data: VarLenArray::from(record.data.as_slice()),
        }
    }
}
fn main() {
    let matches = command!() // requires `cargo` feature
        .arg(arg!(-k --key_expr "lists key_expr").action(ArgAction::Append))
        .get_matches();

    let key_exprs: Vec<String> = matches
        .get_many::<String>("key_expr")
        .unwrap_or_default()
        .cloned() // 文字列の所有権を取得
        .collect();

    // initiate logging
    env_logger::init();

    let config = Config::default();
    let session = zenoh::open(config).res().unwrap().into_arc();

    let file = File::create("timestamped_data.h5").unwrap();

    let data = Arc::new(Mutex::new(HashMap::new()));

    let start = Instant::now();
    let mut tasks = vec![];
    for key_expr in key_exprs.iter().cloned() {
        let data_clone = data.clone();
        let mut map = data_clone.lock().unwrap();
        map.insert(key_expr.clone(), Vec::new());
        drop(map);

        let sub = session
            .declare_subscriber(key_expr.clone())
            .callback(move |sample| {
                let payload = sample.value.payload.contiguous(); // バイト列として取得
                println!("key_expr: {}", sample.key_expr);

                if let Ok(decoded_data) = bincode::deserialize::<IMUMessage>(&payload) {
                    println!("Received data: {:?}", decoded_data);
                } else {
                    println!("Failed to decode data");
                }
                let elapsed = start.elapsed().as_micros();

                let a = RecordVec {
                    timestamp_micro: elapsed as u64,
                    data: payload.to_vec(),
                };
                let mut map = data_clone.lock().unwrap();
                map.get_mut(&key_expr).unwrap().push(a);
            })
            .res()
            .unwrap();
        tasks.push(sub);
    }

    std::thread::sleep(std::time::Duration::from_millis(2000));

    for key_expr in key_exprs.iter().cloned() {
        let map = data.lock().unwrap();
        let x = map.get(&key_expr).unwrap().clone();
        drop(map);
        let x: Vec<Record> = x.into_iter().map(|x| Record::from(x)).collect();
        let dataset = file
            .new_dataset::<Record>()
            .shape((x.len(),))
            .create(key_expr.as_str())
            .unwrap();
        dataset.write(&x).unwrap();
    }
}
