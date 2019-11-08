#![warn(missing_docs)]

use futures::sync::oneshot;
use futures::{future, Future};
use substrate_cli::VersionInfo;

use std::cell::RefCell;

// handles ctrl-c
struct Exit;

impl substrate_cli::IntoExit for Exit {
    type Exit = future::MapErr<oneshot::Receiver<()>, fn(oneshot::Canceled) -> ()>;
    fn into_exit(self) -> Self::Exit {
        // can't use signal directly here because CtrlC takes only `Fn`.
        let (exit_send, exit) = oneshot::channel();

        let exit_send_cell = RefCell::new(Some(exit_send));
        ctrlc::set_handler(move || {
            if let Some(exit_send) = exit_send_cell
                .try_borrow_mut()
                .expect("signal handler not reentrant; qed")
                .take()
            {
                exit_send.send(()).expect("Error sending exit notification");
            }
        })
        .expect("Error setting Ctrl-C handler");

        exit.map_err(drop)
    }
}

fn main() -> Result<(), substrate_cli::error::Error> {
    let version = VersionInfo {
        name: "Plasm Node",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "plasm-node",
        author: "Takumi Yamashita <takumi@staked.co.jp>",
        description: "PlasmChain Node",
        support_url: "https://github.com/stakedtechnologies/Plasm/issues/new",
    };

    plasm_cli::run(std::env::args(), Exit, version)
}
