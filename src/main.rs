use itertools::Itertools;
// use cfg_ip::adapter::Address;
// use std::env;
// use std::net::IpAddr;
// use slint::Model;
// use slint::SharedString;
use slint::VecModel;
use std::rc::Rc;
use cfg_ip::utils;

pub use cfg_ip::ui::generated_code::*;

fn main(){
    let adapters = net_adapters::adapter::get_adapters();
    let window = Main::new().unwrap();

    let net_interfaces = adapters.iter().map(|item| utils::convert(item)).collect_vec();
    let the_model = Rc::new(VecModel::from(net_interfaces));
    let model = slint::ModelRc::from(the_model.clone());
    window.global::<NetInterfaceStatus>().set_all_interfaces(model);

    let list_items = adapters.iter().map(|item| slint::StandardListViewItem::from(item.name())).collect_vec();
    let list_model = utils::create_model_vec(list_items);
    window.global::<NetInterfaceStatus>().set_saved_settings(list_model);

    utils::set_ui_checker(&window);

    // let _aa = window.global::<NetInterfaceStatus>().get_all_interfaces();
    window.run().unwrap();

}
