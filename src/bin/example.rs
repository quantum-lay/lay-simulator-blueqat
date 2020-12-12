use lay::Layer;
use lay::gates::*;
use lay_simulator_blueqat::BlueqatSimulator;

#[tokio::main]
async fn main() {
    let m = 10;
    let mut sim = BlueqatSimulator::new().unwrap();
    sim.initialize();
    let fut = tokio::spawn(sim.send(&init_op));
    for i in 0..m {
        sim.h(i * 2);
        sim.cx(i * 2, i * 2 + 1);
    }
    for i in 0..m * 2 {
        sim.measure(i, ());
    }
    let fut = tokio::spawn(async move {
        fut.await.unwrap();
        sim.send(&sim).await;
        println!("sent!");
        let mut result = String::new();
        sim.receive(&mut result).await;
        println!("{}", result);
        sim });
    for i in 0..m * 2 {
        sim.x(i);
        sim.x(i);
        sim.x(i);
        sim.measure(i, ());
    }
    let mut sim = fut.await.unwrap();
    let mut result = String::new();
    let mut result2 = String::new();
    sim.send_receive(&sim, &mut result).await;
    println!("{}", result);
    sim.send_receive(&sim, &mut result2).await;
    println!("{}", result2);
}
