use cfg_ip::adapter::Address;
use std::env;
use std::net::IpAddr;

#[test]
fn test_cli() {
    let mut args = env::args();
    args.next();
    let set_dynamic = !match args.next() {
        Some(arg) if &arg == "static" => true,
        _ => false,
    };

    let adapters = cfg_ip::adapter::get_adapters();
    let mut local_nic = None;
    for (_i, nic) in adapters.iter().enumerate() {
        // println!("adapter {} = {:?}", i, nic);
        if nic.name() == "以太网" {
            local_nic = Some(nic);
        }
    }

    println!("finish, {} total", adapters.len());

    let nic = local_nic.expect("没有找到适配器");

    let (tx, rx) = std::sync::mpsc::channel();
    if set_dynamic {
        println!("set dynamic ip on adapter {}", nic.name());
        match cfg_ip::set_ip::set_dynamic_ip(nic, tx) {
            Ok(()) => println!("set ok"),
            Err(err) => println!("failed: {err}"),
        };
    } else {
        println!("set static ip on adapter {}", nic.name());
        let address = vec![
            Address {
                ip: "192.168.3.55".parse().unwrap(),
                netmask: "255.255.255.0".parse().unwrap(),
            },
            Address {
                ip: "192.168.4.55".parse().unwrap(),
                netmask: "255.255.255.0".parse().unwrap(),
            },
        ];
        let gateway: Vec<IpAddr> = vec![
            "192.168.3.1".parse().unwrap(),
            "192.168.4.1".parse().unwrap(),
        ];
        let dns: Vec<IpAddr> = vec![
            "192.168.1.1".parse().unwrap(),
            // "58.20.127.238".parse().unwrap(),
            "222.246.129.81".parse().unwrap(),
        ];

        match cfg_ip::set_ip::set_static_ip(nic, &address, &gateway, &dns, tx) {
            Ok(()) => println!("set ok"),
            Err(err) => println!("failed: {err}"),
        };
    }

    while let Ok((line,desc, msg)) = rx.recv() {
        println!("cmd {desc}, line {line}, msg {msg}");
    }

    println!("exit");

    let mut for_exit = String::new();
    let _ = std::io::stdin().read_line(&mut for_exit);
}
