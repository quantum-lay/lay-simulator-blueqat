use std::sync::atomic;
use lay::Layer;
use lay::gates::{PauliGate, CXGate, HGate, SGate, TGate};
use cpython::{Python, PyResult};

pub struct BlueqatSimulator {
    insts: Vec<String>,
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
        Ok(Self { insts: vec![] })
    }

    pub fn raw_pyscript(&mut self, s: String) {
        self.insts.push(s);
    }
}

impl Drop for BlueqatSimulator {
    fn drop(&mut self) {
        USED.store(false, atomic::Ordering::SeqCst);
    }
}

impl Layer for BlueqatSimulator {
    type Qubit = u32;
    type Slot = ();
    type Buffer = String;
    type Requested = ();
    type Response = ();
    type ReqRes = ();

    fn initialize(&mut self) {
        self.insts.push("c = Circuit()".to_owned());
    }

    fn measure(&mut self, q: Self::Qubit, _: ()) {
        self.insts.push(format!("c.m[{}]", q));
    }

    fn send(&mut self) -> Self::Requested {
        let script = self.insts.join("\n");
        Python::acquire_gil().python().run(&script, None, None).unwrap();
    }

    fn receive(&mut self, result: &mut Self::Buffer) -> Self::Response {
        let s = Python::acquire_gil().python()
                                     .eval("c.run(shots=1).most_common()[0][0]", None, None)
                                     .unwrap()
                                     .to_string();
        result.push_str(&s);
    }

    fn send_receive(&mut self, result: &mut Self::Buffer) -> Self::ReqRes {
        let script = self.insts.join("\n");
        Python::acquire_gil().python().run(&script, None, None).unwrap();
        //eprintln!("Circuit: {}", Python::acquire_gil().python().eval("c", None, None).unwrap().to_string());
        let s = Python::acquire_gil().python()
                                     .eval("c.run(shots=1).most_common()[0][0]", None, None)
                                     .unwrap()
                                     .to_string();
        result.push_str(&s);
    }
}

impl PauliGate for BlueqatSimulator {
    fn x(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.x[{}]", q));
    }

    fn y(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.y[{}]", q));
    }

    fn z(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.z[{}]", q));
    }
}

impl HGate for BlueqatSimulator {
    fn h(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.h[{}]", q));
    }
}

impl SGate for BlueqatSimulator {
    fn s(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.s[{}]", q));
    }

    fn sdg(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.sdg[{}]", q));
    }
}

impl TGate for BlueqatSimulator {
    fn t(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.t[{}]", q));
    }

    fn tdg(&mut self, q: Self::Qubit) {
        self.insts.push(format!("c.tdg[{}]", q));
    }
}

impl CXGate for BlueqatSimulator {
    fn cx(&mut self, c: Self::Qubit, t: Self::Qubit) {
        self.insts.push(format!("c.cx[{}, {}]", c, t));
    }
}

#[cfg(test)]
mod tests {
    use crate::BlueqatSimulator;
    use lay::Layer;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn python_raw() {
        let mut sim = BlueqatSimulator::new().unwrap();
        let mut s = String::new();

        sim.initialize();
        sim.raw_pyscript("import numpy as np".to_owned());
        sim.raw_pyscript("print(np.eye(2))".to_owned());
        sim.raw_pyscript("if True: c.x[0]".to_owned());
        sim.raw_pyscript("if False: c.x[1]".to_owned());
        sim.measure(0, ());
        sim.measure(1, ());
        sim.send_receive(&mut s);
        assert_eq!(&s, "10");
    }
}
