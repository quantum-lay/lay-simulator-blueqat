use lay::Operations;
use lay::gates::{CliffordGate};
use lay_simulator_blueqat::{BlueqatSimulator, BlueqatOperations};
use tokio::runtime::Runtime;

type Qubit = u8;

const PHISQUBIT_PER_LOGQUBIT: Qubit = 7;
const MEASURE_ANCILLA_QUBITS: Qubit = 6;
const MEASURE_MASK: u32 = 127;

pub struct SteaneLayer {
    // TODO: not pub.
    pub ops: BlueqatOperations,
    sim: BlueqatSimulator,
    rt: Runtime,
    n_physical_qubits: Qubit,
    n_logical_qubits: Qubit,
}

const ERR_TABLE_X: [u32;8] = [999 /* dummy */, 0, 1, 6, 2, 4, 3, 5];
const ERR_TABLE_Z: [u32;8] = [999 /* dummy */, 3, 4, 6, 5, 0, 1, 2];

impl SteaneLayer {
    pub fn new(n_qubits: Qubit) -> Self {
        Self {
            ops: BlueqatOperations::new(),
            sim: BlueqatSimulator::new().unwrap(),
            rt: Runtime::new().unwrap(),
            n_physical_qubits: PHISQUBIT_PER_LOGQUBIT * n_qubits + MEASURE_ANCILLA_QUBITS,
            n_logical_qubits: n_qubits }
    }

    fn measure_ancilla(&self) -> Qubit {
        self.n_physical_qubits - 6
    }

    fn syndrome_measure_and_recover(&mut self) {
        eprintln!("START syndrome_measure_and_recover");
        let m0 = self.measure_ancilla();
        for i in 0..self.n_logical_qubits {
            for j in 0..PHISQUBIT_PER_LOGQUBIT {
                self.ops.h(i * PHISQUBIT_PER_LOGQUBIT + j);
            }
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT, m0);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 1, m0 + 1);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 2, m0 + 2);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 3, m0 + 1);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 3, m0 + 2);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 4, m0);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 4, m0 + 2);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 5, m0);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 5, m0 + 1);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 5, m0 + 2);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 6, m0);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 6, m0 + 1);

            for j in 0..PHISQUBIT_PER_LOGQUBIT {
                self.ops.h(i * PHISQUBIT_PER_LOGQUBIT + j);
            }
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT, m0 + 3);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT, m0 + 5);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 1, m0 + 4);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 1, m0 + 5);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 2, m0 + 3);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 2, m0 + 4);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 2, m0 + 5);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 3, m0 + 3);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 4, m0 + 4);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 5, m0 + 5);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 6, m0 + 3);
            self.ops.cx(i * PHISQUBIT_PER_LOGQUBIT + 6, m0 + 4);
            for j in 0..MEASURE_ANCILLA_QUBITS {
                //self.ops.measure(i * PHISQUBIT_PER_LOGQUBIT + j, ());
                self.ops.measure(m0 + j, ());
            }
            let mut s = String::new();
            self.rt.block_on(self.sim.send_receive(&self.ops, &mut s));
            //println!("measured s: {}", s);
            let s = String::from_utf8(s.as_bytes()[m0 as usize..].iter().rev().map(|a| *a).collect()).unwrap();
            let measured = Qubit::from_str_radix(&s, 2).unwrap();
            self.ops = BlueqatOperations::new();
            eprintln!("logical qubit: {}, measured: {:b}", i, measured);
            for j in 0..MEASURE_ANCILLA_QUBITS {
                // reset
                if measured & (1 << j) != 0 {
                    self.ops.x(m0 + j);
                }
            }
            if measured & 7 > 0 {
                let err_x = ERR_TABLE_X[(measured & 7) as usize] + (i as u32) * (PHISQUBIT_PER_LOGQUBIT as u32);
                eprintln!("* X Err on {}", err_x);
                self.ops.x(err_x as Qubit);
            }
            if (measured >> 3) > 0 {
                let err_z = ERR_TABLE_Z[(measured >> 3) as usize] + (i as u32) * (PHISQUBIT_PER_LOGQUBIT as u32);
                eprintln!("* Z Err on {}", err_z);
                self.ops.z(err_z as Qubit);
            }
            self.rt.block_on(self.sim.send(&self.ops));
            self.ops = BlueqatOperations::new();
        }
        eprintln!("END   syndrome_measure_and_recover");
    }
}

impl Operations for SteaneLayer {
    type Qubit = Qubit;
    type Slot = Qubit;
    fn initialize(&mut self) {
        self.ops.initialize();
        //self.syndrome_measure_and_recover();
    }
    fn measure(&mut self, q: <Self as lay::Operations>::Qubit, c: <Self as lay::Operations>::Slot) {

    }
}

impl CliffordGate for SteaneLayer {
   fn x(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.x(i);
       }
   }
   fn y(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.y(i);
       }
   }
   fn z(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.z(i);
       }
   }
   fn h(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.h(i);
       }
   }
   fn s(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.s(i);
       }
   }
   fn sdg(&mut self, q: <Self as lay::Operations>::Qubit) {
       for i in (q * PHISQUBIT_PER_LOGQUBIT)..(q * PHISQUBIT_PER_LOGQUBIT + PHISQUBIT_PER_LOGQUBIT) {
           self.ops.sdg(i);
       }
   }
   fn cx(&mut self, c: <Self as lay::Operations>::Qubit, t: <Self as lay::Operations>::Qubit) {
       for i in 0..PHISQUBIT_PER_LOGQUBIT {
           self.ops.cx(c * PHISQUBIT_PER_LOGQUBIT + i, t * PHISQUBIT_PER_LOGQUBIT + i);
       }
   }
}

fn main() {
    let mut steane = SteaneLayer::new(2);
    steane.initialize();
    eprintln!("First syndrome measurement 5 times");
    steane.syndrome_measure_and_recover();
    steane.syndrome_measure_and_recover();
    steane.syndrome_measure_and_recover();
    steane.syndrome_measure_and_recover();
    steane.syndrome_measure_and_recover();
    eprintln!("END First syndrome measurement 5 times\n\n\n");
    eprintln!("Expected: not shown");
    steane.syndrome_measure_and_recover();
    steane.x(0);
    eprintln!("Expected: not shown");
    steane.syndrome_measure_and_recover();
    steane.ops.x(12);
    eprintln!("Expected: 12");
    steane.syndrome_measure_and_recover();
    eprintln!("Expected: not shown");
    steane.syndrome_measure_and_recover();
    steane.ops.z(8);
    eprintln!("Expected: 8");
    steane.syndrome_measure_and_recover();
    eprintln!("Expected: not shown");
    steane.syndrome_measure_and_recover();
}
