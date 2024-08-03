use std::vec;

const STACK_SIZE: usize = 1024 * 1024;

fn main() {
    let mut child_stack = vec![0; STACK_SIZE];
}

fn child_function() -> isize {
    let pid = nix::unistd::getpid();
    if pid == nix::unistd::Pid::from_raw(1) {
        println!("In PID 1");
    } else {
        println!("Not in PID 1, in PID {}", pid);
    }

    0
}
