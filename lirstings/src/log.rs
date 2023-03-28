use std::fmt::Display;

pub trait Log {
    fn info(&mut self, msg: impl Display);
    fn warn(&mut self, msg: impl Display);
    fn error(&mut self, msg: impl Display);
}

pub struct DummyLogger;
impl Log for DummyLogger {
    fn info(&mut self, _msg: impl Display) {}
    fn warn(&mut self, _msg: impl Display) {}
    fn error(&mut self, _msg: impl Display) {}
}

pub struct DefaultLogger;
impl Log for DefaultLogger {
    fn info(&mut self, msg: impl Display) {
        eprintln!("\x1b[1;36mlirstings: \x1b[0;32m{msg}\x1b[0m");
    }

    fn warn(&mut self, msg: impl Display) {
        eprintln!("\x1b[1;36mlirstings: \x1b[33mwarning:\x1b[22m {msg}\x1b[0m");
    }

    fn error(&mut self, msg: impl Display) {
        eprintln!("\x1b[1;36mlirstings: \x1b[31merror:\x1b[22m {msg}\x1b[0m");
    }
}
