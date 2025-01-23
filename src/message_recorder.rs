use clap::{Arg, ArgAction, Command};
use zenoh::config::Config;
use zenoh::prelude::sync::SyncResolve;
use zenoh::prelude::SplitBuffer;

use imu_message::IMUMessage;

fn main() {
    // initiate logging
    env_logger::init();
    let (config, key_exprs) = parse_args();
    let key_expr = key_exprs.get(0).unwrap();

    println!("Openning session...");
    let session = zenoh::open(config).res().unwrap();
    let sub = session.declare_subscriber(key_expr).res().unwrap();

    while let Ok(sample) = sub.recv() {
        let payload = sample.value.payload.contiguous(); // バイト列として取得
        println!("Timestamp: {:?}", sample.timestamp);
        if let Ok(decoded_data) = bincode::deserialize::<IMUMessage>(&payload) {
            println!("Received data: {:?}", decoded_data);
        } else {
            println!("Failed to decode data");
        }
    }
    sub.undeclare().res().unwrap();
    session.close().res().unwrap();
}

fn parse_args() -> (Config, Vec<String>) {
    let matches = Command::new("zenoh video display example")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("The zenoh session mode.")
                .value_parser(clap::builder::PossibleValuesParser::new(["peer", "client"]))
                .default_value("peer"),
        )
        .arg(
            Arg::new("key")
                .short('k')
                .long("key")
                .action(ArgAction::Append)
                .value_name("KEY_EXPR")
                .help("The key expressions to subscribe to.")
                .default_value("demo/imu"),
        )
        .arg(
            Arg::new("peer")
                .short('e')
                .long("peer")
                .action(ArgAction::Append)
                .value_name("LOCATOR")
                .help("Peer locators used to initiate the zenoh session."),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("A configuration file."),
        )
        .get_matches();

    // キー式を取得
    let key_exprs: Vec<String> = matches
        .get_many::<String>("key")
        .unwrap()
        .cloned()
        .collect();

    // Config オブジェクトの構築
    let mut config = if let Some(conf_file) = matches.get_one::<String>("config") {
        Config::from_file(conf_file).unwrap()
    } else {
        Config::default()
    };

    // モードの設定
    if let Some(mode) = matches.get_one::<String>("mode") {
        config.set_mode(Some(mode.parse().unwrap())).unwrap();
    }

    // ピア情報の設定
    if let Some(peers) = matches.get_many::<String>("peer") {
        config
            .connect
            .endpoints
            .extend(peers.map(|p| p.parse().unwrap()));
    }

    (config, key_exprs)
}
