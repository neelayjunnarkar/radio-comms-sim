use radio_comms as radio;

fn main() {
    radio::start().expect("failed to start radio comms");
    //radio::transmit("hello".to_owned(), 1).expect("failed to transmit");
    println!("completed transmission");
    loop {}
}
