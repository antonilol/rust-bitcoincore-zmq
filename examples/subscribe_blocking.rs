use bitcoincore_zmq::subscribe_single_blocking;
use core::ops::ControlFlow;

fn main() {
    let callback = |msg| {
        match msg {
            Ok(msg) => println!("Received message: {msg}"),
            Err(err) => {
                // Do this to exit and return the error
                return ControlFlow::Break(err);
            }
        }

        ControlFlow::Continue(())
    };

    match subscribe_single_blocking("tcp://127.0.0.1:28359", callback) {
        Ok(ControlFlow::Break(err)) => {
            // Callback exited by returning ControlFlow::Break
            println!("Error receiving message: {err}");
        }
        Err(err) => {
            println!("Unable to connect: {err}");
        }
        Ok(ControlFlow::Continue(v)) => {
            // unreachable
            match v {}
        }
    }
}
