use std::sync::Arc;

pub struct Bar {
    bar_core: Arc<BarCore>,
}

impl Bar {
    pub fn add_size(&mut self, size: usize) {
        self.bar_core.curr_file_size = self.bar_core.curr_file_size + size;
    }

    pub fn finish(&self, success: bool) {}
}

struct BarCore {
    curr_file_size: usize,
    finish: bool,
}


pub struct MultiBar {
    bar_vec: Vec<(String, usize, Arc<BarCore>)>,
}

impl MultiBar {
    pub fn new_multi_bar() -> MultiBar {
        MultiBar {
            bar_vec: Vec::with_capacity(8)
        }
    }

    pub fn add_new_bar(&mut self, short_digest: String, file_count: usize) -> Bar {
        let bar_core_arc = Arc::new(BarCore {
            curr_file_size: 0,
            finish: false,
        });
        let bar_data = (short_digest, file_count, bar_core_arc.clone());
        self.bar_vec.push(bar_data);
        Bar {
            bar_core: bar_core_arc
        }
    }
}