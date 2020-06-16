use lay::Layer;
use lay::gates::CliffordGate;
use lay_simulator_blueqat::BlueqatSimulator;

fn main() {
    let mut sim = BlueqatSimulator::new().unwrap();
    sim.initialize();
    for i in 0..10 {
        sim.h(i * 2);
        sim.cx(i * 2, i * 2 + 1);
    }
    for i in 0..20 {
        sim.measure(i, ());
    }
    sim.send();
    println!("{}", sim.receive());
    for i in 0..20 {
        sim.measure(i, ());
    }
    sim.send();
    println!("{}", sim.receive());
    for i in 0..20 {
        sim.measure(i, ());
    }
    sim.send();
    println!("{}", sim.receive());
}
