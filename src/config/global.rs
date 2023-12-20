use std::sync::Arc;

use crate::CmdArgs;
use crate::container::home::HomeDir;

/// 全局App共享的config
pub struct GlobalAppConfig {
    pub cmd_args: CmdArgs,
    pub home_dir: Arc<HomeDir>,
}
