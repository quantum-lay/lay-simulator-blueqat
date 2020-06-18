use lay::Layer;
use lay::Operations;
use lay::gates::CliffordGate;
use lay_simulator_blueqat::{BlueqatSimulator, BlueqatOperations};

#[tokio::main]
async fn main() {
    let m = 10;
    let mut sim = BlueqatSimulator::new().unwrap();
    let mut ops = BlueqatOperations::new();
    ops.initialize();
    for i in 0..m {
        ops.h(i * 2);
        ops.cx(i * 2, i * 2 + 1);
    }
    for i in 0..m * 2 {
        ops.measure(i, ());
    }
    let sim = tokio::spawn(async {
        let sim = sim.send(ops).await;
        println!("Sent!");
        let (sim, result) = sim.receive().await;
        println!("{}", result);
        sim });
    let mut ops = BlueqatOperations::new();
    for i in 0..m * 2 {
        ops.x(i);
        ops.x(i);
        ops.x(i);
        ops.measure(i, ());
    }
    let sim = sim.await.unwrap().send(ops).await;
    println!("Sent!");
    let (sim, result) = sim.receive().await;
    println!("{}", result);
    let mut ops = BlueqatOperations::new();
    for i in 0..m * 2 {
        ops.x(i);
        ops.x(i);
        ops.x(i);
        ops.measure(i, ());
    }
    let sim = sim.send(ops).await;
    println!("Sent!");
    println!("{}", sim.receive().await.1);
}
