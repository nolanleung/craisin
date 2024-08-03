use core::slice;
use nix::mount::{mount, MsFlags};
use nix::sched::{clone, unshare, CloneFlags};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::{chroot, execvp, Pid};
use std::ffi::CString;
use std::process::{self, Command};
use std::{fs, vec};

const STACK_SIZE: usize = 1024 * 1024;

fn main() {
    let mut runtime_stack: Vec<u8> = vec![0; STACK_SIZE];
    let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNET | CloneFlags::CLONE_NEWNS;

    match unsafe { boot(runtime_stack.as_mut_ptr(), flags) } {
        Ok(runtime_pid) => execute_program(runtime_pid),
        Err(err) => println!("Failed to create new process {:?}", err),
    }
}

unsafe fn boot(stack_ptr: *mut u8, flags: CloneFlags) -> Result<Pid, nix::errno::Errno> {
    let runtime_stack_slice = slice::from_raw_parts_mut(stack_ptr, STACK_SIZE);

    clone(Box::new(execute_runtime), runtime_stack_slice, flags, None)
}

fn execute_program(pid: Pid) {
    if pid == Pid::from_raw(0) {
        match waitpid(pid, None) {
            Ok(_) => println!("process terminated"),
            Err(err) => eprintln!("Failed to wait for runtime process: {:?}", err),
        }
    } else {
        unsafe {
            signal::signal(Signal::SIGCHLD, signal::SigHandler::SigIgn)
                .expect("Failed to set SIGCHLD handler");
        }

        let program = CString::new("/bin/sh").unwrap();
        let args = [
            CString::new("/bin/sh").unwrap(),
            CString::new("-c").unwrap(),
            CString::new("echo Hello from the new PID namespace").unwrap(),
        ];

        execvp(&program, &args).expect("Failed to execute program");
    }
}

fn execute_runtime() -> isize {
    network_namespace();
    pid_namespace();
    mount_namespace();

    let program = CString::new("/bin/sh").unwrap();
    let args = [CString::new("/bin/sh").unwrap()];

    execvp(&program, &args).expect("Failed to execute program");

    process::exit(1)
}

fn pid_namespace() {
    match unshare(CloneFlags::CLONE_NEWPID) {
        Ok(_) => {
            println!("We are in the new PID namespace!");
        }
        Err(err) => eprintln!("Failed to create new PID namespace: {:?}", err),
    }
}

fn mount_namespace() {
    match unshare(CloneFlags::CLONE_NEWNS) {
        Ok(_) => {
            fs::create_dir_all("/tmp/croot").expect("Failed to create /tmp/croot directory");

            mount(
                None::<&str>,
                "/",
                None::<&str>,
                MsFlags::MS_PRIVATE | MsFlags::MS_REC,
                None::<&str>,
            )
            .expect("Failed to make mounts private");

            mount::<str, str, str, str>(
                Some("proc"),
                "/proc",
                Some("proc"),
                MsFlags::MS_PRIVATE,
                None::<&str>,
            )
            .expect("Failed to mount /proc");

            chroot("/").expect("Failed to change root");

            println!("We are in the new mount namespace!");
        }
        Err(err) => eprintln!("Failed to create new mount namespace: {:?}", err),
    }
}

fn network_namespace() {
    match unshare(CloneFlags::CLONE_NEWNET) {
        Ok(_) => {
            let output = Command::new("ip")
                .arg("a")
                .output()
                .expect("Failed to execute command");

            println!(
                "Network configuration within the new network namespace:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Err(err) => eprintln!("Failed to create new network namespace: {:?}", err),
    }
}
