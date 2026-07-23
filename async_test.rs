use std::task::{Context, Poll::*, Waker};

use tokio::task::{self, yield_now};

fn run_through_queue(queue: &[&str]) {
    for name in queue {
        println!("- {name}");
    }
}

async fn process(name: &str) -> i32 {
    loop {
        println!("{name}: Part 1");
        yield_now().await;
        println!("{name}: Part 2");
        yield_now().await;
        println!("{name}: Part 3");
        yield_now().await;
    }
    42
}

async fn create_process(name: &str) {
    let mut proc = Box::pin(process(name));

    loop {
        println!("Running event queue for process {name}:");
        run_through_queue(&["foo", "bar", "baz"]);

        let mut context = Context::from_waker(Waker::noop());
        match Future::poll(proc.as_mut(), &mut context) {
            Ready(code) => {
                println!("Process exited with code {code}.");
                break;
            }
            Pending => {}
        }
    }
}

async fn yield_times(times: usize) {
    for _ in 0..times {
        println!("Yielding");
        yield_now().await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    task::spawn(create_process("First process"));
    task::spawn(create_process("Second process"));
    task::spawn(create_process("Third process"));
    task::spawn(yield_times(5));
    task::spawn(yield_times(2));
    task::spawn(yield_times(4));

    loop {
        yield_now().await;
    }
}
