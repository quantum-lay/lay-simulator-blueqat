use lay::Layer;
use lay::gates::CliffordGate;
use lay_simulator_blueqat::BlueqatSimulator;

fn main() {
    let m = 10;
    let mut sim = BlueqatSimulator::new().unwrap();
    sim.initialize();
    for i in 0..m {
        sim.h(i * 2);
        sim.cx(i * 2, i * 2 + 1);
    }
    for i in 0..m * 2 {
        sim.measure(i, ());
    }
    sim.send();
    println!("Sent!");
    println!("{}", sim.receive());
    for i in 0..m * 2 {
        sim.x(i);
        sim.x(i);
        sim.x(i);
        sim.measure(i, ());
    }
    sim.send();
    println!("Sent!");
    println!("{}", sim.receive());
    for i in 0..m * 2 {
        sim.x(i);
        sim.x(i);
        sim.x(i);
        sim.measure(i, ());
    }
    sim.send();
    println!("Sent!");
    println!("{}", sim.receive());
}
