use std::future::Future;
use std::sync::atomic;
use lay::Operations;
use lay::gates::{CliffordGate, TGate};
use cpython::{Python, PyResult};

pub struct BlueqatSimulator {
}

pub struct BlueqatOperations {
    insts: Vec<String>,
}

impl BlueqatOperations {
    pub fn new() -> Self {
        Self { insts: vec![] }
    }

    pub fn raw_pyscript(&mut self, s: String) {
        self.insts.push(s);
    }
}

// BlueqatSimulator is a singleton.
// ... If I make circuit as local scope or make unique id as variable name,
// singleton is not necessary.
static USED: atomic::AtomicBool = atomic::AtomicBool::new(false);

impl BlueqatSimulator {
    fn import_blueqat() -> PyResult<()> {
        Python::acquire_gil().python().run(include_str!("blueqat_initialize.py"), None, None)
    }
    pub fn new() -> Result<Self, ()> {
        if USED.swap(true, atomic::Ordering::SeqCst) {
            return Err(());
        }
        // This error handling is too crude.
        Self::import_blueqat().map_err(|_| ())?;
        Ok(Self { })
    }
    pub fn send(&mut self, ops: &BlueqatOperations) -> impl Future<Output=()> {
        let script = ops.insts.join("\n");
        async move {
            Python::acquire_gil().python().run(&script, None, None).unwrap();
        }
    }
    pub fn receive<'a>(&mut self, result: &'a mut String) -> impl Future<Output=()> + 'a {
        async move {
            let s = Python::acquire_gil().python()
                                         .eval("c.run(shots=1).most_common()[0][0]", None, None)
                                         .unwrap()
                                         .to_string();
            result.push_str(&s);
        }
    }
    pub fn send_receive<'a>(&mut self, ops: &BlueqatOperations, result: &'a mut String) -> impl Future<Output=()> + 'a {
        let script = ops.insts.join("\n");
        async move {
            Python::acquire_gil().python().run(&script, None, None).unwrap();
            //eprintln!("Circuit: {}", Python::acquire_gil().python().eval("c", None, None).unwrap().to_string());
            let s = Python::acquire_gil().python()
                                         .eval("c.run(shots=1).most_common()[0][0]", None, None)
                                         .unwrap()
                                         .to_string();
            result.push_str(&s);
        }
    }
}

impl Drop for BlueqatSimulator {
    fn drop(&mut self) {
        USED.store(false, atomic::Ordering::SeqCst);
    }
}

impl Operations for BlueqatOperations {
    type Qubit = u8;
    type Slot = ();
    fn initialize(&mut self) {
        self.insts.push("c = Circuit()".to_owned());
    }
    fn measure(&mut self, q: Self::Qubit, _: ()) {
        self.insts.push(format!("c.m[{}]", q));
    }
}

impl CliffordGate for BlueqatOperations {
    fn x(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.x[{}]", q));
    }
    fn y(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.y[{}]", q));
    }
    fn z(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.z[{}]", q));
    }
    fn h(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.h[{}]", q));
    }
    fn s(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.s[{}]", q));
    }
    fn sdg(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.sdg[{}]", q));
    }
    fn cx(&mut self, c: Self::Qubit, t: Self::Qubit) {
        self.insts.push(format!("c.cx[{}, {}]", c, t));
    }
}

impl TGate for BlueqatOperations {
    fn t(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.t[{}]", q));
    }
    fn tdg(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.tdg[{}]", q));
    }
}

#[cfg(test)]
mod tests {
    use crate::BlueqatSimulator;
    use crate::BlueqatOperations;
    use lay::Operations;
    use lay::gates::CliffordGate;
    use tokio::runtime::Runtime;
    use tokio::prelude::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn python_raw() {
        let mut rt = Runtime::new().unwrap();
        let mut sim = BlueqatSimulator::new().unwrap();
        let mut ops = BlueqatOperations::new();
        let mut s = String::new();

        ops.initialize();
        ops.raw_pyscript("import numpy as np".to_owned());
        ops.raw_pyscript("print(np.eye(2))".to_owned());
        ops.raw_pyscript("if True: c.x[0]".to_owned());
        ops.raw_pyscript("if False: c.x[1]".to_owned());
        ops.measure(0, ());
        ops.measure(1, ());
        rt.block_on(sim.send_receive(&ops, &mut s));
        assert_eq!(&s, "10");
    }
}
