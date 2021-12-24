use std::rc::Rc;
use std::sync::Arc;

use crate::CmdArgs;
use crate::reg::home::HomeDir;

pub struct GlobalAppConfig {
    pub cmd_args: CmdArgs,
    pub home_dir: Arc<HomeDir>,
}