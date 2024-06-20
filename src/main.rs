use normalize_sic::{inet::INet, pnet::PNet};

fn main() {
    let inet = INet::from_str("(a b)\n(a b)").unwrap();
    println!("net {inet:?}");
    let free_ports = inet.free_ports.len();
    println!("free ports: {free_ports:?}");
    let text = inet.to_string().unwrap();
    println!("text: {text:?}");
    let pnet = PNet::<1>::from_inet(&inet).unwrap();
    println!("pnet: {pnet:#?}");
}
