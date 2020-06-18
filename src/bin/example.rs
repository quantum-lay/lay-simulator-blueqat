use lay::Layer;
use lay::Operations;
use lay::gates::CliffordGate;
use lay_simulator_blueqat::{BlueqatSimulator, BlueqatOperations};

#[tokio::main]
async fn main() {
    let m = 10;
    let mut sim = BlueqatSimulator::new().unwrap();
    let mut init_op = BlueqatOperations::new();
    init_op.initialize();
    let fut = tokio::spawn(sim.send(&init_op));
    let mut ops = BlueqatOperations::new();
    for i in 0..m {
        ops.h(i * 2);
        ops.cx(i * 2, i * 2 + 1);
    }
    for i in 0..m * 2 {
        ops.measure(i, ());
    }
    let fut = tokio::spawn(async move {
        fut.await.unwrap();
        sim.send(&ops).await;
        println!("sent!");
        let mut result = String::new();
        sim.receive(&mut result).await;
        println!("{}", result);
        sim });
    let mut ops = BlueqatOperations::new();
    for i in 0..m * 2 {
        ops.x(i);
        ops.x(i);
        ops.x(i);
        ops.measure(i, ());
    }
    let mut sim = fut.await.unwrap();
    let mut result = String::new();
    let mut result2 = String::new();
    sim.send_receive(&ops, &mut result).await;
    println!("{}", result);
    sim.send_receive(&ops, &mut result2).await;
    println!("{}", result2);
}
