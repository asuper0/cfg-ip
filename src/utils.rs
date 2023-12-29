use crate::ui::generated_code::{
    InterfaceItemCheck, IpV4, Main, NetAddress, NetInterfaceItem, NetItemUtils,
};
use itertools::{self, Itertools};
use net_adapters::adapter::{Address, Nic};
// use slint::SharedString;
use slint::{ComponentHandle, ModelRc, VecModel, Model};
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Rc;
use std::str::FromStr;

pub fn convert_address(value: &Address) -> NetAddress {
    NetAddress {
        ip: convert_ip(&value.ip),
        netmask: convert_ip(&value.netmask),
    }
}

pub fn convert_ip(value: &IpAddr) -> IpV4 {
    IpV4 {
        ip: value.to_string().into(),
    }
}

pub fn create_model_vec<T>(items: Vec<T>) -> ModelRc<T>
where
    T: 'static + Clone,
{
    // let the_model = Rc::new(VecModel::from(Vec::<T>::with_capacity(capacity)));
    let the_model = Rc::new(VecModel::from(items));
    let model = slint::ModelRc::from(the_model);
    model
}

pub fn convert(nic: &Nic) -> NetInterfaceItem {
    NetInterfaceItem {
        address: create_model_vec(
            nic.address()
                .iter()
                .map(|item| convert_address(item))
                .collect_vec(),
        ),
        dns: create_model_vec(nic.dns().iter().map(|item| convert_ip(item)).collect_vec()),
        gateway: create_model_vec(
            nic.gateway()
                .iter()
                .map(|item| convert_ip(item))
                .collect_vec(),
        ),
        guid: nic.guid().into(),
        index: nic.index().unwrap() as i32,
        is_up: nic.is_up(),
        name: nic.name().into(),
        dhcp_server: match nic.dhcp_server() {
            Some(s) => convert_ip(s),
            None => IpV4::default(),
        },
    }
}

pub fn set_ui_checker(window: &Main) {
    window.global::<InterfaceItemCheck>().on_check_address({
        move |_net_address| {
            Ipv4Addr::from_str(&_net_address.ip.ip.as_str()).is_ok()
                && Ipv4Addr::from_str(&_net_address.netmask.ip.as_str()).is_ok()
        }
    });

    window
        .global::<InterfaceItemCheck>()
        .on_check_ip(move |ip| Ipv4Addr::from_str(ip.ip.as_str()).is_ok());
}

pub fn set_item_convert(window: &Main) {
    window.global::<NetItemUtils>().on_get_ip_list({
        move |net_address| {
            let combined  = net_address.iter().map(|item| item.ip.ip.to_string()).join("\n");
            combined.into()
        }
    });

    window.global::<NetItemUtils>().on_get_netmask_list({
        move |net_address| {
            let combined  = net_address.iter().map(|item| item.netmask.ip.to_string()).join("\n");
            combined.into()
        }
    });

    window.global::<NetItemUtils>().on_get_gateway_list({
        move |net_address| {
            let combined  = net_address.iter().map(|item| item.ip.to_string()).join("\n");
            combined.into()
        }
    });

    window.global::<NetItemUtils>().on_get_dns_list({
        move |net_address| {
            let combined  = net_address.iter().map(|item| item.ip.to_string()).join("\n");
            combined.into()
        }
    });

    // window
    //     .global::<NetItemUtils>()
    //     .on_check_ip(move |ip| Ipv4Addr::from_str(ip.ip.as_str()).is_ok());
}
