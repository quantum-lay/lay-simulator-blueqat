use std::sync::atomic;
use lay::{Layer, Operation, Measured, OpsVec, operations::opid};
use lay::gates::{PauliGate, CXGate, HGate, SGate, TGate};
use cpython::{Python, PyResult};

pub fn raw_pyscript(s: String) -> Operation<BlueqatSimulator> {
    Operation::Var(opid::USERDEF, Box::new(s))
}

pub trait RawScriptGate {
    fn raw_pyscript(&mut self, s: String);
}

impl RawScriptGate for OpsVec<BlueqatSimulator> {
    fn raw_pyscript(&mut self, s: String) {
        self.as_mut_vec().push(raw_pyscript(s));
    }
}

#[derive(Debug)]
pub struct BlueqatSimulator {}

#[derive(Debug)]
pub struct BlueqatMeasured(pub String);

impl Measured for BlueqatMeasured {
    type Slot = u32;
    fn get(&self, n: u32) -> bool {
        self.0.as_bytes()[n as usize] == b'1'
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
        Ok(Self {})
    }

    #[inline]
    fn op_to_script(op: &Operation<BlueqatSimulator>) -> String {
        match op {
            Operation::Empty(id) if *id == opid::INIT =>
                "c = Circuit()".to_owned(),
            Operation::QS(id, q, s) if *id == opid::MEAS => {
                assert_eq!(q, s, "Qubit and slot must be same in this simulator.");
                format!("c.m[{}]", q)
            }
            Operation::QQ(id, c, t) if *id == opid::CX =>
                format!("c.cx[{}, {}]", c, t),
            Operation::Q(id, q) if *id == opid::X =>
                format!("c.x[{}]", q),
            Operation::Q(id, q) if *id == opid::Y =>
                format!("c.y[{}]", q),
            Operation::Q(id, q) if *id == opid::Z =>
                format!("c.z[{}]", q),
            Operation::Q(id, q) if *id == opid::H =>
                format!("c.h[{}]", q),
            Operation::Q(id, q) if *id == opid::S =>
                format!("c.s[{}]", q),
            Operation::Q(id, q) if *id == opid::SDG =>
                format!("c.sdg[{}]", q),
            Operation::Q(id, q) if *id == opid::T =>
                format!("c.t[{}]", q),
            Operation::Q(id, q) if *id == opid::TDG =>
                format!("c.tdg[{}]", q),
            Operation::Var(id, cmd) if *id == opid::USERDEF => {
                cmd.downcast_ref::<String>().unwrap().clone()
            }
            _ => unimplemented!("Unknown op {:?}", op)
        }
    }

    fn ops_to_script(ops: &[Operation<BlueqatSimulator>]) -> String {
        ops.iter().map(Self::op_to_script).collect::<Vec<_>>().join("\n")
    }
}

impl Drop for BlueqatSimulator {
    fn drop(&mut self) {
        USED.store(false, atomic::Ordering::SeqCst);
    }
}

impl PauliGate for BlueqatSimulator {}
impl HGate for BlueqatSimulator {}
impl SGate for BlueqatSimulator {}
impl TGate for BlueqatSimulator {}
impl CXGate for BlueqatSimulator {}

impl Layer for BlueqatSimulator {
    type Qubit = u32;
    type Slot = u32;
    type Buffer = ();
    type Requested = PyResult<()>;
    type Response = PyResult<BlueqatMeasured>;

    fn send(&mut self, ops: &[Operation<Self>]) -> Self::Requested {
        let script = Self::ops_to_script(ops);
        Python::acquire_gil().python().run(&script, None, None)?;
        Ok(())
    }

    fn receive(&mut self, _: &mut Self::Buffer) -> Self::Response {
        let s = Python::acquire_gil().python()
                                     .eval("c.run(shots=1).most_common()[0][0]", None, None)?
                                     .to_string();
        Ok(BlueqatMeasured(s))
    }

    fn send_receive(&mut self, ops: &[Operation<Self>], _: &mut Self::Buffer) -> Self::Response {
        let script = Self::ops_to_script(ops);
        Python::acquire_gil().python().run(&script, None, None)?;
        //eprintln!("Circuit: {}", Python::acquire_gil().python().eval("c", None, None).unwrap().to_string());
        let s = Python::acquire_gil().python()
                                     .eval("c.run(shots=1).most_common()[0][0]", None, None)?
                                     .to_string();
        Ok(BlueqatMeasured(s))
    }
}

#[cfg(test)]
mod tests {
    use crate::{BlueqatSimulator, RawScriptGate};
    use lay::{Layer, OpsVec};

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn python_raw() {
        let mut sim = BlueqatSimulator::new().unwrap();
        let mut ops = OpsVec::new();

        ops.initialize();
        ops.raw_pyscript("import numpy as np".to_owned());
        ops.raw_pyscript("print(np.eye(2))".to_owned());
        ops.raw_pyscript("if True: c.x[0]".to_owned());
        ops.raw_pyscript("if False: c.x[1]".to_owned());
        ops.measure(0, 0);
        ops.measure(1, 1);
        let s = sim.send_receive(ops.as_ref(), &mut ()).unwrap();
        assert_eq!(s.0, "10");
    }
}
