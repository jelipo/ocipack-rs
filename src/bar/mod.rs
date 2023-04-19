use std::cell::RefCell;
use std::rc::Rc;

use ubyte::ToByteUnit;

pub struct Bar {
    bar_core: Rc<RefCell<BarCore>>,
}

impl Bar {
    pub fn set_size(&mut self, curr_size: u64, full_size: u64) {
        let mut bar_core_mut = self.bar_core.borrow_mut();
        bar_core_mut.curr_size = curr_size;
        bar_core_mut.full_size = full_size;
    }

    pub fn finish(&mut self, success: bool, finished_info: &str) {
        let mut bar_core_mut = self.bar_core.borrow_mut();
        bar_core_mut.finished = true;
        bar_core_mut.success = success;
        bar_core_mut.finished_info = finished_info.to_string()
    }
}

struct BarCore {
    curr_size: u64,
    full_size: u64,
    finished: bool,
    success: bool,
    finished_info: String,
}

pub struct MultiBar {
    /// name,total,core
    bar_vec: Vec<(String, Rc<RefCell<BarCore>>)>,
    first: bool,
}

impl MultiBar {
    pub fn new_multi_bar() -> MultiBar {
        MultiBar {
            bar_vec: Vec::with_capacity(8),
            first: true,
        }
    }

    pub fn add_new_bar(&mut self, short_digest: String, file_count: u64) -> Bar {
        let bar_core = Rc::new(RefCell::new(BarCore {
            curr_size: 0,
            full_size: file_count,
            finished: false,
            success: false,
            finished_info: String::new(),
        }));
        let bar_data = (short_digest, bar_core.clone());
        self.bar_vec.push(bar_data);
        Bar { bar_core }
    }

    pub fn update(&mut self) {
        if self.first {
            self.first = false;
        } else {
            print!("\x1b[{}A", self.bar_vec.len());
        }
        for (name, bar_core) in &self.bar_vec {
            let bar_core = bar_core.borrow();
            let curr_size = bar_core.curr_size.bytes().to_string();
            let full_size = bar_core.full_size.bytes().to_string();
            let done_str = if bar_core.finished & bar_core.success {
                "√"
            } else if bar_core.finished & !bar_core.success {
                "❌"
            } else {
                ""
            };
            println!("{}{:>12} / {:<12} {:4}{}", name, curr_size, full_size, done_str, bar_core.finished_info);
        }
    }
}

#[test]
fn it_works() {
    println!("{}{:>12} / {:<12} {}", "qwertyui", "123.12Mib", "123.12Mib", "√");
}
