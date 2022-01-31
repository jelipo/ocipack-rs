use std::cell::RefCell;
use std::rc::Rc;

pub struct Bar {
    bar_core: Rc<RefCell<BarCore>>,
}

impl Bar {
    pub fn add_size(&mut self, size: u64) {
        let mut bar_core_mut = self.bar_core.borrow_mut();
        bar_core_mut.curr_file_size += size;
    }

    pub fn finish(&mut self, _success: bool, _finished_info: &str) {
        let mut bar_core_mut = self.bar_core.borrow_mut();
        bar_core_mut.finished = true;
    }
}

struct BarCore {
    curr_file_size: u64,
    finished: bool,
}

pub struct MultiBar {
    /// name,total,core
    bar_vec: Vec<(String, u64, Rc<RefCell<BarCore>>)>,
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
            curr_file_size: 0,
            finished: false,
        }));
        let bar_data = (short_digest, file_count, bar_core.clone());
        self.bar_vec.push(bar_data);
        Bar {
            bar_core
        }
    }

    pub fn update(&mut self) {
        if self.first {
            self.first = false;
        } else {
            print!("\x1b[{}A", self.bar_vec.len());
        }
        for (name, _ds, bar_core) in &self.bar_vec {
            let bar_core = bar_core.borrow();
            println!("{} {}", name, bar_core.curr_file_size);
        }
    }
}