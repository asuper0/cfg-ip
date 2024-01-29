#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use cfg_ip::utils;
use itertools::Itertools;
use net_adapters::adapter::Address;
use serde_derive::{Deserialize, Serialize};
use slint::{Model, SharedString, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub use cfg_ip::ui::generated_code::*;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
struct MyConfig {
    items: cfg_ip::store::IpConfigList,
}
const CONFIG_FILE: &str = "saved_items.yml";

fn main() {
    let window = Main::new().unwrap();
    let cfg: MyConfig = match confy::load_path(CONFIG_FILE) {
        Ok(cfg) => cfg,
        Err(_) => {
            std::fs::remove_file(CONFIG_FILE)
                .expect("The file saved_items.yml may be broken and failed to delete it");
            confy::load_path(CONFIG_FILE)
                .expect("The file saved_items.yml may be broken and failed to delete it")
        }
    };
    let cfg = Arc::new(Mutex::new(cfg));
    load_saved_items(window.as_weak(), cfg.clone());

    refresh_adapters(&window);
    utils::set_ui_checker(&window);
    utils::set_item_convert(&window);

    set_apply_config(&window);
    set_save_config(&window, cfg.clone());
    set_remove_selected(&window, cfg);

    window.on_refresh_adapters({
        let window = window.as_weak();
        move || {
            let window = window.unwrap();
            refresh_adapters(&window);
        }
    });

    window.run().unwrap();
}

fn set_remove_selected(window: &Main, cfg: Arc<Mutex<MyConfig>>) {
    window.on_remove_selected({
        let weak = window.as_weak();
        move |index| {
            if index < 0 {
                return;
            }
            if {
                let mut cfg = cfg.lock().unwrap();
                let is_removed = cfg.items.remove_at(index).is_some();
                if is_removed {
                    confy::store_path::<MyConfig>(CONFIG_FILE, cfg.clone())
                        .expect("Save to file saved_items.yml failed");
                }
                is_removed
            } {
                load_saved_items(weak.clone(), cfg.clone());
            }
        }
    });
}

fn set_apply_config(window: &Main) {
    window.on_apply_config({
        |_item, _dhcp_on, _texts| {
            let (tx, _rx) = std::sync::mpsc::channel();
            match _dhcp_on {
                true => {
                    if let Err(err) = cfg_ip::set_ip::set_dynamic_ip(&_item.name, tx) {
                        rfd::MessageDialog::new()
                            .set_title("Warning")
                            .set_description(&err.to_string())
                            .show();
                        return;
                    }
                }
                false => {
                    // _texts format： ip,netmask,gateway,dns
                    let texts = _texts
                        .as_any()
                        .downcast_ref::<VecModel<SharedString>>()
                        .unwrap();
                    let infos = match cfg_ip::utils::convert_ip_items(texts) {
                        Ok(items) => items,
                        Err(msg) => {
                            rfd::MessageDialog::new()
                                .set_title("Warning")
                                .set_description(&msg.to_string())
                                .show();
                            return;
                        }
                    };

                    let (ip, netmask, gateway, dns) = infos.into_iter().collect_tuple().unwrap();
                    let address = ip
                        .into_iter()
                        .zip(netmask.into_iter())
                        .map(|(item_ip, item_netmask)| Address {
                            ip: item_ip,
                            netmask: item_netmask,
                        })
                        .collect_vec();
                    if let Err(err) =
                        cfg_ip::set_ip::set_static_ip(&_item.name, &address, &gateway, &dns, tx)
                    {
                        rfd::MessageDialog::new()
                            .set_title("Warning")
                            .set_description(&err.to_string())
                            .show();
                        return;
                    }
                }
            };

            #[allow(unused)]
            while let Ok((line, desc, msg)) = _rx.recv() {
                println!("cmd {desc}, line {line}, msg {msg}");
            }

            rfd::MessageDialog::new()
                .set_title("Info")
                .set_description("Config OK")
                .show();
        }
    });
}

fn set_save_config(window: &Main, cfg: Arc<Mutex<MyConfig>>) {
    window.on_save_config({
        let weak = window.as_weak();
        move |_item, _dhcp_on, _texts| {
            let nic = match _dhcp_on {
                true => net_adapters::adapter::Nic::new(
                    &_item.name,
                    _item.index as u32,
                    &_item.guid,
                    _dhcp_on,
                    None,
                    None,
                    None,
                ),
                false => {
                    // _texts format： ip,netmask,gateway,dns
                    let texts = _texts
                        .as_any()
                        .downcast_ref::<VecModel<SharedString>>()
                        .unwrap();
                    let infos = match cfg_ip::utils::convert_ip_items(texts) {
                        Ok(items) => items,
                        Err(msg) => {
                            rfd::MessageDialog::new()
                                .set_title("Warning")
                                .set_description(&msg.to_string())
                                .show();
                            return;
                        }
                    };

                    let (ip, netmask, gateway, dns) = infos.into_iter().collect_tuple().unwrap();
                    let address = ip
                        .into_iter()
                        .zip(netmask.into_iter())
                        .map(|(item_ip, item_netmask)| Address {
                            ip: item_ip,
                            netmask: item_netmask,
                        })
                        .collect_vec();
                    net_adapters::adapter::Nic::new(
                        &_item.name,
                        _item.index as u32,
                        &_item.guid,
                        _dhcp_on,
                        Some(address),
                        Some(gateway),
                        Some(dns),
                    )
                }
            };
            if let Ok(nic) = nic {
                if {
                    let mut cfg = cfg.lock().unwrap();
                    let is_saved = cfg.items.insert(nic);
                    if is_saved {
                        confy::store_path::<MyConfig>(CONFIG_FILE, cfg.clone())
                            .expect("Save to file saved_items.yml failed");
                    }
                    is_saved
                } {
                    load_saved_items(weak.clone(), cfg.clone());
                }
            }
        }
    });
}

fn refresh_adapters(window: &Main) {
    let adapters = net_adapters::adapter::get_adapters();
    let net_interfaces = adapters
        .iter()
        .map(|item| utils::convert(item))
        .collect_vec();
    let the_model = Rc::new(VecModel::from(net_interfaces));
    let model = slint::ModelRc::from(the_model.clone());
    window
        .global::<NetInterfaceStatus>()
        .set_interface_infos(model);

    let list_items = adapters
        .iter()
        .map(|item| slint::StandardListViewItem::from(item.name()))
        .collect_vec();
    let list_model = utils::create_model_vec(list_items);
    window
        .global::<NetInterfaceStatus>()
        .set_interface_names(list_model);

    if window.get_select_system_adapter() {
        let selected_guid = window.get_selected_guid();
        if let Some((index, _)) = adapters
            .iter()
            .find_position(|item| *item.guid() == *selected_guid.as_str())
        {
            window.invoke_select_system(index as i32);
        }
    }
}

fn load_saved_items(window: slint::Weak<Main>, cfg: Arc<Mutex<MyConfig>>) {
    let saved_items = cfg.lock().unwrap().items.get_list();
    let net_interfaces = saved_items
        .iter()
        .map(|item| utils::convert(item))
        .collect_vec();
    let the_model = Rc::new(VecModel::from(net_interfaces));
    let model = slint::ModelRc::from(the_model.clone());

    let window = window.unwrap();
    window
        .global::<NetInterfaceStatus>()
        .set_saved_settings(model);

    let dhcp_text_fn = |dhcp_on| match dhcp_on {
        true => "dhcp",
        false => "static",
    };

    let list_items = saved_items
        .iter()
        .map(|item| {
            slint::StandardListViewItem::from(
                &format!("{} - {}", item.name(), dhcp_text_fn(item.dhcp_on()))[..],
            )
        })
        .collect_vec();
    let list_model = utils::create_model_vec(list_items);
    window
        .global::<NetInterfaceStatus>()
        .set_saved_names(list_model);
}
