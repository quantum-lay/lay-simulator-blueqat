use std::sync::atomic;
use lay::Layer;
use lay::gates::{CliffordGate, TGate};
use cpython::{Python, PyResult};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

pub struct BlueqatSimulator {
    sendbuf: Vec<Op>,
    rt: Runtime,
    fut: Option<JoinHandle<()>>,
}

enum Op {
    Initialize,
    Unary(&'static str, u8),
    Binary(&'static str, u8, u8),
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
        let rt = Runtime::new().unwrap();
        let fut = Some(rt.spawn(async {}));
        // This error handling is too crude.
        Self::import_blueqat().map_err(|_| ())?;
        Ok(Self { sendbuf: vec![], rt, fut })
    }
}

impl Drop for BlueqatSimulator {
    fn drop(&mut self) {
        USED.store(false, atomic::Ordering::SeqCst);
    }
}

impl Layer for BlueqatSimulator {
    type Qubit = u8;
    type Slot = ();
    type Receive = String;
    fn initialize(&mut self) {
        self.sendbuf.push(Op::Initialize);
    }
    // send method should return Result type. (but, async...?)
    fn send(&mut self) {
        let mut ops = vec![];
        std::mem::swap(&mut ops, &mut self.sendbuf);
        let fut = self.fut.take().unwrap();
        self.fut = Some(self.rt.spawn(async {
            // Unwrapping!
            fut.await.unwrap();
            let mut script = vec![];
            for op in ops {
                match op {
                    Op::Initialize => {
                        script.push("c = Circuit()".to_owned());
                    },
                    Op::Unary(g, q) => {
                        script.push(format!("c.{}[{}]", g, q));
                    },
                    Op::Binary(g, c, t) => {
                        script.push(format!("c.{}[{}, {}]", g, c, t));
                    },
                }
            }
            Python::acquire_gil().python().run(&script.join("\n"), None, None).unwrap();
        }));
    }
    fn measure(&mut self, q: Self::Qubit, _: ()) {
        self.sendbuf.push(Op::Unary("m", q));
    }
    fn receive(&mut self) -> String {
        let fut = self.fut.take().unwrap();
        self.fut = Some(self.rt.spawn(async {}));
        // Unwrapping!
        self.rt.block_on(fut).unwrap();
        Python::acquire_gil().python().eval("c.run(shots=1).most_common()[0][0]", None, None).unwrap().to_string()
    }
}

impl CliffordGate for BlueqatSimulator {
    fn x(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("x", q));
    }
    fn y(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("y", q));
    }
    fn z(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("z", q));
    }
    fn h(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("h", q));
    }
    fn s(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("s", q));
    }
    fn sdg(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("sdg", q));
    }
    fn cx(&mut self, c: Self::Qubit, t: Self::Qubit) {
        self.sendbuf.push(Op::Binary("cx", c, t));
    }
}

impl TGate for BlueqatSimulator {
    fn t(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("t", q));
    }
    fn tdg(&mut self, q: Self::Qubit) {
        self.sendbuf.push(Op::Unary("tdg", q));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
