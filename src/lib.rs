use std::future::Future;
use std::sync::atomic;
use lay::Operations;
use lay::gates::{CliffordGate, TGate};
use cpython::{Python, PyResult};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

enum Op {
    Initialize,
    Unary(&'static str, u8),
    Binary(&'static str, u8, u8),
}

pub struct BlueqatSimulator {
}

pub struct BlueqatOperations {
    ops: Vec<Op>,
}

impl BlueqatOperations {
    pub fn new() -> Self {
        Self { ops: vec![] }
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
    // send method should return Result type. (but, async...?)
    pub fn send(self, ops: BlueqatOperations) -> impl Future<Output=Self> {
        async {
            let mut script = vec![];
            for op in ops.ops {
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
            self
        }
    }
    pub fn receive(self) -> impl Future<Output=(Self, String)> {
        async {
            let s = Python::acquire_gil().python()
                                         .eval("c.run(shots=1).most_common()[0][0]", None, None)
                                         .unwrap()
                                         .to_string();
            (self, s)
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
        self.ops.push(Op::Initialize);
    }
    fn measure(&mut self, q: Self::Qubit, _: ()) {
        self.ops.push(Op::Unary("m", q));
    }
}

impl CliffordGate for BlueqatOperations {
    fn x(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("x", q));
    }
    fn y(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("y", q));
    }
    fn z(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("z", q));
    }
    fn h(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("h", q));
    }
    fn s(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("s", q));
    }
    fn sdg(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("sdg", q));
    }
    fn cx(&mut self, c: Self::Qubit, t: Self::Qubit) {
        self.ops.push(Op::Binary("cx", c, t));
    }
}

impl TGate for BlueqatOperations {
    fn t(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("t", q));
    }
    fn tdg(&mut self, q: Self::Qubit) {
        self.ops.push(Op::Unary("tdg", q));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
