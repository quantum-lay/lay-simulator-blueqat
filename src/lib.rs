use std::sync::atomic;
use lay::{Layer, Measured, OpsVec, operations::{opid, OpArgs}};
use lay::gates::{PauliGate, CXGate, HGate, SGate, TGate};
use cpython::{Python, PyResult};

pub fn raw_pyscript(s: String) -> OpArgs<BlueqatSimulator> {
    OpArgs::Var(opid::USERDEF, Box::new(s))
}

pub trait RawScriptGate {
    fn raw_pyscript(&mut self, s: String);
}

impl RawScriptGate for OpsVec<BlueqatSimulator> {
    fn raw_pyscript(&mut self, s: String) {
        self.as_mut_vec().push(raw_pyscript(s));
    }
}

const UNASSIGNED: u32 = 0xffffffff;

#[derive(Debug)]
pub struct BlueqatSimulator {
    slot: [u32; 64],
}

#[derive(Debug)]
pub struct BlueqatMeasured([bool; 64]);

impl BlueqatMeasured {
    pub fn new() -> BlueqatMeasured {
        Self([false; 64])
    }
}

impl Measured for BlueqatMeasured {
    type Slot = u32;
    fn get(&self, n: u32) -> bool {
        (self.0)[n as usize]
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
        Ok(Self { slot: [UNASSIGNED; 64] })
    }

    #[inline]
    fn op_to_script(op: &OpArgs<BlueqatSimulator>) -> String {
        match op {
            OpArgs::Empty(id) if *id == opid::INIT =>
                "c = Circuit()".to_owned(),
            OpArgs::QS(id, q, _) if *id == opid::MEAS => {
                //assert_eq!(q, s, "Qubit and slot must be same in this simulator.");
                format!("c.m[{}]", q)
            }
            OpArgs::QQ(id, c, t) if *id == opid::CX =>
                format!("c.cx[{}, {}]", c, t),
            OpArgs::Q(id, q) if *id == opid::X =>
                format!("c.x[{}]", q),
            OpArgs::Q(id, q) if *id == opid::Y =>
                format!("c.y[{}]", q),
            OpArgs::Q(id, q) if *id == opid::Z =>
                format!("c.z[{}]", q),
            OpArgs::Q(id, q) if *id == opid::H =>
                format!("c.h[{}]", q),
            OpArgs::Q(id, q) if *id == opid::S =>
                format!("c.s[{}]", q),
            OpArgs::Q(id, q) if *id == opid::SDG =>
                format!("c.sdg[{}]", q),
            OpArgs::Q(id, q) if *id == opid::T =>
                format!("c.t[{}]", q),
            OpArgs::Q(id, q) if *id == opid::TDG =>
                format!("c.tdg[{}]", q),
            OpArgs::Var(id, cmd) if *id == opid::USERDEF => {
                cmd.downcast_ref::<String>().unwrap().clone()
            }
            _ => unimplemented!("Unknown op {:?}", op)
        }
    }

    fn ops_to_script(ops: &[OpArgs<BlueqatSimulator>]) -> String {
        ops.iter().map(Self::op_to_script).collect::<Vec<_>>().join("\n")
    }

    fn assign_slot(&mut self, ops: &[OpArgs<BlueqatSimulator>]) {
        for op in ops {
            if let OpArgs::QS(id, q, s) = op {
                if *id != opid::MEAS { continue; }
                if self.slot[*q as usize] == UNASSIGNED {
                    self.slot[*q as usize] = *s as u32;
                } else {
                    panic!("This simulator cannot measure same qubit without receive former result.");
                }
            }
        }
    }

    fn write_buf_reset_slot(&mut self, measured: &str, buf: &mut BlueqatMeasured) {
        let measured = measured.as_bytes();
        for (q, s) in self.slot.iter_mut().enumerate() {
            if *s != UNASSIGNED {
                (buf.0)[*s as usize] = measured[q] == b'1';
                *s = UNASSIGNED;
            }
        }
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
    type Operation = OpArgs<Self>;
    type Qubit = u32;
    type Slot = u32;
    type Buffer = BlueqatMeasured;
    type Requested = PyResult<()>;
    type Response = PyResult<()>;

    fn send(&mut self, ops: &[OpArgs<Self>]) -> Self::Requested {
        let script = Self::ops_to_script(ops);
        self.assign_slot(ops);
        eprintln!("{}", script);
        eprintln!("# --- send ---");
        Python::acquire_gil().python().run(&script, None, None)?;
        Ok(())
    }

    fn receive(&mut self, buf: &mut Self::Buffer) -> Self::Response {
        let s = Python::acquire_gil().python()
                                         .eval("c.run(shots=1).most_common()[0][0]", None, None)?
                                         .to_string();
        eprintln!("# --- receive ---");
        self.write_buf_reset_slot(&s, buf);
        eprintln!("# raw: {}", s);
        eprint!("# map: ");
        for b in 0..s.len() {
            eprint!("{}", buf.get(b as u32) as u8);
        }
        eprintln!();
        Ok(())
    }

    fn send_receive(&mut self, ops: &[OpArgs<Self>], buf: &mut Self::Buffer) -> Self::Response {
        let script = Self::ops_to_script(ops);
        self.assign_slot(ops);
        Python::acquire_gil().python().run(&script, None, None)?;
        //eprintln!("Circuit: {}", Python::acquire_gil().python().eval("c", None, None).unwrap().to_string());
        let s = Python::acquire_gil().python()
                                         .eval("c.run(shots=1).most_common()[0][0]", None, None)?
                                         .to_string();
        eprintln!("{}", script);
        eprintln!("# --- send_receive ---");
        self.write_buf_reset_slot(&s, buf);
        eprintln!("# raw: {}", s);
        eprint!("# map: ");
        for b in 0..s.len() {
            eprint!("{}", buf.get(b as u32) as u8);
        }
        eprintln!();
        Ok(())
    }

    fn make_buffer(&self) -> Self::Buffer {
        Self::Buffer::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{BlueqatSimulator, BlueqatMeasured, RawScriptGate};
    use lay::{Layer, Measured, OpsVec};

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
        let mut measured = BlueqatMeasured::new();
        sim.send_receive(ops.as_ref(), &mut measured).unwrap();
        assert_eq!(measured.get(0), true);
        assert_eq!(measured.get(1), false);
    }
}
